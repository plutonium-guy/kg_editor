import { useQuery } from "@tanstack/react-query";
import { getEntity, type EntityDetail } from "../kg/client";

export function useEntity(id: number) {
  return useQuery<EntityDetail>({
    queryKey: ["entity", id],
    queryFn: () => getEntity(id),
    enabled: Number.isFinite(id),
  });
}
