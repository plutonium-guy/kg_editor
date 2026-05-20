import { describe, expect, it, beforeEach } from "vitest";
import { useStore } from "../state/store";

describe("store", () => {
  beforeEach(() => {
    useStore.setState({
      nodes: {}, rels: {}, selection: { kind: "none" },
      pendingCount: 0, cypherText: "", layout: "cose-bilkent",
    });
  });

  it("addPendingNode increments pendingCount and tags node", () => {
    useStore.getState().addPendingNode({ localId: 1, labels: ["P"], props: {} });
    expect(useStore.getState().pendingCount).toBe(1);
    expect(useStore.getState().nodes["l:1"]?.pending).toBe(true);
  });

  it("replaceGraph wipes pending and resets selection", () => {
    useStore.getState().addPendingNode({ localId: 1, labels: ["P"], props: {} });
    useStore.getState().replaceGraph([{ id: 99, labels: ["X"], props: {} }], []);
    const s = useStore.getState();
    expect(s.pendingCount).toBe(0);
    expect(s.nodes["s:99"]).toBeTruthy();
    expect(s.nodes["l:1"]).toBeUndefined();
    expect(s.selection.kind).toBe("none");
  });

  it("resetPending drops pending nodes only", () => {
    useStore.getState().replaceGraph([{ id: 1, labels: ["A"], props: {} }], []);
    useStore.getState().addPendingNode({ localId: 7, labels: ["B"], props: {} });
    useStore.getState().resetPending();
    const s = useStore.getState();
    expect(s.pendingCount).toBe(0);
    expect(s.nodes["s:1"]).toBeTruthy();
    expect(s.nodes["l:7"]).toBeUndefined();
  });
});
