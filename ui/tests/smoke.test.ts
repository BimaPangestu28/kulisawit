import { describe, expect, it } from "vitest";
import { api } from "@/lib/api";
import type { Project } from "@/types/api";

describe("test harness", () => {
  it("MSW intercepts /api/projects", async () => {
    const projects = await api.get<Project[]>("/projects");
    expect(projects).toHaveLength(2);
    expect(projects[0].name).toBe("Demo Project");
  });
});
