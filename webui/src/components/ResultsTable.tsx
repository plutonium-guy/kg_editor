interface ResultsTableProps {
  rows: Record<string, unknown>[];
}

export default function ResultsTable(p: ResultsTableProps) {
  if (p.rows.length === 0) return null;
  const cols = Array.from(new Set(p.rows.flatMap((r) => Object.keys(r))));
  return (
    <table style={{ width: "100%", fontFamily: "monospace", fontSize: 12 }}>
      <thead><tr>{cols.map((c) => <th key={c}>{c}</th>)}</tr></thead>
      <tbody>
        {p.rows.slice(0, 200).map((r, i) => (
          <tr key={i}>{cols.map((c) => <td key={c}>{JSON.stringify(r[c])}</td>)}</tr>
        ))}
      </tbody>
    </table>
  );
}
