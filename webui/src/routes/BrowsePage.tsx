import { useSearchParams } from "react-router-dom";
import { useState } from "react";
import EntityListSidebar from "../components/EntityListSidebar";
import EntityList from "../components/EntityList";
import EntityForm from "../components/EntityForm";
import { useSchema } from "../hooks/useSchema";
import { useStore } from "../state/store";
import { Button } from "../components/ui/button";
import { X } from "lucide-react";

export default function BrowsePage() {
  const [params, setParams] = useSearchParams();
  const label = params.get("label") ?? undefined;
  const showAdd = params.get("add") === "1";
  const { data: schema } = useSchema();
  const append = useStore((s) => s.append);
  const [tmpCounter, setTmpCounter] = useState(0);

  const closeAdd = () => {
    const p = new URLSearchParams(params);
    p.delete("add");
    setParams(p);
  };

  return (
    <div className="grid grid-cols-[16rem_1fr] h-full">
      <EntityListSidebar />
      <section className="p-6 overflow-auto">
        <header className="flex items-center justify-between mb-4">
          <h1 className="text-2xl font-bold text-slate-900">{label ? `${label}` : "All entities"}</h1>
          {label && (
            <Button variant="primary" size="sm" onClick={() => {
              const p = new URLSearchParams(params);
              p.set("add", "1");
              setParams(p);
            }}>+ New {label}</Button>
          )}
        </header>
        <EntityList label={label} />
        {showAdd && schema && label && schema.nodes[label] && (
          <aside className="fixed top-14 right-0 w-[420px] h-[calc(100vh-3.5rem)] bg-white border-l border-slate-200 shadow-xl z-40 overflow-auto">
            <div className="flex items-center justify-between px-5 py-3 border-b border-slate-200">
              <h2 className="font-semibold text-slate-900">New {label}</h2>
              <button onClick={closeAdd} className="text-slate-400 hover:text-slate-600"><X size={18} /></button>
            </div>
            <div className="p-5">
              <EntityForm
                fields={schema.nodes[label].props}
                submitLabel="Stage"
                onSubmit={(values) => {
                  append({ kind: "create_node", tmpId: `t${tmpCounter}`, label, props: values });
                  setTmpCounter(tmpCounter + 1);
                  closeAdd();
                }}
              />
            </div>
          </aside>
        )}
      </section>
    </div>
  );
}
