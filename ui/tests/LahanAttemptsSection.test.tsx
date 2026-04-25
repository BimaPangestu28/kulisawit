import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { http, HttpResponse } from "msw";
import { server } from "@/test/server";
import { LahanAttemptsSection } from "@/components/LahanAttemptsSection";
import type { Task } from "@/types/api";

const task: Task = {
  id: "t1",
  project_id: "p1",
  column_id: "c1",
  title: "x",
  description: null,
  position: 0,
  tags: [],
  linked_files: [],
  created_at: 0,
  updated_at: 0,
};

function renderWithClient(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

describe("LahanAttemptsSection (tabs)", () => {
  beforeEach(() => {
    server.resetHandlers();
  });

  it("renders one tab per attempt", async () => {
    server.use(
      http.get("/api/tasks/:id/attempts", () =>
        HttpResponse.json([
          { id: "a1", task_id: "t1", agent_id: "mock", status: "running",
            prompt_variant: null, worktree_path: "/x", branch_name: "b1",
            started_at: 1, completed_at: null,
            verification_status: null, verification_output: null },
          { id: "a2", task_id: "t1", agent_id: "mock", status: "completed",
            prompt_variant: null, worktree_path: "/y", branch_name: "b2",
            started_at: 2, completed_at: 3,
            verification_status: null, verification_output: null },
        ]),
      ),
    );
    renderWithClient(<LahanAttemptsSection task={task} />);
    await waitFor(() => {
      expect(screen.getAllByRole("tab")).toHaveLength(2);
    });
  });

  it("default-selects last attempt (most recent)", async () => {
    server.use(
      http.get("/api/tasks/:id/attempts", () =>
        HttpResponse.json([
          { id: "a1", task_id: "t1", agent_id: "mock", status: "running",
            prompt_variant: null, worktree_path: "/x", branch_name: "b1",
            started_at: 1, completed_at: null,
            verification_status: null, verification_output: null },
          { id: "a2", task_id: "t1", agent_id: "mock", status: "completed",
            prompt_variant: null, worktree_path: "/y", branch_name: "b2",
            started_at: 2, completed_at: 3,
            verification_status: null, verification_output: null },
        ]),
      ),
    );
    renderWithClient(<LahanAttemptsSection task={task} />);
    await waitFor(() => {
      const tabs = screen.getAllByRole("tab");
      // Last tab should be active (aria-selected=true)
      expect(tabs[1]).toHaveAttribute("aria-selected", "true");
      expect(tabs[0]).toHaveAttribute("aria-selected", "false");
    });
  });
});
