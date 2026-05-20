import { useStore } from "../state/store";

interface QueryBoxProps {
  onRun: () => void;
}

export default function QueryBox(p: QueryBoxProps) {
  const { cypherText, setCypher } = useStore();
  return (
    <div>
      <textarea
        value={cypherText}
        onChange={(e) => setCypher(e.target.value)}
        rows={2}
        style={{ width: "calc(100% - 80px)", fontFamily: "monospace" }}
        placeholder="MATCH (n) RETURN n LIMIT 50"
      />
      <button onClick={p.onRun} style={{ marginLeft: 8 }}>Run</button>
    </div>
  );
}
