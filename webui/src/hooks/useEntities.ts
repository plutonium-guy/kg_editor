import { useQuery } from "@tanstack/react-query";
import { listEntities, type EntityDetail } from "../kg/client";

export function useEntities(label?: string, q?: string, limit = 50) {
  return useQuery<EntityDetail[]>({
    queryKey: ["entities", label ?? null, q ?? null, limit],
    queryFn: () => listEntities(label, q, limit),
  });
}
