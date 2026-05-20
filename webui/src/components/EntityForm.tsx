import { useMemo } from "react";
import { useForm, type FieldErrors } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import type { FieldSpec } from "../kg/schema";
import { Input, Select } from "./ui/input";
import { Button } from "./ui/button";

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
    <form onSubmit={handleSubmit((v) => onSubmit(coerceForSubmit(v, fields)))} className="space-y-4">
      {fields.map((f) => (
        <FieldInput
          key={f.name}
          field={f}
          register={register}
          error={(errors as FieldErrors)[f.name]?.message as string | undefined}
        />
      ))}
      <Button type="submit" variant="primary">{submitLabel}</Button>
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
      default: s = z.unknown();
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
      input = <Input id={id} {...register(field.name)} />;
      break;
    case "int":
    case "float":
      input = <Input id={id} type="number" step={field.type === "int" ? 1 : "any"} {...register(field.name)} />;
      break;
    case "ref":
      input = <Input id={id} type="number" step={1} {...register(field.name)} placeholder={`id of ${(field as { label?: string }).label ?? ""}`} />;
      break;
    case "bool":
      input = <input id={id} type="checkbox" className="h-4 w-4 rounded border-slate-300 text-blue-600 focus:ring-blue-500" {...register(field.name)} />;
      break;
    case "date":
      input = <Input id={id} type="date" {...register(field.name)} />;
      break;
    case "date_time":
      input = <Input id={id} type="datetime-local" {...register(field.name)} />;
      break;
    case "enum":
      input = (
        <Select id={id} {...register(field.name)}>
          <option value="">(unset)</option>
          {field.values.map((v) => <option key={v} value={v}>{v}</option>)}
        </Select>
      );
      break;
    default:
      input = <Input id={id} {...register((field as FieldSpec).name)} />;
  }
  return (
    <div>
      <label htmlFor={id} className="block text-sm font-medium text-slate-700 mb-1">
        {field.name}{field.required && <span className="text-red-500"> *</span>}
      </label>
      {input}
      {error && <p className="mt-1 text-xs text-red-600">{error}</p>}
    </div>
  );
}
