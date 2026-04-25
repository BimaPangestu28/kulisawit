import { http, HttpResponse } from "msw";
import type { BoardResponse, Project } from "@/types/api";

export const sampleProjects: Project[] = [
  {
    id: "01900000-0000-0000-0000-000000000001",
    name: "Demo Project",
    repo_path: "/tmp/demo",
    created_at: 1745000000000,
    column_ids: [],
  },
  {
    id: "01900000-0000-0000-0000-000000000002",
    name: "Other Project",
    repo_path: "/tmp/other",
    created_at: 1744000000000,
    column_ids: [],
  },
];

export const sampleBoard: BoardResponse = {
  project: sampleProjects[0],
  columns: [
    { id: "c1", name: "Backlog", position: 0, tasks: [] },
    {
      id: "c2",
      name: "Todo",
      position: 1,
      tasks: [
        {
          id: "t1",
          project_id: sampleProjects[0].id,
          column_id: "c2",
          title: "First task",
          description: "Hello world",
          position: 0,
          tags: ["foo", "bar"],
          linked_files: [],
          created_at: 0,
          updated_at: 0,
        },
      ],
    },
    { id: "c3", name: "Doing", position: 2, tasks: [] },
    { id: "c4", name: "Review", position: 3, tasks: [] },
    { id: "c5", name: "Done", position: 4, tasks: [] },
  ],
};

export const handlers = [
  http.get("/api/projects", () => HttpResponse.json(sampleProjects)),
  http.get("/api/projects/:id/board", ({ params }) => {
    if (params.id !== sampleProjects[0].id) {
      return new HttpResponse(null, { status: 404 });
    }
    return HttpResponse.json(sampleBoard);
  }),
];
