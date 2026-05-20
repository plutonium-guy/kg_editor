import { runQuery } from "./client";
import type { NodeView, RelView } from "./types";

/** PropValue serde envelope is `{kind, value}`. Unwrap to a plain JS value. */
function unwrap(p: unknown): unknown {
  if (p === null || p === undefined) return p;
  if (typeof p !== "object") return p;
  const o = p as { kind?: string; value?: unknown };
  if (o.kind == null) return p;
  switch (o.kind) {
    case "null":   return null;
    case "bool":   return o.value;
    case "int":    return o.value;
    case "float":  return o.value;
    case "string": return o.value;
    case "list":   return Array.isArray(o.value) ? (o.value as unknown[]).map(unwrap) : [];
    case "map": {
      const out: Record<string, unknown> = {};
      for (const [k, v] of Object.entries((o.value as Record<string, unknown>) ?? {})) {
        out[k] = unwrap(v);
      }
      return out;
    }
    default: return o.value;
  }
}

export async function refreshGraph(limit = 50): Promise<{ nodes: NodeView[]; rels: RelView[] }> {
  const nodeRes = await runQuery(
    "MATCH (n) RETURN id(n) AS id, labels(n) AS labels, properties(n) AS properties LIMIT $lim",
    { lim: limit }
  );
  const relRes = await runQuery(
    "MATCH (s)-[r]->(e) RETURN id(r) AS id, type(r) AS type, id(s) AS start, id(e) AS end, properties(r) AS properties LIMIT $lim",
    { lim: limit * 2 }
  );

  const nodes: NodeView[] = nodeRes.rows.map((row) => {
    const id     = unwrap(row.id) as number;
    const labels = (unwrap(row.labels) as string[]) ?? [];
    const props  = (unwrap(row.properties) as Record<string, unknown>) ?? {};
    return { id, labels, props };
  });
  const rels: RelView[] = relRes.rows.map((row) => {
    const id    = unwrap(row.id) as number;
    const type  = unwrap(row.type) as string;
    const start = unwrap(row.start) as number;
    const end   = unwrap(row.end) as number;
    const props = (unwrap(row.properties) as Record<string, unknown>) ?? {};
    return { id, type, startKey: `s:${start}`, endKey: `s:${end}`, props };
  });
  return { nodes, rels };
}
