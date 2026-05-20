import { Link, useParams } from "react-router-dom";
import { useSchema } from "../hooks/useSchema";
import { useEntities } from "../hooks/useEntities";

export default function EntityListSidebar() {
  const { data: schema } = useSchema();
  if (!schema) return null;
  const labels = Object.keys(schema.nodes).sort();
  return (
    <aside style={{ width: 240, borderRight: "1px solid #d1d5db", padding: 12, overflow: "auto", background: "#fafafa" }}>
      {labels.map((label) => <LabelGroup key={label} label={label} />)}
    </aside>
  );
}

function LabelGroup({ label }: { label: string }) {
  const { data } = useEntities(label);
  const { id: currentId } = useParams();
  return (
    <details open style={{ marginBottom: 12 }}>
      <summary style={{ cursor: "pointer", padding: "4px 0", color: "#1f2937" }}>
        <strong>{label}</strong> <span style={{ color: "#6b7280" }}>({data?.length ?? "…"})</span>
      </summary>
      <ul style={{ paddingLeft: 16, listStyle: "none", margin: 0 }}>
        {(data ?? []).slice(0, 25).map((e) => (
          <li key={e.id} style={{ padding: "2px 0" }}>
            <Link to={`/entity/${e.id}`} style={{
              color: String(e.id) === currentId ? "#2563eb" : "#374151",
              fontWeight: String(e.id) === currentId ? 600 : 400,
              textDecoration: "none",
              fontSize: 14,
            }}>
              {String(e.props.name ?? e.id)}
            </Link>
          </li>
        ))}
      </ul>
      <Link to={`/browse?label=${label}&add=1`} style={{ fontSize: 12, color: "#2563eb" }}>+ Add {label}</Link>
    </details>
  );
}
