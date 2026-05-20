import { useState } from "react";
import { useSchema } from "../hooks/useSchema";
import { useEntities } from "../hooks/useEntities";
import EntityForm from "./EntityForm";

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
    <div role="dialog" aria-modal="true" style={{
      position: "fixed", top: 0, right: 0, width: 420, height: "100vh",
      background: "#fff", borderLeft: "1px solid #d1d5db", padding: 16,
      overflow: "auto", boxShadow: "-4px 0 12px rgba(0,0,0,0.08)", zIndex: 40,
    }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <h3 style={{ margin: 0 }}>Link from {p.fromLabel} #{p.fromId}</h3>
        <button onClick={p.onClose} style={closeBtn}>×</button>
      </div>

      <section style={{ marginTop: 16 }}>
        <div style={label}>1. Relationship type</div>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 4 }}>
          {allowedTypes.length === 0 && <span style={{ color: "#6b7280", fontSize: 13 }}>No relationship types defined from {p.fromLabel}.</span>}
          {allowedTypes.map(([t]) => (
            <button key={t} onClick={() => { setType(t); setTarget(null); }} style={t === type ? typeBtnActive : typeBtn}>{t}</button>
          ))}
        </div>
      </section>

      {type && schema.rels[type] && (
        <section style={{ marginTop: 16 }}>
          <div style={label}>2. Target {schema.rels[type].endpoints.find(([s]) => s === p.fromLabel)?.[1] ?? ""}</div>
          <TargetPicker
            allowedLabel={schema.rels[type].endpoints.find(([s]) => s === p.fromLabel)?.[1] ?? ""}
            onChoose={setTarget}
            chosen={target}
          />
        </section>
      )}

      {type && target && schema.rels[type] && (
        <section style={{ marginTop: 16 }}>
          <div style={label}>3. Properties (optional)</div>
          <EntityForm
            fields={schema.rels[type].props}
            submitLabel="Stage link"
            onSubmit={(values) => { p.onStage(type, target.id, values); p.onClose(); }}
          />
        </section>
      )}
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
      <input
        value={q}
        onChange={(e) => setQ(e.target.value)}
        placeholder={`Search ${allowedLabel}…`}
        style={{ width: "100%", padding: "6px 8px", border: "1px solid #d1d5db", borderRadius: 4 }}
      />
      <ul style={{ maxHeight: 180, overflow: "auto", margin: "6px 0 0", padding: 0, listStyle: "none", border: "1px solid #e5e7eb", borderRadius: 4 }}>
        {(data ?? []).map((e) => {
          const isChosen = chosen?.id === e.id;
          return (
            <li
              key={e.id}
              onClick={() => onChoose({ id: e.id, label: allowedLabel })}
              style={{
                padding: "6px 8px",
                cursor: "pointer",
                background: isChosen ? "#dbeafe" : "transparent",
                borderBottom: "1px solid #f3f4f6",
                fontSize: 14,
              }}
            >
              {String(e.props.name ?? e.id)}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

const closeBtn: React.CSSProperties = {
  background: "transparent", border: 0, fontSize: 20, cursor: "pointer", padding: "0 8px",
};
const label: React.CSSProperties = { fontSize: 13, color: "#374151", fontWeight: 600 };
const typeBtn: React.CSSProperties = {
  padding: "4px 10px", background: "#fff", border: "1px solid #d1d5db",
  borderRadius: 4, cursor: "pointer", fontSize: 13,
};
const typeBtnActive: React.CSSProperties = { ...typeBtn, background: "#dbeafe", borderColor: "#2563eb", fontWeight: 600 };
