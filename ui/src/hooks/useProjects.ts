import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { Project } from "@/types/api";

export function useProjects() {
  return useQuery<Project[]>({
    queryKey: ["projects"],
    queryFn: () => api.get<Project[]>("/projects"),
  });
}
