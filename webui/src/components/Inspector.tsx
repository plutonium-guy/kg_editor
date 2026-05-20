import { useMemo, useState } from "react";
import { useStore } from "../state/store";

interface InspectorProps {
  onStageUpdateNode: (id: number, sets: Record<string, unknown>, unsets: string[]) => void;
  onStageUpdateRel:  (id: number, sets: Record<string, unknown>, unsets: string[]) => void;
  onStageDeleteNode: (id: number) => void;
  onStageDeleteRel:  (id: number) => void;
}

export default function Inspector(p: InspectorProps) {
  const { selection, nodes, rels } = useStore();
  const target = useMemo(() => {
    if (selection.kind === "node") return { kind: "node" as const, item: nodes[selection.key] };
    if (selection.kind === "rel")  return { kind: "rel"  as const, item: rels[selection.key] };
    return null;
  }, [selection, nodes, rels]);

  const [edit, setEdit] = useState(false);
  const [drafts, setDrafts] = useState<Record<string, string>>({});

  if (!target || !target.item) return <div>Select a node or relationship.</div>;

  const item = target.item;
  const itemProps = item.props ?? {};

  if (!edit) {
    return (
      <div>
        <h3>{target.kind === "node"
          ? `Node ${(item as { labels?: string[] }).labels?.join(":") ?? "?"}`
          : `Rel ${(item as { type?: string }).type ?? "?"}`}</h3>
        <dl>
          {Object.entries(itemProps).map(([k, v]) => (
            <div key={k}><dt><strong>{k}</strong></dt><dd>{JSON.stringify(v)}</dd></div>
          ))}
        </dl>
        <button onClick={() => {
          setDrafts(Object.fromEntries(Object.entries(itemProps).map(([k, v]) => [k, String(v ?? "")])));
          setEdit(true);
        }}>Edit</button>{" "}
        <button onClick={() => {
          if (item.id != null) {
            if (target.kind === "node") p.onStageDeleteNode(item.id);
            else p.onStageDeleteRel(item.id);
          }
        }}>Delete</button>
      </div>
    );
  }

  return (
    <div>
      <h3>Editing {target.kind === "node"
        ? (item as { labels?: string[] }).labels?.join(":")
        : (item as { type?: string }).type}</h3>
      {Object.entries(drafts).map(([k, v]) => (
        <div key={k}>
          <label>{k}<input value={v} onChange={(e) => setDrafts({ ...drafts, [k]: e.target.value })} /></label>
        </div>
      ))}
      <button onClick={() => {
        if (item.id == null) { setEdit(false); return; }
        const sets: Record<string, unknown> = {};
        const unsets: string[] = [];
        for (const [k, v] of Object.entries(drafts)) {
          if (v === "") unsets.push(k);
          else sets[k] = isNaN(Number(v)) ? v : Number(v);
        }
        if (target.kind === "node") p.onStageUpdateNode(item.id, sets, unsets);
        else p.onStageUpdateRel(item.id, sets, unsets);
        setEdit(false);
      }}>Save</button>{" "}
      <button onClick={() => setEdit(false)}>Cancel</button>
    </div>
  );
}
