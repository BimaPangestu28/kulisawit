import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { BoardResponse } from "@/types/api";

export function useBoard(projectId: string | null) {
  return useQuery<BoardResponse>({
    queryKey: ["board", projectId],
    queryFn: () => api.get<BoardResponse>(`/projects/${projectId}/board`),
    enabled: projectId !== null,
  });
}
