import { Link } from "react-router-dom";
import { useEntities } from "../hooks/useEntities";

export default function EntityList({ label, q }: { label?: string; q?: string }) {
  const { data, isLoading, error } = useEntities(label, q);
  if (isLoading) return <p className="text-sm text-slate-500">Loading…</p>;
  if (error) return <p className="text-sm text-red-600">{String(error)}</p>;
  if (!data || data.length === 0) return <p className="text-sm text-slate-500">No entities yet.</p>;

  const sample = data[0]?.props ?? {};
  const nameKey = "name" in sample ? "name" : Object.keys(sample)[0] ?? "id";
  const otherKeys = Object.keys(sample).filter((k) => k !== nameKey).slice(0, 3);

  return (
    <div className="overflow-hidden rounded-lg border border-slate-200 bg-white">
      <table className="w-full text-sm">
        <thead className="bg-slate-50 border-b border-slate-200">
          <tr className="text-left">
            <th className="px-4 py-2 font-semibold text-slate-600">{nameKey}</th>
            {otherKeys.map((k) => <th key={k} className="px-4 py-2 font-semibold text-slate-600">{k}</th>)}
            <th className="px-4 py-2 font-semibold text-slate-600 text-right">id</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-slate-100">
          {data.map((e) => (
            <tr key={e.id} className="hover:bg-slate-50">
              <td className="px-4 py-2"><Link to={`/entity/${e.id}`} className="text-blue-600 hover:underline">{String(e.props[nameKey] ?? "")}</Link></td>
              {otherKeys.map((k) => <td key={k} className="px-4 py-2 text-slate-700">{String(e.props[k] ?? "")}</td>)}
              <td className="px-4 py-2 text-slate-400 text-right">{e.id}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
