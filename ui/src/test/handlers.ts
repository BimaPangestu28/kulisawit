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

  http.post("/api/tasks", async ({ request }) => {
    const body = (await request.json()) as Record<string, unknown>;
    const fakeTask = {
      id: "new-task-id",
      project_id: body.project_id,
      column_id: body.column_id,
      title: body.title,
      description: body.description ?? null,
      position: 0,
      tags: body.tags ?? [],
      linked_files: body.linked_files ?? [],
      created_at: 1745000000000,
      updated_at: 1745000000000,
    };
    return HttpResponse.json(fakeTask);
  }),

  http.patch("/api/tasks/:id", async ({ params, request }) => {
    const body = (await request.json()) as Record<string, unknown>;
    return HttpResponse.json({
      id: params.id,
      project_id: "01900000-0000-0000-0000-000000000001",
      column_id: body.column_id ?? "c1",
      title: body.title ?? "Existing title",
      description: body.description ?? null,
      position: 0,
      tags: [],
      linked_files: [],
      created_at: 1745000000000,
      updated_at: 1745000000001,
    });
  }),

  http.post("/api/tasks/:id/dispatch", async () =>
    HttpResponse.json({ attempt_ids: ["a1", "a2"] }),
  ),

  http.get("/api/tasks/:id/attempts", () =>
    HttpResponse.json([
      {
        id: "a1",
        task_id: "t1",
        agent_id: "mock",
        status: "running",
        prompt_variant: null,
        worktree_path: "/tmp/wt/a1",
        branch_name: "kulisawit/a1",
        started_at: 1745000000000,
        completed_at: null,
        verification_status: null,
        verification_output: null,
      },
    ]),
  ),

  http.get("/api/tasks/:id", ({ params }) =>
    HttpResponse.json({
      id: params.id,
      project_id: "01900000-0000-0000-0000-000000000001",
      column_id: "c1",
      title: "Refactor parser",
      description: "Make it streaming",
      position: 0,
      tags: ["refactor"],
      linked_files: [],
      created_at: 1745000000000,
      updated_at: 1745000000000,
    }),
  ),
];
