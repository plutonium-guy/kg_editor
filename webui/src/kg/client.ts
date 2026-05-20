const BASE = (import.meta.env.VITE_KG_SERVER_BASE as string | undefined) ?? "http://localhost:9000";

async function asJson<T>(r: Response): Promise<T> {
  if (!r.ok) {
    const t = await r.text();
    let body: unknown = t;
    try { body = JSON.parse(t); } catch {}
    throw new Error(`server ${r.status}: ${JSON.stringify(body)}`);
  }
  return r.json() as Promise<T>;
}

export interface EntityCreateBody { label: string; props: Record<string, unknown>; }
export interface EntityResponse  { id: number; label: string; props: Record<string, unknown>; }
export interface EntityDetail {
  id: number;
  labels: string[];
  props: Record<string, unknown>;
  out_rels: Array<{ id: number; type: string; target_id: number; target_labels: string[]; props: Record<string, unknown> }>;
  in_rels:  Array<{ id: number; type: string; source_id: number; source_labels: string[]; props: Record<string, unknown> }>;
}

export async function listEntities(label?: string, q?: string, limit = 50): Promise<EntityDetail[]> {
  const u = new URL(`${BASE}/entities`);
  if (label) u.searchParams.set("label", label);
  if (q)     u.searchParams.set("q", q);
  u.searchParams.set("limit", String(limit));
  return asJson(await fetch(u));
}

export async function getEntity(id: number): Promise<EntityDetail> {
  return asJson(await fetch(`${BASE}/entities/${id}`));
}

export async function createEntity(body: EntityCreateBody): Promise<EntityResponse> {
  return asJson(await fetch(`${BASE}/entities`, {
    method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body),
  }));
}

export async function updateEntity(id: number, set: Record<string, unknown>, unset: string[] = []): Promise<void> {
  await asJson(await fetch(`${BASE}/entities/${id}`, {
    method: "PUT", headers: { "content-type": "application/json" }, body: JSON.stringify({ set, unset }),
  }));
}

export async function deleteEntity(id: number, cascade = true): Promise<void> {
  await asJson(await fetch(`${BASE}/entities/${id}?cascade=${cascade}`, { method: "DELETE" }));
}

export async function createLink(type: string, start_id: number, end_id: number, props: Record<string, unknown> = {}): Promise<{ id: number }> {
  return asJson(await fetch(`${BASE}/links`, {
    method: "POST", headers: { "content-type": "application/json" },
    body: JSON.stringify({ type, start_id, end_id, props }),
  }));
}

export async function deleteLink(id: number): Promise<void> {
  await asJson(await fetch(`${BASE}/links/${id}`, { method: "DELETE" }));
}

export async function search(q: string, limit = 20): Promise<EntityDetail[]> {
  const u = new URL(`${BASE}/search`);
  u.searchParams.set("q", q);
  u.searchParams.set("limit", String(limit));
  return asJson(await fetch(u));
}
