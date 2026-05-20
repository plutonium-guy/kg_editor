import init, * as wasm from "../../pkg/kg_core_wasm";
import type { EmitOutputJs, Props } from "./types";

let inited = false;
export async function loadWasm(): Promise<void> {
  if (!inited) {
    await init();
    inited = true;
  }
}

/** Convert a JS prop bag to an array of [name, PropValue-js] tuples. */
function toPropArray(p: Props): Array<[string, any]> {
  return Object.entries(p).map(([k, v]) => [k, toWasm(v)]);
}

/** Recursively convert a JS value to the JS-side PropValue shape. */
function toWasm(v: unknown): any {
  if (v === null || v === undefined) return wasm.prop_null();
  if (typeof v === "boolean") return wasm.prop_bool(v);
  if (typeof v === "number") {
    return Number.isInteger(v) ? wasm.prop_int(BigInt(v)) : wasm.prop_float(v);
  }
  if (typeof v === "bigint") return wasm.prop_int(v);
  if (typeof v === "string") return wasm.prop_string(v);
  if (Array.isArray(v)) return wasm.prop_list(v.map(toWasm));
  if (typeof v === "object") {
    const arr: Array<[string, any]> = [];
    for (const [k, x] of Object.entries(v as Record<string, unknown>)) arr.push([k, toWasm(x)]);
    return wasm.prop_map(arr);
  }
  throw new Error(`unsupported prop type for value ${String(v)}`);
}

export type NodeRef =
  | { kind: "server"; id: number }
  | { kind: "local"; id: number };

export class JsUow {
  private inner = new wasm.WasmUow();

  createNode(labels: string[], props: Props): number {
    return Number(this.inner.create_node(labels, toPropArray(props)));
  }
  mergeNode(labels: string[], key: Props, set: Props): number {
    return Number(this.inner.merge_node(labels, toPropArray(key), toPropArray(set)));
  }
  updateNode(id: number, sets: Props, unsets: string[]): void {
    this.inner.update_node(BigInt(id), toPropArray(sets), unsets);
  }
  deleteNode(id: number, detach: boolean = true): void {
    this.inner.delete_node(BigInt(id), detach);
  }
  createRel(start: NodeRef, end: NodeRef, ty: string, props: Props): number {
    return Number(this.inner.create_rel(start, end, ty, toPropArray(props)));
  }
  mergeRel(start: NodeRef, end: NodeRef, ty: string, key: Props, set: Props): number {
    return Number(this.inner.merge_rel(start, end, ty, toPropArray(key), toPropArray(set)));
  }
  updateRel(id: number, sets: Props, unsets: string[]): void {
    this.inner.update_rel(BigInt(id), toPropArray(sets), unsets);
  }
  deleteRel(id: number): void {
    this.inner.delete_rel(BigInt(id));
  }
  emit(): EmitOutputJs {
    return this.inner.emit() as EmitOutputJs;
  }
}
