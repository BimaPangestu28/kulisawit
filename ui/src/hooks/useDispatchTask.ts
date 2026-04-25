import { useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { DispatchRequest, DispatchResponse } from "@/types/api";

interface Vars {
  id: string;
  body: DispatchRequest;
}

export function useDispatchTask() {
  const queryClient = useQueryClient();
  return useMutation<DispatchResponse, Error, Vars>({
    mutationFn: ({ id, body }) =>
      api.post<DispatchResponse>(`/tasks/${id}/dispatch`, body),
    onSuccess: (_resp, vars) => {
      queryClient.invalidateQueries({
        queryKey: ["task-attempts", vars.id],
      });
    },
  });
}
