import { useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { BoardResponse, Task, UpdateTaskRequest } from "@/types/api";

interface Vars {
  id: string;
  body: UpdateTaskRequest;
}

interface MutationContext {
  snapshot?: ReturnType<ReturnType<typeof useQueryClient>["getQueriesData"]>;
}

export function useUpdateTask() {
  const queryClient = useQueryClient();
  return useMutation<Task, Error, Vars, MutationContext>({
    mutationFn: ({ id, body }) => api.patch<Task>(`/tasks/${id}`, body),
    onMutate: async ({ id, body }) => {
      // Only run optimistic UI for column moves.
      if (body.column_id === undefined) return {};
      await queryClient.cancelQueries({ queryKey: ["board"] });
      const snapshot = queryClient.getQueriesData<BoardResponse>({
        queryKey: ["board"],
      });
      queryClient.setQueriesData<BoardResponse>(
        { queryKey: ["board"] },
        (old) => {
          if (!old) return old;
          const allTasks = old.columns.flatMap((c) => c.tasks);
          const moved = allTasks.find((t) => t.id === id);
          if (!moved) return old;
          const updated = { ...moved, column_id: body.column_id! };
          return {
            ...old,
            columns: old.columns.map((c) => ({
              ...c,
              tasks:
                c.id === body.column_id
                  ? [...c.tasks.filter((t) => t.id !== id), updated]
                  : c.tasks.filter((t) => t.id !== id),
            })),
          };
        },
      );
      return { snapshot };
    },
    onError: (_err, _vars, ctx) => {
      if (!ctx?.snapshot) return;
      ctx.snapshot.forEach(([key, data]) => {
        queryClient.setQueryData(key, data);
      });
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["board"] });
      queryClient.invalidateQueries({ queryKey: ["task"] });
    },
  });
}
