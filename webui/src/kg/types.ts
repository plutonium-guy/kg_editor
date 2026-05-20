export type NodeId = number;
export type LocalId = number;
export type RelId = number;

export type Props = Record<string, unknown>;

export interface NodeView {
  id?: NodeId;
  localId?: LocalId;
  labels: string[];
  props: Props;
  pending?: boolean;
}

export interface RelView {
  id?: RelId;
  localId?: LocalId;
  type: string;
  startKey: string;
  endKey: string;
  props: Props;
  pending?: boolean;
}

export interface EmitOutputJs {
  ddl: Array<{ cypher: string; params: Props }>;
  data: { cypher: string; params: Props } | null;
}

export type Selection =
  | { kind: "node"; key: string }
  | { kind: "rel"; key: string }
  | { kind: "none" };
