import { useMemo, useRef, useState } from "react";
import { useSchema } from "../hooks/useSchema";
import { useEntities } from "../hooks/useEntities";
import { useQuery } from "@tanstack/react-query";
import { Link as RouterLink } from "react-router-dom";
import GraphCanvas, { labelColor } from "../components/GraphCanvas";
import RelationshipPicker from "../components/RelationshipPicker";
import EntityForm from "../components/EntityForm";
import { Button } from "../components/ui/button";
import { Card, CardHeader, CardTitle, CardContent } from "../components/ui/card";
import { Badge } from "../components/ui/badge";
import { useStore } from "../state/store";
import { Maximize2, X, ExternalLink, Plus, Trash2 } from "lucide-react";

const BASE = (import.meta.env.VITE_KG_SERVER_BASE as string | undefined) ?? "http://localhost:9000";

interface EdgeRow {
  id: number;
  type: string;
  source: number;
  target: number;
  props: Record<string, unknown>;
}

export default function GraphPage() {
  const { data: schema } = useSchema();
  const [labels, setLabels] = useState<Set<string>>(new Set());
  const [layout, setLayout] = useState("cose-bilkent");
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [linkState, setLinkState] = useState<{
    fromId: number;
    fromLabel: string;
    toId?: number;
  } | null>(null);
  const [addPos, setAddPos] = useState<{ x: number; y: number } | null>(null);
  const [addLabel, setAddLabel] = useState<string | null>(null);
  const append = useStore((s) => s.append);
  const tmpRef = useRef(0);

  const { data: allNodes } = useEntities(undefined, undefined, 200);
  const { data: allEdges } = useQuery<EdgeRow[]>({
    queryKey: ["edges-all"],
    queryFn: async () => {
      const r = await fetch(`${BASE}/query`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          cypher:
            "MATCH (s)-[r]->(e) RETURN id(r) AS id, type(r) AS type, id(s) AS source, id(e) AS target, properties(r) AS props LIMIT 1000",
          params: {},
        }),
      });
      if (!r.ok) throw new Error(`edges fetch ${r.status}`);
      const v = await r.json();
      // /query rows arrive as PropValue-tagged objects ({kind,value}); unwrap them.
      const u = (x: unknown): unknown => {
        if (x && typeof x === "object" && "kind" in x && "value" in x)
          return (x as { value: unknown }).value;
        return x;
      };
      return (v.rows ?? []).map((row: Record<string, unknown>) => ({
        id: u(row.id) as number,
        type: u(row.type) as string,
        source: u(row.source) as number,
        target: u(row.target) as number,
        props: (u(row.props) as Record<string, unknown>) ?? {},
      }));
    },
  });

  const nodesById = useMemo(() => {
    const m = new Map<
      number,
      { id: number; label: string; name: string; props: Record<string, unknown> }
    >();
    for (const n of allNodes ?? []) {
      m.set(n.id, {
        id: n.id,
        label: n.labels[0] ?? "?",
        name: String(n.props.name ?? n.id),
        props: n.props,
      });
    }
    return m;
  }, [allNodes]);

  const filteredNodes = useMemo(
    () =>
      Array.from(nodesById.values()).filter(
        (n) => labels.size === 0 || labels.has(n.label)
      ),
    [nodesById, labels]
  );

  const selected = selectedId != null ? nodesById.get(selectedId) ?? null : null;

  if (!schema) return <p className="p-6 text-sm text-slate-500">Loading…</p>;

  return (
    <div className="grid grid-rows-[48px_1fr] h-full">
      {/* ── Toolbar ── */}
      <div className="flex items-center gap-4 px-4 border-b border-slate-200 bg-white overflow-x-auto">
        {/* Label filter pills */}
        <div className="flex items-center gap-1 flex-shrink-0">
          {Object.keys(schema.nodes).map((l) => {
            const active = labels.has(l);
            return (
              <button
                key={l}
                onClick={() => {
                  const next = new Set(labels);
                  if (active) next.delete(l);
                  else next.add(l);
                  setLabels(next);
                }}
                className={`flex items-center gap-1.5 px-2 py-1 rounded-md text-xs transition-colors ${
                  active
                    ? "bg-blue-100 text-blue-700 border border-blue-200"
                    : "text-slate-400 hover:bg-slate-100 hover:text-slate-700"
                }`}
              >
                <span
                  className="w-2 h-2 rounded-full flex-shrink-0"
                  style={{ background: labelColor(l) }}
                />
                {l}
              </button>
            );
          })}
          {labels.size > 0 && (
            <button
              onClick={() => setLabels(new Set())}
              className="text-xs text-slate-400 hover:text-slate-700 px-1"
            >
              Clear
            </button>
          )}
        </div>

        <div className="ml-auto flex items-center gap-2 flex-shrink-0">
          {/* Layout switcher */}
          <select
            value={layout}
            onChange={(e) => setLayout(e.target.value)}
            className="h-8 rounded-md border border-slate-300 bg-white px-2 text-xs"
          >
            <option value="cose-bilkent">cose-bilkent</option>
            <option value="grid">grid</option>
            <option value="circle">circle</option>
            <option value="concentric">concentric</option>
            <option value="breadthfirst">breadthfirst</option>
          </select>
          {/* Zoom-to-fit — toggle the same layout name to force a re-run + fit in GraphCanvas */}
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setLayout((l) => l + " ")}
            title="Zoom to fit"
          >
            <Maximize2 size={14} />
            Fit
          </Button>
        </div>
      </div>

      {/* ── Canvas area ── */}
      <div className="relative overflow-hidden">
        <GraphCanvas
          nodes={filteredNodes.map((n) => ({
            id: n.id,
            label: n.label,
            name: n.name,
          }))}
          edges={allEdges ?? []}
          selectedId={selectedId}
          onSelectNode={setSelectedId}
          onLinkRequested={(fromId, toId) => {
            const src = nodesById.get(fromId);
            if (src) setLinkState({ fromId: src.id, fromLabel: src.label, toId });
          }}
          onAddNodeRequested={(pos) => setAddPos(pos)}
          layout={layout.trimEnd()}
        />

        {/* Hint banner */}
        <div className="pointer-events-none absolute top-3 left-1/2 -translate-x-1/2 bg-white/90 backdrop-blur px-3 py-1.5 rounded-md border border-slate-200 shadow-sm text-xs text-slate-500 whitespace-nowrap">
          Click node to inspect · Shift-drag node → node to link · Right-click canvas to add
        </div>

        {/* ── Add-node label picker (appears at right-click position) ── */}
        {addPos && (
          <div
            className="absolute z-30 bg-white rounded-lg border border-slate-200 shadow-xl p-2 w-52"
            style={{
              left: Math.min(addPos.x, window.innerWidth - 220),
              top: Math.min(addPos.y, window.innerHeight - 280),
            }}
          >
            <div className="text-xs font-semibold text-slate-500 uppercase tracking-wide px-2 pb-1.5">
              Add node
            </div>
            {Object.keys(schema.nodes).map((l) => (
              <button
                key={l}
                onClick={() => {
                  setAddLabel(l);
                  setAddPos(null);
                }}
                className="flex items-center gap-2 w-full text-left px-2 py-1.5 rounded-md text-sm hover:bg-slate-50 text-slate-700"
              >
                <span
                  className="w-2 h-2 rounded-full flex-shrink-0"
                  style={{ background: labelColor(l) }}
                />
                {l}
              </button>
            ))}
            <button
              onClick={() => setAddPos(null)}
              className="block w-full text-left px-2 py-1.5 text-xs text-slate-400 hover:bg-slate-50 rounded-md mt-0.5"
            >
              Cancel
            </button>
          </div>
        )}

        {/* ── Node detail panel (slides in from right) ── */}
        {selected && (
          <aside className="absolute top-3 right-3 bottom-3 w-[320px] z-20 flex flex-col">
            <Card className="flex flex-col h-full overflow-hidden">
              <CardHeader>
                <div className="flex items-center gap-2 min-w-0 flex-1">
                  <Badge tone="blue" className="flex-shrink-0">
                    {selected.label}
                  </Badge>
                  <CardTitle className="truncate">{selected.name}</CardTitle>
                </div>
                <button
                  onClick={() => setSelectedId(null)}
                  className="flex-shrink-0 text-slate-400 hover:text-slate-600 ml-2"
                >
                  <X size={16} />
                </button>
              </CardHeader>
              <CardContent className="overflow-auto flex-1 flex flex-col gap-4">
                {/* Props table */}
                <dl className="divide-y divide-slate-100">
                  {Object.entries(selected.props).map(([k, v]) => (
                    <div key={k} className="grid grid-cols-[100px_1fr] py-1.5 text-sm gap-2">
                      <dt className="text-slate-500 truncate">{k}</dt>
                      <dd className="text-slate-900 truncate">{String(v)}</dd>
                    </div>
                  ))}
                </dl>
                {/* Action buttons */}
                <div className="flex flex-wrap items-center gap-2 pt-1">
                  <Button
                    size="sm"
                    variant="primary"
                    onClick={() =>
                      setLinkState({ fromId: selected.id, fromLabel: selected.label })
                    }
                  >
                    <Plus size={14} />
                    Link
                  </Button>
                  <Button
                    size="sm"
                    variant="danger"
                    onClick={() => {
                      append({ kind: "delete_node", id: selected.id, cascade: true });
                      setSelectedId(null);
                    }}
                  >
                    <Trash2 size={14} />
                    Delete
                  </Button>
                  <RouterLink
                    to={`/entity/${selected.id}`}
                    className="inline-flex items-center gap-1 text-sm text-blue-600 hover:underline"
                  >
                    <ExternalLink size={14} />
                    Open
                  </RouterLink>
                </div>
              </CardContent>
            </Card>
          </aside>
        )}

        {/* ── Add-node form (modal overlay) ── */}
        {addLabel && (
          <div className="absolute inset-0 z-40 bg-slate-900/30 backdrop-blur-sm flex items-center justify-center">
            <Card className="w-[420px] max-h-[80vh] overflow-auto">
              <CardHeader>
                <CardTitle>New {addLabel}</CardTitle>
                <button
                  onClick={() => setAddLabel(null)}
                  className="text-slate-400 hover:text-slate-600"
                >
                  <X size={18} />
                </button>
              </CardHeader>
              <CardContent>
                <EntityForm
                  fields={schema.nodes[addLabel]?.props ?? []}
                  submitLabel="Stage"
                  onSubmit={(values) => {
                    append({
                      kind: "create_node",
                      tmpId: `tg${tmpRef.current++}`,
                      label: addLabel,
                      props: values,
                    });
                    setAddLabel(null);
                  }}
                />
              </CardContent>
            </Card>
          </div>
        )}

        {/* ── RelationshipPicker (slides in from right) ── */}
        {linkState && (
          <RelationshipPicker
            fromId={linkState.fromId}
            fromLabel={linkState.fromLabel}
            initialTarget={
              linkState.toId
                ? {
                    id: linkState.toId,
                    label: nodesById.get(linkState.toId)?.label ?? "",
                  }
                : undefined
            }
            onClose={() => setLinkState(null)}
            onStage={(type, targetId, props) => {
              append({
                kind: "create_link",
                tmpId: `tlg${Date.now()}`,
                type,
                start: { kind: "id", ref: linkState.fromId },
                end: { kind: "id", ref: targetId },
                props,
              });
              setLinkState(null);
            }}
          />
        )}
      </div>
    </div>
  );
}
