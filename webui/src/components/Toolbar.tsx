import { useStore } from "../state/store";

interface ToolbarProps {
  onCommit: () => void;
  onDiscard: () => void;
  onRefresh: () => void;
}

export default function Toolbar(p: ToolbarProps) {
  const { pendingCount, layout, setLayout } = useStore();
  return (
    <>
      <strong>kg editor</strong>
      <span style={{ marginLeft: 16 }}>
        <button onClick={p.onRefresh}>Refresh</button>{" "}
        <label>Layout
          <select value={layout} onChange={(e) => setLayout(e.target.value as "cose-bilkent" | "dagre" | "grid")}>
            <option value="cose-bilkent">cose-bilkent</option>
            <option value="dagre">dagre</option>
            <option value="grid">grid</option>
          </select>
        </label>{" "}
        <button onClick={p.onCommit} disabled={pendingCount === 0}>Commit ({pendingCount})</button>{" "}
        <button onClick={p.onDiscard} disabled={pendingCount === 0}>Discard</button>
      </span>
    </>
  );
}
