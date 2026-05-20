import { useEffect, useRef } from "react";
import cytoscape, { type Core, type ElementDefinition } from "cytoscape";
// @ts-ignore
import coseBilkent from "cytoscape-cose-bilkent";
cytoscape.use(coseBilkent);
import { useNavigate } from "react-router-dom";

interface GraphCanvasProps {
  nodes: Array<{ id: number; label: string; name: string }>;
  edges: Array<{ id: number; type: string; source: number; target: number }>;
}

export default function GraphCanvas({ nodes, edges }: GraphCanvasProps) {
  const ref = useRef<HTMLDivElement>(null);
  const cyRef = useRef<Core | null>(null);
  const navigate = useNavigate();

  useEffect(() => {
    if (!ref.current) return;
    const cy = cytoscape({
      container: ref.current,
      style: [
        {
          selector: "node",
          style: {
            label: "data(name)",
            "background-color": "data(color)",
            "font-size": 11,
            color: "#fff",
            "text-valign": "center",
            "text-halign": "center",
            "text-outline-color": "data(color)",
            "text-outline-width": 1,
            width: 44, height: 44,
            "border-width": 2,
            "border-color": "rgba(0,0,0,0.15)",
          },
        },
        {
          selector: "edge",
          style: {
            label: "data(type)",
            "curve-style": "bezier",
            "target-arrow-shape": "triangle",
            "line-color": "#9ca3af",
            "target-arrow-color": "#9ca3af",
            "font-size": 9,
            color: "#6b7280",
            "text-rotation": "autorotate" as never,
            "text-margin-y": -8,
            width: 1.5,
          },
        },
        { selector: ":selected", style: { "background-color": "#f59e0b", "line-color": "#f59e0b", "target-arrow-color": "#f59e0b" } },
      ] as unknown as cytoscape.StylesheetStyle[],
      layout: { name: "cose-bilkent", animate: false } as never,
    });
    cy.on("tap", "node", (e) => navigate(`/entity/${e.target.data("entityId")}`));
    cyRef.current = cy;
    return () => { cy.destroy(); cyRef.current = null; };
  }, [navigate]);

  useEffect(() => {
    const cy = cyRef.current;
    if (!cy) return;
    const elems: ElementDefinition[] = [];
    for (const n of nodes) {
      elems.push({ group: "nodes", data: { id: `n${n.id}`, name: n.name, entityId: n.id, color: labelColor(n.label) } });
    }
    const nodeIds = new Set(nodes.map((n) => n.id));
    for (const e of edges) {
      if (!nodeIds.has(e.source) || !nodeIds.has(e.target)) continue;
      elems.push({ group: "edges", data: { id: `e${e.id}`, source: `n${e.source}`, target: `n${e.target}`, type: e.type } });
    }
    cy.json({ elements: elems });
    cy.layout({ name: "cose-bilkent", animate: false } as never).run();
    cy.fit(undefined, 40);
  }, [nodes, edges]);

  return <div ref={ref} style={{ width: "100%", height: "100%", background: "#f9fafb" }} />;
}

function labelColor(label: string): string {
  let h = 0;
  for (let i = 0; i < label.length; i++) h = (h * 31 + label.charCodeAt(i)) % 360;
  return `hsl(${h}, 55%, 48%)`;
}
