export type Props = Record<string, unknown>;

export type RefHandle =
  | { kind: "id"; ref: number }
  | { kind: "tmp"; ref: string };

export type PendingOp =
  | { kind: "create_node"; tmpId: string; label: string; props: Props }
  | { kind: "update_node"; id: number; set: Props; unset: string[] }
  | { kind: "delete_node"; id: number; cascade: boolean }
  | { kind: "create_link"; tmpId: string; type: string; start: RefHandle; end: RefHandle; props: Props }
  | { kind: "delete_link"; id: number };

export function describe(op: PendingOp): string {
  switch (op.kind) {
    case "create_node": {
      const name = (op.props as { name?: unknown }).name;
      return `Create ${op.label}${name != null ? ` "${name}"` : ""}`;
    }
    case "update_node": return `Update node #${op.id}`;
    case "delete_node": return `Delete node #${op.id}`;
    case "create_link": return `Link ${op.type} ${refSummary(op.start)} → ${refSummary(op.end)}`;
    case "delete_link": return `Delete link #${op.id}`;
  }
}

function refSummary(r: RefHandle): string {
  return r.kind === "id" ? `#${r.ref}` : `(new) ${r.ref}`;
}
