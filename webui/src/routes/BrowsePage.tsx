import { useSearchParams } from "react-router-dom";
import { useState } from "react";
import EntityListSidebar from "../components/EntityListSidebar";
import EntityList from "../components/EntityList";
import EntityForm from "../components/EntityForm";
import { useSchema } from "../hooks/useSchema";
import { useStore } from "../state/store";

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
    <div style={{ display: "grid", gridTemplateColumns: "240px 1fr", height: "100%" }}>
      <EntityListSidebar />
      <section style={{ padding: 16, overflow: "auto" }}>
        <h2 style={{ marginTop: 0, color: "#1f2937" }}>{label ? `${label} entities` : "All entities"}</h2>
        <EntityList label={label} />
        {showAdd && schema && label && schema.nodes[label] && (
          <aside style={{
            position: "fixed", top: 48, right: 0, width: 420, height: "100vh",
            background: "#fff", borderLeft: "1px solid #d1d5db",
            padding: 16, overflow: "auto", boxShadow: "-4px 0 12px rgba(0,0,0,0.08)", zIndex: 40,
          }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
              <h3 style={{ margin: 0 }}>New {label}</h3>
              <button onClick={closeAdd} style={{ background: "transparent", border: 0, fontSize: 20, cursor: "pointer" }}>×</button>
            </div>
            <EntityForm
              fields={schema.nodes[label].props}
              submitLabel="Stage"
              onSubmit={(values) => {
                append({
                  kind: "create_node",
                  tmpId: `t${tmpCounter}`,
                  label,
                  props: values,
                });
                setTmpCounter(tmpCounter + 1);
                closeAdd();
              }}
            />
          </aside>
        )}
      </section>
    </div>
  );
}
