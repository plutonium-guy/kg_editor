import { Link, useParams } from "react-router-dom";
import { useSchema } from "../hooks/useSchema";
import { useEntities } from "../hooks/useEntities";
import { ChevronDown, Plus } from "lucide-react";
import { useState } from "react";

export default function EntityListSidebar() {
  const { data: schema } = useSchema();
  if (!schema) return null;
  const labels = Object.keys(schema.nodes).sort();
  return (
    <aside className="w-64 border-r border-slate-200 bg-white overflow-auto">
      <div className="p-3 border-b border-slate-200">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-500">Entities</h2>
      </div>
      <div className="p-2">
        {labels.map((l) => <LabelGroup key={l} label={l} />)}
      </div>
    </aside>
  );
}

function labelColor(label: string): string {
  let h = 0;
  for (let i = 0; i < label.length; i++) h = (h * 31 + label.charCodeAt(i)) % 360;
  return `hsl(${h}, 55%, 48%)`;
}

function LabelGroup({ label }: { label: string }) {
  const { data } = useEntities(label);
  const { id: currentId } = useParams();
  const [open, setOpen] = useState(true);
  return (
    <div className="mb-1">
      <div className="flex items-center justify-between px-2 py-1.5 rounded-md hover:bg-slate-50">
        <button onClick={() => setOpen(!open)} className="flex items-center gap-2 flex-1 text-left">
          <ChevronDown size={14} className={`text-slate-500 transition-transform ${open ? "" : "-rotate-90"}`} />
          <span className="w-2 h-2 rounded-full" style={{ background: labelColor(label) }} />
          <span className="font-medium text-sm text-slate-800">{label}</span>
          <span className="text-xs text-slate-400">{data?.length ?? "…"}</span>
        </button>
        <Link to={`/browse?label=${label}&add=1`} className="text-slate-400 hover:text-blue-600">
          <Plus size={14} />
        </Link>
      </div>
      {open && (
        <ul className="pl-7 pr-2">
          {(data ?? []).slice(0, 50).map((e) => {
            const active = String(e.id) === currentId;
            return (
              <li key={e.id}>
                <Link
                  to={`/entity/${e.id}`}
                  className={`block px-2 py-1 rounded-md text-sm truncate ${
                    active ? "bg-blue-50 text-blue-700 font-medium" : "text-slate-600 hover:bg-slate-50"
                  }`}
                >
                  {String(e.props.name ?? e.id)}
                </Link>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
