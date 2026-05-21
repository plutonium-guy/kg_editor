import { useState } from "react";
import { useSchema } from "../hooks/useSchema";
import { useEntities } from "../hooks/useEntities";
import EntityForm from "./EntityForm";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Badge } from "./ui/badge";
import { X, Sparkles } from "lucide-react";

interface PickerProps {
  fromId: number;
  fromLabel: string;
  initialTarget?: { id: number; label: string };
  onStage: (type: string, targetId: number, props: Record<string, unknown>) => void;
  onClose: () => void;
}

export default function RelationshipPicker(p: PickerProps) {
  const { data: schema } = useSchema();
  const [type, setType] = useState<string | null>(null);
  const [isCustom, setIsCustom] = useState(false);
  const [customType, setCustomType] = useState("");
  const [target, setTarget] = useState<{ id: number; label: string } | null>(p.initialTarget ?? null);

  if (!schema) return null;
  const allowedTypes = Object.entries(schema.rels).filter(([, def]) =>
    def.endpoints.some(([s]) => s === p.fromLabel)
  );

  const customLooksValid = /^[A-Za-z_][A-Za-z0-9_]*$/.test(customType);
  const effectiveType = isCustom ? (customLooksValid ? customType.toUpperCase() : null) : type;

  // For the props step we need rel-def props. Custom types have no props field —
  // we still let the user submit with empty props.
  const propsForType = effectiveType && schema.rels[effectiveType] ? schema.rels[effectiveType].props : [];

  return (
    <div className="fixed top-14 right-0 z-50 w-[420px] h-[calc(100vh-3.5rem)] bg-white border-l border-slate-200 shadow-xl overflow-auto">
      <div className="flex items-center justify-between px-5 py-3 border-b border-slate-200">
        <h2 className="font-semibold text-slate-900">Link from <Badge tone="blue">{p.fromLabel} #{p.fromId}</Badge></h2>
        <button onClick={p.onClose} className="text-slate-400 hover:text-slate-600"><X size={18} /></button>
      </div>

      <div className="p-5 space-y-5">
        <Step n={1} title="Relationship type">
          {allowedTypes.length === 0 && !isCustom && (
            <p className="text-sm text-slate-500 mb-2">No schema-defined types from {p.fromLabel}. Use a custom type below.</p>
          )}
          <div className="flex flex-wrap gap-2 mb-2">
            {allowedTypes.map(([t]) => (
              <button
                key={t}
                onClick={() => { setIsCustom(false); setType(t); }}
                className={`px-3 py-1.5 rounded-md text-sm transition-colors ${
                  !isCustom && t === type
                    ? "bg-blue-100 text-blue-700 border border-blue-300 font-medium"
                    : "bg-white text-slate-700 border border-slate-300 hover:bg-slate-50"
                }`}
              >{t}</button>
            ))}
            <button
              onClick={() => { setIsCustom(true); setType(null); }}
              className={`px-3 py-1.5 rounded-md text-sm transition-colors flex items-center gap-1.5 ${
                isCustom
                  ? "bg-amber-100 text-amber-800 border border-amber-300 font-medium"
                  : "bg-white text-slate-600 border border-dashed border-slate-300 hover:bg-slate-50"
              }`}
            >
              <Sparkles size={14} /> Custom…
            </button>
          </div>
          {isCustom && (
            <div className="space-y-1">
              <Input
                value={customType}
                onChange={(e) => setCustomType(e.target.value)}
                placeholder="e.g. MENTORS, OWNS, COLLABORATES_WITH"
                autoFocus
              />
              {customType.length > 0 && !customLooksValid && (
                <p className="text-xs text-red-600">Must start with a letter or underscore; only A-Z, 0-9, _</p>
              )}
              <p className="text-xs text-slate-500">Convention: SCREAMING_SNAKE_CASE. Stored as-is in Neo4j; not added to schema.</p>
            </div>
          )}
        </Step>

        {effectiveType && (
          <Step n={2} title="Target">
            <TargetPicker
              allowedLabel={
                isCustom
                  ? null
                  : schema.rels[effectiveType]?.endpoints.find(([s]) => s === p.fromLabel)?.[1] ?? null
              }
              onChoose={setTarget}
              chosen={target}
            />
          </Step>
        )}

        {effectiveType && target && (
          <Step n={3} title="Properties (optional)">
            {propsForType.length === 0 ? (
              <div>
                <p className="text-sm text-slate-500 mb-3">No properties defined for this type.</p>
                <Button
                  variant="primary"
                  onClick={() => { p.onStage(effectiveType, target.id, {}); p.onClose(); }}
                >Stage link</Button>
              </div>
            ) : (
              <EntityForm
                fields={propsForType}
                submitLabel="Stage link"
                onSubmit={(values) => { p.onStage(effectiveType, target.id, values); p.onClose(); }}
              />
            )}
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
  allowedLabel: string | null;
  onChoose: (e: { id: number; label: string }) => void;
  chosen: { id: number; label: string } | null;
}) {
  const { data: schema } = useSchema();
  const [labelOverride, setLabelOverride] = useState<string | null>(allowedLabel);
  const effectiveLabel = labelOverride ?? "";
  const [q, setQ] = useState("");
  const { data } = useEntities(effectiveLabel || undefined, q || undefined, 10);

  // If allowedLabel is null (custom type), let user pick the label first.
  if (!allowedLabel && schema && !labelOverride) {
    return (
      <div>
        <p className="text-xs text-slate-500 mb-2">Custom type — pick target label:</p>
        <div className="flex flex-wrap gap-2">
          {Object.keys(schema.nodes).map((l) => (
            <button
              key={l}
              onClick={() => setLabelOverride(l)}
              className="px-3 py-1.5 rounded-md text-sm bg-white text-slate-700 border border-slate-300 hover:bg-slate-50"
            >{l}</button>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div>
      <Input value={q} onChange={(e) => setQ(e.target.value)} placeholder={`Search ${effectiveLabel}…`} />
      <ul className="mt-2 max-h-44 overflow-auto rounded-md border border-slate-200 divide-y divide-slate-100">
        {(data ?? []).length === 0 && <li className="px-3 py-2 text-sm text-slate-400">No matches.</li>}
        {(data ?? []).map((e) => {
          const isChosen = chosen?.id === e.id;
          return (
            <li
              key={e.id}
              onClick={() => onChoose({ id: e.id, label: effectiveLabel })}
              className={`px-3 py-2 text-sm cursor-pointer ${isChosen ? "bg-blue-50 text-blue-700 font-medium" : "hover:bg-slate-50 text-slate-700"}`}
            >
              {String(e.props.name ?? e.id)}
            </li>
          );
        })}
      </ul>
      {!allowedLabel && (
        <button
          onClick={() => setLabelOverride(null)}
          className="mt-2 text-xs text-slate-500 hover:text-slate-700"
        >← Change target label</button>
      )}
    </div>
  );
}
