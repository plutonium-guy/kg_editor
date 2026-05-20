import { useState } from "react";
import { useSchema } from "../hooks/useSchema";
import { useEntities } from "../hooks/useEntities";
import EntityForm from "./EntityForm";
import { Input } from "./ui/input";
import { Badge } from "./ui/badge";
import { X } from "lucide-react";

interface PickerProps {
  fromId: number;
  fromLabel: string;
  onStage: (type: string, targetId: number, props: Record<string, unknown>) => void;
  onClose: () => void;
}

export default function RelationshipPicker(p: PickerProps) {
  const { data: schema } = useSchema();
  const [type, setType] = useState<string | null>(null);
  const [target, setTarget] = useState<{ id: number; label: string } | null>(null);

  if (!schema) return null;
  // Only show rel types whose first endpoint label matches fromLabel.
  const allowedTypes = Object.entries(schema.rels).filter(([, def]) =>
    def.endpoints.some(([s]) => s === p.fromLabel)
  );

  return (
    <div className="fixed top-14 right-0 z-50 w-[420px] h-[calc(100vh-3.5rem)] bg-white border-l border-slate-200 shadow-xl overflow-auto">
      <div className="flex items-center justify-between px-5 py-3 border-b border-slate-200">
        <h2 className="font-semibold text-slate-900">Link from <Badge tone="blue">{p.fromLabel} #{p.fromId}</Badge></h2>
        <button onClick={p.onClose} className="text-slate-400 hover:text-slate-600"><X size={18} /></button>
      </div>

      <div className="p-5 space-y-5">
        <Step n={1} title="Relationship type">
          {allowedTypes.length === 0 ? (
            <p className="text-sm text-slate-500">No relationship types defined from {p.fromLabel}.</p>
          ) : (
            <div className="flex flex-wrap gap-2">
              {allowedTypes.map(([t]) => (
                <button
                  key={t}
                  onClick={() => { setType(t); setTarget(null); }}
                  className={`px-3 py-1.5 rounded-md text-sm transition-colors ${
                    t === type
                      ? "bg-blue-100 text-blue-700 border border-blue-300 font-medium"
                      : "bg-white text-slate-700 border border-slate-300 hover:bg-slate-50"
                  }`}
                >{t}</button>
              ))}
            </div>
          )}
        </Step>

        {type && schema.rels[type] && (
          <Step n={2} title={`Target ${schema.rels[type].endpoints.find(([s]) => s === p.fromLabel)?.[1] ?? ""}`}>
            <TargetPicker
              allowedLabel={schema.rels[type].endpoints.find(([s]) => s === p.fromLabel)?.[1] ?? ""}
              onChoose={setTarget}
              chosen={target}
            />
          </Step>
        )}

        {type && target && schema.rels[type] && (
          <Step n={3} title="Properties (optional)">
            <EntityForm
              fields={schema.rels[type].props}
              submitLabel="Stage link"
              onSubmit={(values) => { p.onStage(type, target.id, values); p.onClose(); }}
            />
          </Step>
        )}
      </div>
    </div>
  );
}

function Step({ n, title, children }: { n: number; title: string; children: React.ReactNode }) {
  return (
    <div>
      <h3 className="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-2">
        <span className="inline-flex items-center justify-center w-5 h-5 rounded-full bg-slate-200 text-slate-700 mr-2 text-[10px]">{n}</span>
        {title}
      </h3>
      {children}
    </div>
  );
}

function TargetPicker({
  allowedLabel, onChoose, chosen,
}: {
  allowedLabel: string;
  onChoose: (e: { id: number; label: string }) => void;
  chosen: { id: number; label: string } | null;
}) {
  const [q, setQ] = useState("");
  const { data } = useEntities(allowedLabel, q || undefined, 10);
  return (
    <div>
      <Input value={q} onChange={(e) => setQ(e.target.value)} placeholder={`Search ${allowedLabel}…`} />
      <ul className="mt-2 max-h-44 overflow-auto rounded-md border border-slate-200 divide-y divide-slate-100">
        {(data ?? []).length === 0 && <li className="px-3 py-2 text-sm text-slate-400">No matches.</li>}
        {(data ?? []).map((e) => {
          const isChosen = chosen?.id === e.id;
          return (
            <li
              key={e.id}
              onClick={() => onChoose({ id: e.id, label: allowedLabel })}
              className={`px-3 py-2 text-sm cursor-pointer ${isChosen ? "bg-blue-50 text-blue-700 font-medium" : "hover:bg-slate-50 text-slate-700"}`}
            >
              {String(e.props.name ?? e.id)}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

