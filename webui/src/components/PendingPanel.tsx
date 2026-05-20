import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useStore } from "../state/store";
import { describe, type PendingOp, type RefHandle } from "../kg/pending";
import * as api from "../kg/client";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
import { X } from "lucide-react";

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
    <div className="fixed bottom-0 left-0 right-0 z-50 border-t border-amber-300 bg-amber-50 shadow-lg max-h-[40vh] overflow-auto">
      <div className="px-6 py-3 flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <Badge tone="amber">{pending.length} pending</Badge>
          <span className="text-sm text-amber-900">unsaved changes</span>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={() => clear()} disabled={running}>Discard all</Button>
          <Button variant="primary" size="sm" onClick={commit} disabled={running}>
            {running ? "Committing…" : "Commit all"}
          </Button>
        </div>
      </div>
      <ul className="px-6 pb-3 space-y-1">
        {pending.map((op, i) => (
          <li key={i} className="flex items-center justify-between text-sm text-amber-900 bg-amber-100/60 rounded px-3 py-1.5">
            <span>{describe(op)}</span>
            <button onClick={() => remove(i)} disabled={running} className="text-amber-700 hover:text-amber-900"><X size={14} /></button>
          </li>
        ))}
      </ul>
      {err && <div className="px-6 pb-3 text-sm text-red-700">{err}</div>}
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
