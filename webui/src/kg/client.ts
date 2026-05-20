import type { EmitOutputJs } from "./types";

const BASE = (import.meta.env.VITE_KG_SERVER_BASE as string | undefined) ?? "http://localhost:9000";

async function asJson<T>(r: Response): Promise<T> {
  if (!r.ok) {
    const t = await r.text();
    let body: unknown = t;
    try { body = JSON.parse(t); } catch { /* keep raw */ }
    throw new Error(`server ${r.status}: ${JSON.stringify(body)}`);
  }
  return r.json() as Promise<T>;
}

/** Wrap a plain JS value in a PropValue serde envelope so kg-server can deserialize it. */
function envelope(v: unknown): Record<string, unknown> {
  if (v === null || v === undefined) return { kind: "null", value: null };
  if (typeof v === "boolean") return { kind: "bool", value: v };
  if (typeof v === "number") {
    return Number.isInteger(v) ? { kind: "int", value: v } : { kind: "float", value: v };
  }
  if (typeof v === "string") return { kind: "string", value: v };
  if (Array.isArray(v)) return { kind: "list", value: v.map(envelope) };
  if (typeof v === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, x] of Object.entries(v as Record<string, unknown>)) out[k] = envelope(x);
    return { kind: "map", value: out };
  }
  throw new Error(`unsupported param type ${typeof v}`);
}

function envelopeParams(p: Record<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(p)) out[k] = envelope(v);
  return out;
}

export interface QueryResponse {
  rows: Record<string, unknown>[];
}

export async function runQuery(cypher: string, params: Record<string, unknown> = {}): Promise<QueryResponse> {
  const r = await fetch(`${BASE}/query`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ cypher, params: envelopeParams(params) }),
  });
  return asJson<QueryResponse>(r);
}

export async function commit(out: EmitOutputJs): Promise<void> {
  const r = await fetch(`${BASE}/commit`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(out),
  });
  await asJson<{ ok: boolean }>(r);
}

export async function health(): Promise<{ status: string; neo4j: string }> {
  const r = await fetch(`${BASE}/health`);
  return asJson(r);
}
