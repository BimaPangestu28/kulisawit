import { useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { CreateTaskRequest, Task } from "@/types/api";

export function useCreateTask() {
  const queryClient = useQueryClient();
  return useMutation<Task, Error, CreateTaskRequest>({
    mutationFn: (req) => api.post<Task>("/tasks", req),
    onSuccess: (created) => {
      queryClient.invalidateQueries({
        queryKey: ["board", created.project_id],
      });
    },
  });
}
