import { useMemo } from "react";
import { useForm, type FieldErrors } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import type { FieldSpec } from "../kg/schema";

interface Props {
  fields: FieldSpec[];
  initial?: Record<string, unknown>;
  onSubmit: (values: Record<string, unknown>) => void;
  submitLabel?: string;
}

export default function EntityForm({ fields, initial, onSubmit, submitLabel = "Save" }: Props) {
  const schema = useMemo(() => buildZodSchema(fields), [fields]);
  const { register, handleSubmit, formState: { errors } } = useForm<Record<string, unknown>>({
    resolver: zodResolver(schema),
    defaultValues: initial as never,
  });

  return (
    <form onSubmit={handleSubmit((v) => onSubmit(coerceForSubmit(v, fields)))} style={{ display: "grid", gap: 12 }}>
      {fields.map((f) => (
        <FieldInput
          key={f.name}
          field={f}
          register={register}
          error={(errors as FieldErrors)[f.name]?.message as string | undefined}
        />
      ))}
      <button type="submit" style={{
        padding: "8px 16px", background: "#1f2937", color: "#fff", border: 0, borderRadius: 4, cursor: "pointer", justifySelf: "start",
      }}>{submitLabel}</button>
    </form>
  );
}

function buildZodSchema(fields: FieldSpec[]): z.ZodTypeAny {
  const shape: Record<string, z.ZodTypeAny> = {};
  for (const f of fields) {
    let s: z.ZodTypeAny;
    switch (f.type) {
      case "string": case "date": case "date_time": s = z.string(); break;
      case "int":   s = z.coerce.number().int(); break;
      case "float": s = z.coerce.number(); break;
      case "bool":  s = z.coerce.boolean(); break;
      case "enum":  s = f.values.length > 0 ? z.enum(f.values as [string, ...string[]]) : z.string(); break;
      case "ref":   s = z.coerce.number().int(); break;
    }
    if (!f.required) s = s.optional().or(z.literal(""));
    shape[f.name] = s;
  }
  return z.object(shape);
}

/** Drop empty-string-as-unset so server gets clean payloads. */
function coerceForSubmit(values: Record<string, unknown>, fields: FieldSpec[]): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const f of fields) {
    const v = values[f.name];
    if (v === "" || v === undefined || v === null) continue;
    out[f.name] = v;
  }
  return out;
}

interface FieldInputProps {
  field: FieldSpec;
  register: ReturnType<typeof useForm>["register"];
  error?: string;
}

function FieldInput({ field, register, error }: FieldInputProps) {
  const id = `field-${field.name}`;
  let input: React.ReactElement;
  switch (field.type) {
    case "string":
      input = <input id={id} {...register(field.name)} style={inputStyle} />;
      break;
    case "int":
    case "float":
      input = <input id={id} type="number" step={field.type === "int" ? 1 : "any"} {...register(field.name)} style={inputStyle} />;
      break;
    case "ref":
      input = <input id={id} type="number" step={1} {...register(field.name)} style={inputStyle} placeholder={`id of ${(field as { label?: string }).label}`} />;
      break;
    case "bool":
      input = <input id={id} type="checkbox" {...register(field.name)} />;
      break;
    case "date":
      input = <input id={id} type="date" {...register(field.name)} style={inputStyle} />;
      break;
    case "date_time":
      input = <input id={id} type="datetime-local" {...register(field.name)} style={inputStyle} />;
      break;
    case "enum":
      input = (
        <select id={id} {...register(field.name)} style={inputStyle}>
          <option value="">(unset)</option>
          {field.values.map((v) => <option key={v} value={v}>{v}</option>)}
        </select>
      );
      break;
    default: {
      const _exhaustive: never = field;
      throw new Error(`Unhandled field type: ${JSON.stringify(_exhaustive)}`);
    }
  }
  return (
    <label htmlFor={id} style={{ display: "grid", gridTemplateColumns: "140px 1fr", gap: 8, alignItems: "center" }}>
      <span style={{ fontSize: 13, color: "#374151" }}>
        {field.name}{field.required ? <span style={{ color: "#dc2626" }}> *</span> : null}
      </span>
      <div>
        {input}
        {error && <div style={{ color: "#dc2626", fontSize: 12, marginTop: 4 }}>{error}</div>}
      </div>
    </label>
  );
}

const inputStyle: React.CSSProperties = {
  padding: "6px 8px",
  border: "1px solid #d1d5db",
  borderRadius: 4,
  width: "100%",
  fontSize: 14,
};
