import { useState } from "react";
import { useSchema } from "../hooks/useSchema";
import { useEntities } from "../hooks/useEntities";
import { useQuery } from "@tanstack/react-query";
import GraphCanvas from "../components/GraphCanvas";

const BASE = (import.meta.env.VITE_KG_SERVER_BASE as string | undefined) ?? "http://localhost:9000";

interface EdgeRow { id: number; type: string; source: number; target: number; }

export default function GraphPage() {
  const { data: schema } = useSchema();
  const [labels, setLabels] = useState<Set<string>>(new Set());

  const { data: allNodes } = useEntities(undefined, undefined, 200);
  const { data: allEdges } = useQuery<EdgeRow[]>({
    queryKey: ["edges-all"],
    queryFn: async () => {
      const r = await fetch(`${BASE}/query`, {
        method: "POST", headers: { "content-type": "application/json" },
        body: JSON.stringify({
          cypher: "MATCH (s)-[r]->(e) RETURN id(r) AS id, type(r) AS type, id(s) AS source, id(e) AS target LIMIT 500",
          params: {},
        }),
      });
      if (!r.ok) throw new Error(`edges fetch ${r.status}`);
      const v = await r.json();
      // /query rows arrive as PropValue-tagged objects ({kind,value}); unwrap them.
      const u = (x: unknown): unknown => {
        if (x && typeof x === "object" && "kind" in x && "value" in x) return (x as { value: unknown }).value;
        return x;
      };
      return (v.rows ?? []).map((row: Record<string, unknown>) => ({
        id:     u(row.id) as number,
        type:   u(row.type) as string,
        source: u(row.source) as number,
        target: u(row.target) as number,
      }));
    },
  });

  if (!schema) return <p style={{ padding: 24 }}>Loading…</p>;

  const filteredNodes = (allNodes ?? [])
    .filter((n) => labels.size === 0 || labels.has(n.labels[0]))
    .map((n) => ({ id: n.id, label: n.labels[0] ?? "?", name: String(n.props.name ?? n.id) }));
  const filteredEdges = (allEdges ?? []);

  return (
    <div style={{ display: "grid", gridTemplateColumns: "220px 1fr", height: "100%" }}>
      <aside style={{ borderRight: "1px solid #d1d5db", padding: 12, background: "#fafafa", overflow: "auto" }}>
        <h3 style={{ margin: "0 0 8px", color: "#374151", fontSize: 14 }}>Labels</h3>
        {Object.keys(schema.nodes).map((l) => (
          <label key={l} style={{ display: "flex", alignItems: "center", gap: 6, padding: "4px 0", fontSize: 14, cursor: "pointer" }}>
            <input
              type="checkbox"
              checked={labels.has(l)}
              onChange={(e) => {
                const next = new Set(labels);
                if (e.target.checked) next.add(l); else next.delete(l);
                setLabels(next);
              }}
            />
            <span style={{ display: "inline-block", width: 10, height: 10, borderRadius: 5, background: labelDot(l) }} />
            {l}
          </label>
        ))}
        <p style={{ fontSize: 11, color: "#6b7280", marginTop: 12 }}>Empty selection shows all labels.</p>
      </aside>
      <GraphCanvas nodes={filteredNodes} edges={filteredEdges} />
    </div>
  );
}

function labelDot(label: string): string {
  let h = 0;
  for (let i = 0; i < label.length; i++) h = (h * 31 + label.charCodeAt(i)) % 360;
  return `hsl(${h}, 55%, 48%)`;
}
