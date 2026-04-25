import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { http } from "msw";
import { server } from "@/test/server";
import { LahanInfoSection } from "@/components/LahanInfoSection";
import type { Task, BoardColumn } from "@/types/api";

const task: Task = {
  id: "t1",
  project_id: "p1",
  column_id: "c1",
  title: "Refactor parser",
  description: "Make it streaming",
  position: 0,
  tags: ["initial"],
  linked_files: ["src/parser.rs"],
  created_at: 0,
  updated_at: 0,
};

const columns: BoardColumn[] = [
  { id: "c1", name: "Backlog", position: 0, tasks: [] },
  { id: "c2", name: "Todo", position: 1, tasks: [] },
  { id: "c3", name: "Doing", position: 2, tasks: [] },
];

function renderWithClient(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

describe("LahanInfoSection (post-3.2.3 refactor)", () => {
  beforeEach(() => {
    server.resetHandlers();
  });

  it("does NOT render title input (handled by EditableTitle in drawer header)", () => {
    renderWithClient(<LahanInfoSection task={task} columns={columns} />);
    expect(screen.queryByDisplayValue("Refactor parser")).not.toBeInTheDocument();
    // Description still present
    expect(screen.getByDisplayValue("Make it streaming")).toBeInTheDocument();
  });

  it("save sends only changed fields including tags+files", async () => {
    let receivedBody: Record<string, unknown> | null = null;
    server.use(
      http.patch("/api/tasks/:id", async ({ request }) => {
        receivedBody = (await request.json()) as Record<string, unknown>;
        return new Response(
          JSON.stringify({ ...task, description: "new desc" }),
          { headers: { "content-type": "application/json" } },
        );
      }),
    );
    renderWithClient(<LahanInfoSection task={task} columns={columns} />);
    const desc = screen.getByDisplayValue("Make it streaming");
    await userEvent.clear(desc);
    await userEvent.type(desc, "new desc");
    await userEvent.click(screen.getByRole("button", { name: /save/i }));
    await waitFor(() => {
      expect(receivedBody).not.toBeNull();
    });
    expect(receivedBody).toEqual({ description: "new desc" });
  });

  it("move dropdown fires immediately on selection", async () => {
    let receivedBody: Record<string, unknown> | null = null;
    server.use(
      http.patch("/api/tasks/:id", async ({ request }) => {
        receivedBody = (await request.json()) as Record<string, unknown>;
        return new Response(
          JSON.stringify({ ...task, column_id: "c2" }),
          { headers: { "content-type": "application/json" } },
        );
      }),
    );
    renderWithClient(<LahanInfoSection task={task} columns={columns} />);
    const trigger = screen.getByLabelText(/column/i);
    await userEvent.click(trigger);
    await userEvent.click(await screen.findByRole("option", { name: "Todo" }));
    await waitFor(() => {
      expect(receivedBody).toEqual({ column_id: "c2" });
    });
  });
});
