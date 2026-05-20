import { create } from "zustand";
import type { PendingOp } from "../kg/pending";

export interface Store {
  pending: PendingOp[];
  append(op: PendingOp): void;
  remove(idx: number): void;
  clear(): void;
}

export const useStore = create<Store>((set) => ({
  pending: [],
  append: (op) => set((s) => ({ pending: [...s.pending, op] })),
  remove: (idx) => set((s) => ({ pending: s.pending.filter((_, i) => i !== idx) })),
  clear: () => set({ pending: [] }),
}));
