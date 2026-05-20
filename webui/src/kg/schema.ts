export type FieldType =
  | { type: "string" } | { type: "int" } | { type: "float" } | { type: "bool" }
  | { type: "date" } | { type: "date_time" }
  | { type: "enum"; values: string[] }
  | { type: "ref";  label: string };

export type FieldSpec = FieldType & {
  name: string;
  required?: boolean;
  unique?: boolean;
  default?: unknown;
};

export interface NodeDef {
  description?: string;
  props: FieldSpec[];
  indexes?: string[][];
}
export interface RelDef {
  endpoints: [string, string][];
  cardinality: "many_to_many" | "one_to_many" | "one_to_one";
  props: FieldSpec[];
}
export interface SchemaFile {
  nodes: Record<string, NodeDef>;
  rels: Record<string, RelDef>;
}
