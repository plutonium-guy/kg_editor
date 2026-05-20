import { useEffect, useRef } from "react";
import cytoscape, { type Core, type ElementDefinition } from "cytoscape";
// @ts-ignore
import coseBilkent from "cytoscape-cose-bilkent";
// @ts-ignore
import dagre from "cytoscape-dagre";
import { useStore } from "../state/store";

cytoscape.use(coseBilkent);
cytoscape.use(dagre);

export default function Canvas() {
  const ref = useRef<HTMLDivElement>(null);
  const cyRef = useRef<Core | null>(null);
  const { nodes, rels, layout, setSelection } = useStore();

  useEffect(() => {
    if (!ref.current) return;
    const cy = cytoscape({
      container: ref.current,
      style: [
        {
          selector: "node",
          style: {
            label: "data(label)",
            "background-color": "#3b82f6",
            color: "#fff",
            "text-valign": "center",
            "text-halign": "center",
            "font-size": 10,
            width: 36,
            height: 36,
          },
        },
        {
          selector: "node.pending",
          style: {
            "border-width": 2,
            "border-style": "dashed",
            "border-color": "#9ca3af",
            opacity: 0.6,
          },
        },
        {
          selector: "edge",
          style: {
            label: "data(type)",
            "curve-style": "bezier",
            "target-arrow-shape": "triangle",
            "line-color": "#6b7280",
            "target-arrow-color": "#6b7280",
            "font-size": 9,
          },
        },
        {
          selector: "edge.pending",
          style: {
            "line-style": "dashed",
            opacity: 0.6,
          },
        },
        {
          selector: ":selected",
          style: {
            "background-color": "#f59e0b",
            "line-color": "#f59e0b",
            "target-arrow-color": "#f59e0b",
          },
        },
      ] as unknown as cytoscape.StylesheetStyle[],
      layout: { name: layout, animate: false },
    });
    cy.on("tap", "node", (e) => setSelection({ kind: "node", key: e.target.id() }));
    cy.on("tap", "edge", (e) => setSelection({ kind: "rel",  key: e.target.id() }));
    cy.on("tap", (e) => { if (e.target === cy) setSelection({ kind: "none" }); });

    // Right-click empty canvas → fire custom event for App to handle.
    cy.on("cxttap", (e) => {
      if (e.target === cy) {
        const pos = e.position;
        document.dispatchEvent(new CustomEvent("kg-add-node", { detail: { x: pos.x, y: pos.y } }));
      }
    });
    // Shift+drag from one node to another → fire custom event for App to handle.
    let dragStart: string | null = null;
    cy.on("mousedown", "node", (e) => { if ((e.originalEvent as MouseEvent).shiftKey) dragStart = e.target.id(); });
    cy.on("mouseup", "node", (e) => {
      if (dragStart && dragStart !== e.target.id()) {
        document.dispatchEvent(new CustomEvent("kg-add-rel", { detail: { from: dragStart, to: e.target.id() } }));
      }
      dragStart = null;
    });

    cyRef.current = cy;
    return () => { cy.destroy(); cyRef.current = null; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const cy = cyRef.current;
    if (!cy) return;
    const elems: ElementDefinition[] = [];
    for (const [k, n] of Object.entries(nodes)) {
      elems.push({
        group: "nodes",
        data: { id: k, label: n.labels[0] ?? "?" },
        classes: n.pending ? "pending" : undefined,
      });
    }
    for (const [k, r] of Object.entries(rels)) {
      elems.push({
        group: "edges",
        data: { id: k, source: r.startKey, target: r.endKey, type: r.type },
        classes: r.pending ? "pending" : undefined,
      });
    }
    cy.json({ elements: elems });
    cy.layout({ name: layout, animate: false }).run();
  }, [nodes, rels, layout]);

  return <div ref={ref} style={{ width: "100%", height: "100%" }} />;
}
