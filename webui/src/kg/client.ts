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

export interface QueryResponse {
  rows: Record<string, unknown>[];
}

export async function runQuery(cypher: string, params: Record<string, unknown> = {}): Promise<QueryResponse> {
  const r = await fetch(`${BASE}/query`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ cypher, params }),
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
