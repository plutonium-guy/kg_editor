import { useState } from "react";
import { Card, CardHeader, CardTitle, CardContent } from "./ui/card";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Plus, Save, Trash2 } from "lucide-react";
import FieldSpecEditor from "./FieldSpecEditor";
import type { NodeDef, FieldSpec, SchemaFile } from "../kg/schema";

interface Props {
  label: string;
  initial: NodeDef;
  schema: SchemaFile;
  onSave: (def: NodeDef) => void;
  onDelete: () => void;
}

export default function NodeLabelEditor({ label, initial, schema, onSave, onDelete }: Props) {
  const [description, setDescription] = useState(initial.description ?? "");
  const [props, setProps] = useState<FieldSpec[]>(initial.props ?? []);
  const [indexes, setIndexes] = useState<string[][]>(initial.indexes ?? []);

  return (
    <Card>
      <CardHeader>
        <CardTitle>{label}</CardTitle>
        <div className="flex gap-2">
          <Button variant="primary" size="sm" onClick={() => onSave({ description, props, indexes })}>
            <Save size={14} /> Save
          </Button>
          <Button variant="danger" size="sm" onClick={onDelete}>
            <Trash2 size={14} /> Delete label
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-5">
        <div>
          <label className="block text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1">Description</label>
          <Input value={description} onChange={(e) => setDescription(e.target.value)} placeholder={`What is a ${label}?`} />
        </div>

        <div>
          <div className="flex items-center justify-between mb-2">
            <label className="text-xs font-semibold text-slate-500 uppercase tracking-wider">Properties</label>
            <Button
              size="sm" variant="outline"
              onClick={() => setProps([...props, { name: "", type: "string" } as FieldSpec])}
            ><Plus size={14} /> Property</Button>
          </div>
          <div className="space-y-2">
            {props.length === 0 && <p className="text-sm text-slate-400">No properties yet.</p>}
            {props.map((p, i) => (
              <FieldSpecEditor
                key={i}
                spec={p}
                onChange={(next) => setProps(props.map((x, j) => (j === i ? next : x)))}
                onRemove={() => setProps(props.filter((_, j) => j !== i))}
                refLabels={Object.keys(schema.nodes)}
              />
            ))}
          </div>
        </div>

        <div>
          <div className="flex items-center justify-between mb-2">
            <label className="text-xs font-semibold text-slate-500 uppercase tracking-wider">Indexes</label>
            <Button
              size="sm" variant="outline"
              onClick={() => setIndexes([...indexes, []])}
            ><Plus size={14} /> Index</Button>
          </div>
          <div className="space-y-2">
            {indexes.length === 0 && <p className="text-sm text-slate-400">No indexes.</p>}
            {indexes.map((cols, i) => (
              <div key={i} className="flex gap-2">
                <Input
                  value={cols.join(", ")}
                  onChange={(e) => {
                    const next = e.target.value.split(",").map((s) => s.trim()).filter(Boolean);
                    setIndexes(indexes.map((c, j) => (j === i ? next : c)));
                  }}
                  placeholder="comma-separated prop names"
                />
                <Button variant="ghost" size="icon" onClick={() => setIndexes(indexes.filter((_, j) => j !== i))}>
                  <Trash2 size={16} />
                </Button>
              </div>
            ))}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
