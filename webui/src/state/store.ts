import { create } from "zustand";
import type { NodeView, RelView, Selection } from "../kg/types";

export interface StoreState {
  nodes: Record<string, NodeView>;
  rels:  Record<string, RelView>;

  selection: Selection;
  pendingCount: number;
  cypherText: string;
  layout: "cose-bilkent" | "dagre" | "grid";

  setSelection: (s: Selection) => void;
  replaceGraph: (nodes: NodeView[], rels: RelView[]) => void;
  addPendingNode: (n: NodeView) => void;
  addPendingRel:  (r: RelView)  => void;
  removePending: (key: string) => void;
  setCypher: (s: string) => void;
  setLayout: (l: StoreState["layout"]) => void;
  resetPending: () => void;
}

export function nodeKey(n: { id?: number; localId?: number }): string {
  if (n.id != null) return `s:${n.id}`;
  if (n.localId != null) return `l:${n.localId}`;
  throw new Error("node has no id");
}
export function relKey(r: { id?: number; localId?: number }): string {
  if (r.id != null) return `s:${r.id}`;
  if (r.localId != null) return `l:${r.localId}`;
  throw new Error("rel has no id");
}

export const useStore = create<StoreState>((set) => ({
  nodes: {},
  rels: {},
  selection: { kind: "none" },
  pendingCount: 0,
  cypherText: "MATCH (n) RETURN n LIMIT 50",
  layout: "cose-bilkent",

  setSelection: (s) => set({ selection: s }),
  replaceGraph: (nodes, rels) => {
    const ns: Record<string, NodeView> = {};
    const rs: Record<string, RelView> = {};
    for (const n of nodes) ns[nodeKey(n)] = n;
    for (const r of rels)  rs[relKey(r)]  = r;
    set({ nodes: ns, rels: rs, pendingCount: 0, selection: { kind: "none" } });
  },
  addPendingNode: (n) => set((s) => ({
    nodes: { ...s.nodes, [nodeKey(n)]: { ...n, pending: true } },
    pendingCount: s.pendingCount + 1,
  })),
  addPendingRel: (r) => set((s) => ({
    rels: { ...s.rels, [relKey(r)]: { ...r, pending: true } },
    pendingCount: s.pendingCount + 1,
  })),
  removePending: (key) => set((s) => {
    const nodes = Object.fromEntries(Object.entries(s.nodes).filter(([k]) => k !== key));
    const rels  = Object.fromEntries(Object.entries(s.rels).filter(([k]) => k !== key));
    return { nodes, rels, pendingCount: Math.max(0, s.pendingCount - 1) };
  }),
  setCypher: (s) => set({ cypherText: s }),
  setLayout: (l) => set({ layout: l }),
  resetPending: () => set((s) => {
    const nodes: Record<string, NodeView> = {};
    const rels:  Record<string, RelView>  = {};
    for (const [k, v] of Object.entries(s.nodes)) if (!v.pending) nodes[k] = v;
    for (const [k, v] of Object.entries(s.rels))  if (!v.pending) rels[k]  = v;
    return { nodes, rels, pendingCount: 0 };
  }),
}));
