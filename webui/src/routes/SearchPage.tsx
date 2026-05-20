import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { search } from "../kg/client";
import { Input } from "../components/ui/input";
import { Badge } from "../components/ui/badge";
import { Search as SearchIcon } from "lucide-react";

export default function SearchPage() {
  const [q, setQ] = useState("");
  const { data, isLoading } = useQuery({
    queryKey: ["search", q],
    queryFn: () => search(q),
    enabled: q.length >= 1,
  });
  return (
    <div className="p-6 max-w-3xl mx-auto">
      <div className="relative">
        <SearchIcon size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
        <Input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="Search across all entities…"
          autoFocus
          className="pl-9 h-11 text-base"
        />
      </div>
      {isLoading && <p className="mt-4 text-sm text-slate-500">Searching…</p>}
      {q.length >= 1 && data && data.length === 0 && !isLoading && (
        <p className="mt-4 text-sm text-slate-500">No matches.</p>
      )}
      <ul className="mt-4 divide-y divide-slate-100 rounded-lg border border-slate-200 bg-white">
        {(data ?? []).map((e) => (
          <li key={e.id} className="px-4 py-3 hover:bg-slate-50">
            <Link to={`/entity/${e.id}`} className="flex items-center gap-3">
              <Badge tone="blue">{e.labels[0]}</Badge>
              <span className="font-medium text-slate-900">{String(e.props.name ?? e.id)}</span>
              <span className="text-slate-400 text-sm ml-auto">#{e.id}</span>
            </Link>
          </li>
        ))}
      </ul>
    </div>
  );
}
