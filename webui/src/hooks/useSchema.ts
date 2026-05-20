import { useQuery } from "@tanstack/react-query";
import type { SchemaFile } from "../kg/schema";

const BASE = (import.meta.env.VITE_KG_SERVER_BASE as string | undefined) ?? "http://localhost:9000";

export function useSchema() {
  return useQuery<SchemaFile>({
    queryKey: ["schema"],
    queryFn: async () => {
      const r = await fetch(`${BASE}/schema`);
      if (!r.ok) throw new Error(`schema fetch ${r.status}`);
      return r.json();
    },
    staleTime: Infinity,
  });
}
