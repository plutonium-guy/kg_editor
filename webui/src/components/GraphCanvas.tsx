import { useEffect, useRef, useState } from "react";
import cytoscape, { type Core, type ElementDefinition, type EventObject } from "cytoscape";
// @ts-ignore
import coseBilkent from "cytoscape-cose-bilkent";
cytoscape.use(coseBilkent);

export type GraphNode = { id: number; label: string; name: string };
export type GraphEdge = {
  id: number;
  type: string;
  source: number;
  target: number;
  props?: Record<string, unknown>;
};

interface GraphCanvasProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  selectedId: number | null;
  onSelectNode: (id: number | null) => void;
  onLinkRequested: (fromId: number, toId: number) => void;
  onAddNodeRequested: (pos: { x: number; y: number }) => void;
  layout: string;
}

export default function GraphCanvas({
  nodes,
  edges,
  selectedId,
  onSelectNode,
  onLinkRequested,
  onAddNodeRequested,
  layout,
}: GraphCanvasProps) {
  const ref = useRef<HTMLDivElement>(null);
  const cyRef = useRef<Core | null>(null);
  const [tip, setTip] = useState<{ x: number; y: number; html: string } | null>(null);

  // Stable callback refs so the init effect can reference current handlers without re-running.
  const onSelectNodeRef = useRef(onSelectNode);
  const onLinkRequestedRef = useRef(onLinkRequested);
  const onAddNodeRequestedRef = useRef(onAddNodeRequested);
  useEffect(() => { onSelectNodeRef.current = onSelectNode; }, [onSelectNode]);
  useEffect(() => { onLinkRequestedRef.current = onLinkRequested; }, [onLinkRequested]);
  useEffect(() => { onAddNodeRequestedRef.current = onAddNodeRequested; }, [onAddNodeRequested]);

  // Init cytoscape once.
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
            width: 44,
            height: 44,
            "border-width": 2,
            "border-color": "rgba(0,0,0,0.15)",
            "transition-property": "background-color, border-color, border-width",
            "transition-duration": 150 as never,
          },
        },
        {
          selector: "node:selected",
          style: { "border-color": "#2563eb", "border-width": 4 },
        },
        {
          selector: "node.link-source",
          style: { "border-color": "#f59e0b", "border-width": 4 },
        },
        {
          selector: "edge",
          style: {
            label: "data(type)",
            "curve-style": "bezier",
            "target-arrow-shape": "triangle",
            "line-color": "#94a3b8",
            "target-arrow-color": "#94a3b8",
            "font-size": 9,
            color: "#64748b",
            "text-rotation": "autorotate" as never,
            "text-margin-y": -8,
            width: 1.5,
          },
        },
        {
          selector: "edge:selected",
          style: {
            "line-color": "#2563eb",
            "target-arrow-color": "#2563eb",
            width: 2.5,
          },
        },
      ] as unknown as cytoscape.StylesheetStyle[],
      layout: { name: "cose-bilkent", animate: false } as never,
      wheelSensitivity: 0.2,
    });

    // Click node → select
    cy.on("tap", "node", (e: EventObject) => {
      onSelectNodeRef.current(Number(e.target.data("entityId")));
    });
    // Click empty canvas → deselect
    cy.on("tap", (e: EventObject) => {
      if (e.target === cy) onSelectNodeRef.current(null);
    });

    // Right-click empty canvas → add-node request
    cy.on("cxttap", (e: EventObject) => {
      if (e.target === cy) {
        const r = ref.current?.getBoundingClientRect();
        onAddNodeRequestedRef.current({
          x: (e.renderedPosition?.x ?? 0) + (r?.left ?? 0),
          y: (e.renderedPosition?.y ?? 0) + (r?.top ?? 0),
        });
      }
    });

    // Drag-to-link: Shift/Meta/Ctrl + mousedown on a node starts a link gesture.
    // The source node is made un-grabbable so the canvas doesn't move it.
    // On mouseup over a different node the link is requested.
    let linkSourceId: number | null = null;

    cy.on("mousedown", "node", (e: EventObject) => {
      const oe = e.originalEvent as MouseEvent;
      if (oe.shiftKey || oe.metaKey || oe.ctrlKey) {
        linkSourceId = Number(e.target.data("entityId"));
        e.target.addClass("link-source");
        cy.userZoomingEnabled(false);
        cy.userPanningEnabled(false);
        e.target.ungrabify();
      }
    });

    cy.on("mouseup", "node", (e: EventObject) => {
      if (linkSourceId == null) return;
      const targetId = Number(e.target.data("entityId"));
      const src = linkSourceId;
      linkSourceId = null;
      cy.nodes().removeClass("link-source");
      cy.userZoomingEnabled(true);
      cy.userPanningEnabled(true);
      cy.nodes().grabify();
      if (src !== targetId) onLinkRequestedRef.current(src, targetId);
    });

    // Cancel link drag on empty canvas mouseup
    cy.on("mouseup", (e: EventObject) => {
      if (e.target === cy && linkSourceId != null) {
        linkSourceId = null;
        cy.nodes().removeClass("link-source");
        cy.userZoomingEnabled(true);
        cy.userPanningEnabled(true);
        cy.nodes().grabify();
      }
    });

    // Edge hover tooltip
    cy.on("mouseover", "edge", (e: EventObject) => {
      const props = e.target.data("props") as Record<string, unknown> | undefined;
      const type = e.target.data("type") as string;
      const html = `<strong>${escapeHtml(type)}</strong>${formatProps(props)}`;
      const pos = e.renderedPosition;
      const r = ref.current?.getBoundingClientRect();
      setTip({
        x: (pos?.x ?? 0) + (r?.left ?? 0),
        y: (pos?.y ?? 0) + (r?.top ?? 0) - 40,
        html,
      });
    });
    cy.on("mouseout", "edge", () => setTip(null));

    cyRef.current = cy;
    return () => {
      cy.destroy();
      cyRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Reconcile elements + layout when props change.
  useEffect(() => {
    const cy = cyRef.current;
    if (!cy) return;
    const elems: ElementDefinition[] = [];
    const nodeIds = new Set(nodes.map((n) => n.id));
    for (const n of nodes) {
      elems.push({
        group: "nodes",
        data: {
          id: `n${n.id}`,
          name: n.name,
          entityId: n.id,
          color: labelColor(n.label),
        },
      });
    }
    for (const e of edges) {
      if (!nodeIds.has(e.source) || !nodeIds.has(e.target)) continue;
      elems.push({
        group: "edges",
        data: {
          id: `e${e.id}`,
          source: `n${e.source}`,
          target: `n${e.target}`,
          type: e.type,
          props: e.props,
        },
      });
    }
    cy.json({ elements: elems });
    cy.layout({ name: layout, animate: false } as never).run();
    cy.fit(undefined, 50);
  }, [nodes, edges, layout]);

  // Reflect external selection
  useEffect(() => {
    const cy = cyRef.current;
    if (!cy) return;
    cy.nodes().unselect();
    if (selectedId != null) {
      cy.getElementById(`n${selectedId}`).select();
    }
  }, [selectedId]);

  return (
    <div className="relative w-full h-full">
      <div ref={ref} className="w-full h-full bg-slate-50" />
      {tip && (
        <div
          className="pointer-events-none absolute z-50 rounded-md bg-slate-900 text-white text-xs px-2 py-1 shadow-lg max-w-[240px]"
          style={{ left: tip.x, top: tip.y }}
          dangerouslySetInnerHTML={{ __html: tip.html }}
        />
      )}
    </div>
  );
}

export function labelColor(label: string): string {
  let h = 0;
  for (let i = 0; i < label.length; i++) h = (h * 31 + label.charCodeAt(i)) % 360;
  return `hsl(${h}, 55%, 48%)`;
}

function formatProps(p: Record<string, unknown> | undefined): string {
  if (!p) return "";
  const entries = Object.entries(p);
  if (entries.length === 0) return "";
  return (
    "<div class='mt-1 text-slate-300'>" +
    entries.map(([k, v]) => `${escapeHtml(k)}: ${escapeHtml(String(v))}`).join("<br/>") +
    "</div>"
  );
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c] as string)
  );
}
