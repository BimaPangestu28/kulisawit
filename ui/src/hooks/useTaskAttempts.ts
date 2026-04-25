import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { Attempt } from "@/types/api";

export function useTaskAttempts(taskId: string | null) {
  return useQuery<Attempt[]>({
    queryKey: ["task-attempts", taskId],
    queryFn: () => api.get<Attempt[]>(`/tasks/${taskId}/attempts`),
    enabled: taskId !== null,
  });
}
