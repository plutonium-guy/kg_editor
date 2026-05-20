import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useStore } from "../state/store";
import { describe, type PendingOp, type RefHandle } from "../kg/pending";
import * as api from "../kg/client";

export default function PendingPanel() {
  const { pending, remove, clear } = useStore();
  const qc = useQueryClient();
  const [running, setRunning] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  if (pending.length === 0) return null;

  const commit = async () => {
    setRunning(true);
    setErr(null);
    const tmpToId: Record<string, number> = {};
    try {
      for (const op of pending) {
        await applyOp(op, tmpToId);
      }
      clear();
      // Invalidate every query so lists/details refetch.
      await qc.invalidateQueries();
    } catch (e) {
      setErr(String(e));
    } finally {
      setRunning(false);
    }
  };

  return (
    <div style={{
      position: "fixed", left: 0, right: 0, bottom: 0,
      background: "#fef3c7", borderTop: "2px solid #d97706",
      padding: "12px 16px", boxShadow: "0 -2px 8px rgba(0,0,0,0.08)",
      maxHeight: "40vh", overflow: "auto",
      zIndex: 50,
    }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
        <strong style={{ color: "#92400e" }}>{pending.length} pending change{pending.length !== 1 ? "s" : ""}</strong>
        <div>
          <button onClick={() => clear()} disabled={running} style={btnSecondary}>Discard all</button>{" "}
          <button onClick={commit} disabled={running} style={btnPrimary}>
            {running ? "Committing…" : "Commit all"}
          </button>
        </div>
      </div>
      <ul style={{ margin: 0, paddingLeft: 18, color: "#78350f" }}>
        {pending.map((op, i) => (
          <li key={i} style={{ padding: "2px 0" }}>
            {describe(op)}{" "}
            <button onClick={() => remove(i)} disabled={running} style={btnRemove}>×</button>
          </li>
        ))}
      </ul>
      {err && <div style={{ color: "#dc2626", marginTop: 8, fontSize: 13 }}>{err}</div>}
    </div>
  );
}

async function applyOp(op: PendingOp, tmpToId: Record<string, number>): Promise<void> {
  switch (op.kind) {
    case "create_node": {
      const r = await api.createEntity({ label: op.label, props: op.props });
      tmpToId[op.tmpId] = r.id;
      break;
    }
    case "update_node": await api.updateEntity(op.id, op.set, op.unset); break;
    case "delete_node": await api.deleteEntity(op.id, op.cascade); break;
    case "create_link": {
      const sid = resolveRef(op.start, tmpToId);
      const eid = resolveRef(op.end, tmpToId);
      await api.createLink(op.type, sid, eid, op.props);
      break;
    }
    case "delete_link": await api.deleteLink(op.id); break;
  }
}

function resolveRef(r: RefHandle, tmpToId: Record<string, number>): number {
  if (r.kind === "id") return r.ref;
  const id = tmpToId[r.ref];
  if (id == null) throw new Error(`unresolved tmpId in link: ${r.ref}`);
  return id;
}

const btnPrimary: React.CSSProperties = {
  padding: "6px 14px", background: "#d97706", color: "#fff", border: 0,
  borderRadius: 4, cursor: "pointer", fontWeight: 600,
};
const btnSecondary: React.CSSProperties = {
  padding: "6px 14px", background: "transparent", color: "#92400e", border: "1px solid #d97706",
  borderRadius: 4, cursor: "pointer",
};
const btnRemove: React.CSSProperties = {
  background: "transparent", border: 0, color: "#92400e", cursor: "pointer", fontSize: 16, padding: 0, marginLeft: 4,
};
