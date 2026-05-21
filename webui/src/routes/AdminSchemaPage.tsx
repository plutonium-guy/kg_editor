import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useSchema } from "../hooks/useSchema";
import * as api from "../kg/client";
import NodeLabelEditor from "../components/NodeLabelEditor";
import RelTypeEditor from "../components/RelTypeEditor";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Plus, RefreshCw } from "lucide-react";
import type { NodeDef, RelDef } from "../kg/schema";

export default function AdminSchemaPage() {
  const { data: schema } = useSchema();
  const qc = useQueryClient();
  const [tab, setTab] = useState<"nodes" | "rels">("nodes");
  const [activeLabel, setActiveLabel] = useState<string | null>(null);
  const [activeRel, setActiveRel] = useState<string | null>(null);
  const [newName, setNewName] = useState("");
  const [err, setErr] = useState<string | null>(null);

  if (!schema) return <p className="p-6 text-sm text-slate-500">Loading…</p>;

  const labels = Object.keys(schema.nodes).sort();
  const rels = Object.keys(schema.rels).sort();
  const current = tab === "nodes" ? activeLabel : activeRel;

  const handleErr = (e: unknown) => setErr(String(e));
  const ok = () => { setErr(null); qc.invalidateQueries({ queryKey: ["schema"] }); };

  const reload = async () => {
    try { await api.reloadSchema(); ok(); } catch (e) { handleErr(e); }
  };

  return (
    <div className="grid grid-cols-[240px_1fr] h-full">
      <aside className="border-r border-slate-200 bg-white overflow-auto">
        <div className="p-3 border-b border-slate-200 flex items-center justify-between">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-500">Schema admin</h2>
          <Button size="icon" variant="ghost" onClick={reload} title="Reload from server"><RefreshCw size={14} /></Button>
        </div>
        <div className="p-2">
          <div className="flex gap-1 mb-2">
            <button
              onClick={() => setTab("nodes")}
              className={`flex-1 px-2 py-1 text-xs rounded ${tab === "nodes" ? "bg-slate-200 font-semibold" : "text-slate-600 hover:bg-slate-50"}`}
            >Labels ({labels.length})</button>
            <button
              onClick={() => setTab("rels")}
              className={`flex-1 px-2 py-1 text-xs rounded ${tab === "rels" ? "bg-slate-200 font-semibold" : "text-slate-600 hover:bg-slate-50"}`}
            >Rels ({rels.length})</button>
          </div>

          <div className="flex gap-1 mb-2">
            <Input
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder={tab === "nodes" ? "NewLabel" : "NEW_REL"}
              className="h-8 text-sm"
            />
            <Button
              size="sm"
              variant="primary"
              onClick={async () => {
                const trimmed = newName.trim();
                if (!trimmed) return;
                try {
                  if (tab === "nodes") {
                    await api.createSchemaNode(trimmed, { description: "", props: [], indexes: [] });
                    setActiveLabel(trimmed);
                  } else {
                    await api.createSchemaRel(trimmed.toUpperCase(), { endpoints: [], cardinality: "many_to_many", props: [] });
                    setActiveRel(trimmed.toUpperCase());
                  }
                  setNewName("");
                  ok();
                } catch (e) { handleErr(e); }
              }}
            ><Plus size={14} /></Button>
          </div>

          <ul className="space-y-0.5">
            {(tab === "nodes" ? labels : rels).map((name) => (
              <li key={name}>
                <button
                  onClick={() => tab === "nodes" ? setActiveLabel(name) : setActiveRel(name)}
                  className={`w-full text-left px-2 py-1 rounded text-sm ${
                    current === name
                      ? "bg-blue-50 text-blue-700 font-medium"
                      : "text-slate-700 hover:bg-slate-50"
                  }`}
                >
                  {name}
                </button>
              </li>
            ))}
          </ul>
        </div>
      </aside>

      <section className="overflow-auto p-6">
        {err && (
          <div className="mb-4 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">{err}</div>
        )}
        {!current && (
          <div className="text-sm text-slate-500">
            <p>Select a {tab === "nodes" ? "label" : "relationship type"} on the left, or add a new one.</p>
            <p className="mt-2">Schema is persisted in Neo4j under <code className="bg-slate-100 px-1 rounded">:_Schema</code> nodes. Changes apply immediately to all clients on next refresh.</p>
          </div>
        )}
        {current && tab === "nodes" && schema.nodes[current] && (
          <NodeLabelEditor
            label={current}
            initial={schema.nodes[current]}
            schema={schema}
            onSave={async (def: NodeDef) => {
              try { await api.updateSchemaNode(current, def); ok(); } catch (e) { handleErr(e); }
            }}
            onDelete={async () => {
              if (!confirm(`Delete label "${current}"? Entities with this label keep their data; only the schema definition is removed.`)) return;
              try { await api.deleteSchemaNode(current); setActiveLabel(null); ok(); } catch (e) { handleErr(e); }
            }}
          />
        )}
        {current && tab === "rels" && schema.rels[current] && (
          <RelTypeEditor
            type={current}
            initial={schema.rels[current]}
            schema={schema}
            onSave={async (def: RelDef) => {
              try { await api.updateSchemaRel(current, def); ok(); } catch (e) { handleErr(e); }
            }}
            onDelete={async () => {
              if (!confirm(`Delete rel type "${current}"?`)) return;
              try { await api.deleteSchemaRel(current); setActiveRel(null); ok(); } catch (e) { handleErr(e); }
            }}
          />
        )}
      </section>
    </div>
  );
}
