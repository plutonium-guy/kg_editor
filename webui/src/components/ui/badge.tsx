import * as React from "react";
import { cn } from "../../lib/utils";

interface Props extends React.HTMLAttributes<HTMLSpanElement> {
  tone?: "default" | "blue" | "green" | "amber" | "red" | "slate";
}
export function Badge({ tone = "default", className, ...props }: Props) {
  const tones: Record<string, string> = {
    default: "bg-slate-100 text-slate-700",
    blue:    "bg-blue-100 text-blue-700",
    green:   "bg-emerald-100 text-emerald-700",
    amber:   "bg-amber-100 text-amber-800",
    red:     "bg-red-100 text-red-700",
    slate:   "bg-slate-200 text-slate-800",
  };
  return <span className={cn("inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium", tones[tone], className)} {...props} />;
}
