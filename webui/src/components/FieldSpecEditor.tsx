import { Input, Select } from "./ui/input";
import { Button } from "./ui/button";
import { X } from "lucide-react";
import type { FieldSpec } from "../kg/schema";

interface Props {
  spec: FieldSpec;
  onChange: (next: FieldSpec) => void;
  onRemove: () => void;
  refLabels: string[];
}

export default function FieldSpecEditor({ spec, onChange, onRemove, refLabels }: Props) {
  const set = (patch: Partial<FieldSpec>) => onChange({ ...spec, ...patch } as FieldSpec);
  return (
    <div className="rounded-md border border-slate-200 bg-white p-3 space-y-2">
      <div className="flex gap-2">
        <Input
          value={spec.name}
          onChange={(e) => set({ name: e.target.value })}
          placeholder="field name"
          className="flex-1"
        />
        <Select
          value={spec.type}
          onChange={(e) => {
            const t = e.target.value as FieldSpec["type"];
            // Clear type-specific fields when type changes
            const next: FieldSpec = { name: spec.name, required: spec.required, unique: spec.unique, type: t } as FieldSpec;
            if (t === "enum") (next as any).values = (spec as any).values ?? [];
            if (t === "ref")  (next as any).label  = (spec as any).label  ?? "";
            onChange(next);
          }}
          className="w-32"
        >
          <option value="string">string</option>
          <option value="int">int</option>
          <option value="float">float</option>
          <option value="bool">bool</option>
          <option value="date">date</option>
          <option value="date_time">date_time</option>
          <option value="enum">enum</option>
          <option value="ref">ref</option>
        </Select>
        <Button variant="ghost" size="icon" onClick={onRemove}><X size={16} /></Button>
      </div>

      {spec.type === "enum" && (
        <Input
          value={((spec as any).values ?? []).join(", ")}
          onChange={(e) => set({ values: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) } as never)}
          placeholder="comma-separated values: low, medium, high"
        />
      )}
      {spec.type === "ref" && (
        <Select
          value={(spec as any).label ?? ""}
          onChange={(e) => set({ label: e.target.value } as never)}
        >
          <option value="">(pick referenced label)</option>
          {refLabels.map((l) => <option key={l} value={l}>{l}</option>)}
        </Select>
      )}

      <div className="flex gap-4 text-xs text-slate-700">
        <label className="flex items-center gap-1">
          <input type="checkbox" checked={!!spec.required} onChange={(e) => set({ required: e.target.checked })} />
          required
        </label>
        <label className="flex items-center gap-1">
          <input type="checkbox" checked={!!spec.unique} onChange={(e) => set({ unique: e.target.checked })} />
          unique
        </label>
      </div>
    </div>
  );
}
