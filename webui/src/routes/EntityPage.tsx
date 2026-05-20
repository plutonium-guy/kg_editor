import { useParams, Link } from "react-router-dom";
import { useState } from "react";
import { useEntity } from "../hooks/useEntity";
import { useSchema } from "../hooks/useSchema";
import { useStore } from "../state/store";
import EntityForm from "../components/EntityForm";
import RelationshipPicker from "../components/RelationshipPicker";

export default function EntityPage() {
  const { id } = useParams();
  const numId = Number(id);
  const { data: e, isLoading } = useEntity(numId);
  const { data: schema } = useSchema();
  const append = useStore((s) => s.append);
  const [editing, setEditing] = useState(false);
  const [linking, setLinking] = useState(false);

  if (isLoading) return <p style={{ padding: 24, color: "#6b7280" }}>Loading…</p>;
  if (!e || !schema) return <p style={{ padding: 24 }}>Not found.</p>;

  const label = e.labels[0];
  const def = label ? schema.nodes[label] : undefined;

  return (
    <div style={{ padding: 24, maxWidth: 880, margin: "0 auto" }}>
      <h2 style={{ margin: "0 0 24px", color: "#1f2937" }}>
        {label} <span style={{ color: "#9ca3af", fontWeight: "normal" }}>#{e.id}</span>
      </h2>

      <section style={card}>
        <header style={cardHeader}>
          <h3 style={cardTitle}>Properties</h3>
          {!editing && def && (
            <div>
              <button onClick={() => setEditing(true)} style={btnSecondary}>Edit</button>{" "}
              <button onClick={() => append({ kind: "delete_node", id: numId, cascade: true })} style={btnDanger}>Delete entity</button>
            </div>
          )}
        </header>
        {!editing ? (
          <dl style={{ margin: 0 }}>
            {Object.entries(e.props).length === 0 && <p style={{ color: "#6b7280", fontSize: 14 }}>No properties.</p>}
            {Object.entries(e.props).map(([k, v]) => (
              <div key={k} style={{ display: "grid", gridTemplateColumns: "140px 1fr", padding: "6px 0", borderBottom: "1px solid #f3f4f6" }}>
                <dt style={{ color: "#6b7280", fontSize: 13 }}>{k}</dt>
                <dd style={{ margin: 0, fontSize: 14, color: "#1f2937" }}>{formatValue(v)}</dd>
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
        ) : <p>Unknown schema for label {label}</p>}
      </section>

      <section style={{ ...card, marginTop: 16 }}>
        <header style={cardHeader}>
          <h3 style={cardTitle}>Relationships</h3>
          <button onClick={() => setLinking(true)} style={btnSecondary}>+ Link</button>
        </header>
        <ul style={{ margin: 0, padding: 0, listStyle: "none" }}>
          {e.out_rels.length === 0 && e.in_rels.length === 0 && (
            <li style={{ color: "#6b7280", fontSize: 14 }}>No relationships.</li>
          )}
          {e.out_rels.map((r) => (
            <li key={`o${r.id}`} style={relRow}>
              <span style={{ color: "#2563eb", fontWeight: 600 }}>{r.type}</span>{" → "}
              <Link to={`/entity/${r.target_id}`} style={{ color: "#1f2937" }}>
                {r.target_labels[0]} #{r.target_id}
              </Link>
              <button onClick={() => append({ kind: "delete_link", id: r.id })} style={btnRemove}>×</button>
            </li>
          ))}
          {e.in_rels.map((r) => (
            <li key={`i${r.id}`} style={relRow}>
              {"← "}<span style={{ color: "#2563eb", fontWeight: 600 }}>{r.type}</span>{" "}
              <Link to={`/entity/${r.source_id}`} style={{ color: "#1f2937" }}>
                {r.source_labels[0]} #{r.source_id}
              </Link>
              <button onClick={() => append({ kind: "delete_link", id: r.id })} style={btnRemove}>×</button>
            </li>
          ))}
        </ul>
      </section>

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
}

function formatValue(v: unknown): string {
  if (v == null) return "";
  if (typeof v === "object") return JSON.stringify(v);
  return String(v);
}

const card: React.CSSProperties = {
  background: "#fff", border: "1px solid #e5e7eb", borderRadius: 8, padding: 16,
};
const cardHeader: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12,
};
const cardTitle: React.CSSProperties = { margin: 0, fontSize: 15, color: "#374151" };
const btnSecondary: React.CSSProperties = {
  padding: "4px 12px", background: "#fff", border: "1px solid #d1d5db",
  borderRadius: 4, cursor: "pointer", fontSize: 13,
};
const btnDanger: React.CSSProperties = {
  padding: "4px 12px", background: "#fff", border: "1px solid #dc2626",
  borderRadius: 4, cursor: "pointer", fontSize: 13, color: "#dc2626",
};
const btnRemove: React.CSSProperties = {
  background: "transparent", border: 0, color: "#dc2626", cursor: "pointer", marginLeft: 8,
};
const relRow: React.CSSProperties = {
  padding: "6px 0", borderBottom: "1px solid #f3f4f6", fontSize: 14, display: "flex", alignItems: "center", gap: 4,
};
