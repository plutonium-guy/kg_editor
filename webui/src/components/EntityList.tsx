import { Link } from "react-router-dom";
import { useEntities } from "../hooks/useEntities";

export default function EntityList({ label, q }: { label?: string; q?: string }) {
  const { data, isLoading, error } = useEntities(label, q);
  if (isLoading) return <p style={{ color: "#6b7280" }}>Loading…</p>;
  if (error) return <p style={{ color: "#dc2626" }}>{String(error)}</p>;
  if (!data || data.length === 0) return <p style={{ color: "#6b7280" }}>No entities.</p>;

  const sampleProps = data[0]?.props ?? {};
  const nameKey = "name" in sampleProps ? "name" : Object.keys(sampleProps)[0] ?? "id";
  const otherKeys = Object.keys(sampleProps).filter((k) => k !== nameKey).slice(0, 3);

  return (
    <table style={{ width: "100%", borderCollapse: "collapse" }}>
      <thead>
        <tr style={{ borderBottom: "1px solid #d1d5db", textAlign: "left", color: "#374151" }}>
          <th style={th}>{nameKey}</th>
          {otherKeys.map((k) => <th key={k} style={th}>{k}</th>)}
          <th style={th}>id</th>
        </tr>
      </thead>
      <tbody>
        {data.map((e) => (
          <tr key={e.id} style={{ borderBottom: "1px solid #e5e7eb" }}>
            <td style={td}><Link to={`/entity/${e.id}`} style={{ color: "#2563eb" }}>{String(e.props[nameKey] ?? "")}</Link></td>
            {otherKeys.map((k) => <td key={k} style={td}>{String(e.props[k] ?? "")}</td>)}
            <td style={{ ...td, color: "#9ca3af" }}>{e.id}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

const th: React.CSSProperties = { padding: "8px", fontWeight: 600, fontSize: 13 };
const td: React.CSSProperties = { padding: "8px", fontSize: 14 };
