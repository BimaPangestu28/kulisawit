import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { Task } from "@/types/api";

export function useTask(taskId: string | null) {
  return useQuery<Task>({
    queryKey: ["task", taskId],
    queryFn: () => api.get<Task>(`/tasks/${taskId}`),
    enabled: taskId !== null,
  });
}
