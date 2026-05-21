import { useState } from "react";
import { Card, CardHeader, CardTitle, CardContent } from "./ui/card";
import { Button } from "./ui/button";
import { Select } from "./ui/input";
import { Plus, Save, Trash2 } from "lucide-react";
import FieldSpecEditor from "./FieldSpecEditor";
import type { RelDef, FieldSpec, SchemaFile } from "../kg/schema";

interface Props {
  type: string;
  initial: RelDef;
  schema: SchemaFile;
  onSave: (def: RelDef) => void;
  onDelete: () => void;
}

export default function RelTypeEditor({ type, initial, schema, onSave, onDelete }: Props) {
  const [cardinality, setCardinality] = useState<RelDef["cardinality"]>(initial.cardinality ?? "many_to_many");
  const [endpoints, setEndpoints] = useState<[string, string][]>(initial.endpoints ?? []);
  const [props, setProps] = useState<FieldSpec[]>(initial.props ?? []);

  const labels = Object.keys(schema.nodes);

  return (
    <Card>
      <CardHeader>
        <CardTitle>{type}</CardTitle>
        <div className="flex gap-2">
          <Button variant="primary" size="sm" onClick={() => onSave({ cardinality, endpoints, props })}>
            <Save size={14} /> Save
          </Button>
          <Button variant="danger" size="sm" onClick={onDelete}>
            <Trash2 size={14} /> Delete type
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-5">
        <div>
          <label className="block text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1">Cardinality</label>
          <Select value={cardinality} onChange={(e) => setCardinality(e.target.value as RelDef["cardinality"])}>
            <option value="many_to_many">many_to_many</option>
            <option value="one_to_many">one_to_many</option>
            <option value="one_to_one">one_to_one</option>
          </Select>
        </div>

        <div>
          <div className="flex items-center justify-between mb-2">
            <label className="text-xs font-semibold text-slate-500 uppercase tracking-wider">Allowed endpoints</label>
            <Button size="sm" variant="outline" onClick={() => setEndpoints([...endpoints, [labels[0] ?? "", labels[0] ?? ""]])}>
              <Plus size={14} /> Endpoint pair
            </Button>
          </div>
          <div className="space-y-2">
            {endpoints.length === 0 && <p className="text-sm text-slate-400">Any labels allowed (no constraint).</p>}
            {endpoints.map(([from, to], i) => (
              <div key={i} className="flex gap-2 items-center">
                <Select
                  value={from}
                  onChange={(e) => setEndpoints(endpoints.map((p, j) => j === i ? [e.target.value, p[1]] as [string, string] : p))}
                  className="flex-1"
                >
                  {labels.map((l) => <option key={l} value={l}>{l}</option>)}
                </Select>
                <span className="text-slate-400">→</span>
                <Select
                  value={to}
                  onChange={(e) => setEndpoints(endpoints.map((p, j) => j === i ? [p[0], e.target.value] as [string, string] : p))}
                  className="flex-1"
                >
                  {labels.map((l) => <option key={l} value={l}>{l}</option>)}
                </Select>
                <Button variant="ghost" size="icon" onClick={() => setEndpoints(endpoints.filter((_, j) => j !== i))}>
                  <Trash2 size={16} />
                </Button>
              </div>
            ))}
          </div>
        </div>

        <div>
          <div className="flex items-center justify-between mb-2">
            <label className="text-xs font-semibold text-slate-500 uppercase tracking-wider">Properties</label>
            <Button size="sm" variant="outline" onClick={() => setProps([...props, { name: "", type: "string" } as FieldSpec])}>
              <Plus size={14} /> Property
            </Button>
          </div>
          <div className="space-y-2">
            {props.length === 0 && <p className="text-sm text-slate-400">No properties.</p>}
            {props.map((p, i) => (
              <FieldSpecEditor
                key={i}
                spec={p}
                onChange={(next) => setProps(props.map((x, j) => (j === i ? next : x)))}
                onRemove={() => setProps(props.filter((_, j) => j !== i))}
                refLabels={labels}
              />
            ))}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
