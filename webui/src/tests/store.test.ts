import { describe, expect, it, beforeEach } from "vitest";
import { useStore } from "../state/store";

describe("pending store", () => {
  beforeEach(() => useStore.setState({ pending: [] }));

  it("append adds an op", () => {
    useStore.getState().append({ kind: "create_node", tmpId: "t1", label: "Person", props: { name: "Alice" } });
    expect(useStore.getState().pending).toHaveLength(1);
  });

  it("remove by index drops only that op", () => {
    useStore.getState().append({ kind: "create_node", tmpId: "t1", label: "Person", props: {} });
    useStore.getState().append({ kind: "create_node", tmpId: "t2", label: "Person", props: {} });
    useStore.getState().remove(0);
    expect(useStore.getState().pending).toHaveLength(1);
    const remaining = useStore.getState().pending[0] as { tmpId?: string };
    expect(remaining.tmpId).toBe("t2");
  });

  it("clear empties", () => {
    useStore.getState().append({ kind: "create_node", tmpId: "t1", label: "Person", props: {} });
    useStore.getState().clear();
    expect(useStore.getState().pending).toEqual([]);
  });
});
