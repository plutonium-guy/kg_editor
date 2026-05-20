import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { search } from "../kg/client";

export default function SearchPage() {
  const [q, setQ] = useState("");
  const { data, isLoading } = useQuery({
    queryKey: ["search", q],
    queryFn: () => search(q),
    enabled: q.length >= 1,
  });
  return (
    <div style={{ padding: 24, maxWidth: 800, margin: "0 auto" }}>
      <input
        value={q}
        onChange={(e) => setQ(e.target.value)}
        placeholder="Search across all entities…"
        autoFocus
        style={{ width: "100%", padding: "10px 12px", fontSize: 16, border: "1px solid #d1d5db", borderRadius: 6 }}
      />
      {isLoading && <p style={{ color: "#6b7280" }}>Searching…</p>}
      <ul style={{ marginTop: 12, padding: 0, listStyle: "none" }}>
        {(data ?? []).map((e) => (
          <li key={e.id} style={{ padding: "8px 0", borderBottom: "1px solid #e5e7eb" }}>
            <Link to={`/entity/${e.id}`} style={{ color: "#2563eb", textDecoration: "none" }}>
              <strong>{e.labels[0]}</strong> — {String(e.props.name ?? e.id)}
            </Link>
          </li>
        ))}
        {q.length >= 1 && data && data.length === 0 && !isLoading && (
          <li style={{ color: "#6b7280", padding: "8px 0" }}>No matches.</li>
        )}
      </ul>
    </div>
  );
}
