export type { Props } from "./pending";

export type NodeId = number;
export type LocalId = number;
export type RelId = number;

export interface EmitOutputJs {
  ddl: Array<{ cypher: string; params: Props }>;
  data: { cypher: string; params: Props } | null;
}

type Props = Record<string, unknown>;
