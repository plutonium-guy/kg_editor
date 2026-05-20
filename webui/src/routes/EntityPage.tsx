import { useParams, Link } from "react-router-dom";
import { useState } from "react";
import { useEntity } from "../hooks/useEntity";
import { useSchema } from "../hooks/useSchema";
import { useStore } from "../state/store";
import EntityForm from "../components/EntityForm";
import RelationshipPicker from "../components/RelationshipPicker";
import { Card, CardHeader, CardTitle, CardContent } from "../components/ui/card";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { Trash2, Pencil, Plus, X, ArrowRight, ArrowLeft } from "lucide-react";
import EntityListSidebar from "../components/EntityListSidebar";

export default function EntityPage() {
  const { id } = useParams();
  const numId = Number(id);
  const { data: e, isLoading } = useEntity(numId);
  const { data: schema } = useSchema();
  const append = useStore((s) => s.append);
  const [editing, setEditing] = useState(false);
  const [linking, setLinking] = useState(false);

  return (
    <div className="grid grid-cols-[16rem_1fr] h-full">
      <EntityListSidebar />
      <section className="overflow-auto p-6">
        {isLoading && <p className="text-sm text-slate-500">Loading…</p>}
        {!isLoading && (!e || !schema) && <p className="text-sm text-slate-500">Not found.</p>}
        {!isLoading && e && schema && (() => {
          const label = e.labels[0];
          const def = label ? schema.nodes[label] : undefined;
          return (
            <div className="max-w-3xl mx-auto space-y-4">
              <header className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <Badge tone="blue">{label}</Badge>
                  <h1 className="text-2xl font-bold text-slate-900">
                    {String(e.props.name ?? `#${e.id}`)}
                  </h1>
                  <span className="text-slate-400 text-sm">#{e.id}</span>
                </div>
              </header>

              <Card>
                <CardHeader>
                  <CardTitle>Properties</CardTitle>
                  {!editing && def && (
                    <div className="flex gap-2">
                      <Button size="sm" variant="outline" onClick={() => setEditing(true)}><Pencil size={14} /> Edit</Button>
                      <Button size="sm" variant="danger" onClick={() => append({ kind: "delete_node", id: numId, cascade: true })}><Trash2 size={14} /> Delete</Button>
                    </div>
                  )}
                </CardHeader>
                <CardContent>
                  {!editing ? (
                    <dl className="divide-y divide-slate-100">
                      {Object.entries(e.props).length === 0 && <p className="text-sm text-slate-400">No properties.</p>}
                      {Object.entries(e.props).map(([k, v]) => (
                        <div key={k} className="grid grid-cols-[140px_1fr] py-2">
                          <dt className="text-sm text-slate-500">{k}</dt>
                          <dd className="text-sm text-slate-900">{formatValue(v)}</dd>
                        </div>
                      ))}
                    </dl>
                  ) : def ? (
                    <EntityForm
                      fields={def.props}
                      initial={e.props}
                      submitLabel="Stage update"
                      onSubmit={(values) => {
                        const set: Record<string, unknown> = {};
                        const unset: string[] = [];
                        for (const f of def.props) {
                          const newVal = values[f.name];
                          const oldVal = e.props[f.name];
                          if (newVal == null || newVal === "") {
                            if (oldVal != null) unset.push(f.name);
                          } else if (newVal !== oldVal) {
                            set[f.name] = newVal;
                          }
                        }
                        if (Object.keys(set).length > 0 || unset.length > 0) {
                          append({ kind: "update_node", id: numId, set, unset });
                        }
                        setEditing(false);
                      }}
                    />
                  ) : <p className="text-sm text-slate-500">Unknown schema for label {label}</p>}
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>Relationships</CardTitle>
                  <Button size="sm" variant="primary" onClick={() => setLinking(true)}><Plus size={14} /> Link</Button>
                </CardHeader>
                <CardContent>
                  {(e.out_rels.length === 0 && e.in_rels.length === 0) && <p className="text-sm text-slate-400">No relationships.</p>}
                  <ul className="divide-y divide-slate-100">
                    {e.out_rels.map((r) => (
                      <li key={`o${r.id}`} className="flex items-center justify-between py-2 text-sm">
                        <span className="flex items-center gap-2 text-slate-700">
                          <Badge tone="blue">{r.type}</Badge>
                          <ArrowRight size={14} className="text-slate-400" />
                          <Link to={`/entity/${r.target_id}`} className="text-blue-600 hover:underline">
                            {r.target_labels[0]} #{r.target_id}
                          </Link>
                        </span>
                        <button onClick={() => append({ kind: "delete_link", id: r.id })} className="text-slate-400 hover:text-red-600"><X size={16} /></button>
                      </li>
                    ))}
                    {e.in_rels.map((r) => (
                      <li key={`i${r.id}`} className="flex items-center justify-between py-2 text-sm">
                        <span className="flex items-center gap-2 text-slate-700">
                          <Link to={`/entity/${r.source_id}`} className="text-blue-600 hover:underline">
                            {r.source_labels[0]} #{r.source_id}
                          </Link>
                          <ArrowLeft size={14} className="text-slate-400" />
                          <Badge tone="slate">{r.type}</Badge>
                        </span>
                        <button onClick={() => append({ kind: "delete_link", id: r.id })} className="text-slate-400 hover:text-red-600"><X size={16} /></button>
                      </li>
                    ))}
                  </ul>
                </CardContent>
              </Card>

              {linking && label && (
                <RelationshipPicker
                  fromId={numId}
                  fromLabel={label}
                  onClose={() => setLinking(false)}
                  onStage={(type, targetId, props) => append({
                    kind: "create_link",
                    tmpId: `tl${Date.now()}`,
                    type,
                    start: { kind: "id", ref: numId },
                    end:   { kind: "id", ref: targetId },
                    props,
                  })}
                />
              )}
            </div>
          );
        })()}
      </section>
    </div>
  );
}

function formatValue(v: unknown): string {
  if (v == null) return "—";
  if (typeof v === "object") return JSON.stringify(v);
  return String(v);
}
