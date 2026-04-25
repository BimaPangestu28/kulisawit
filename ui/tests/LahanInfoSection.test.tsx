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
  tags: [],
  linked_files: [],
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

describe("LahanInfoSection", () => {
  beforeEach(() => {
    server.resetHandlers();
  });

  it("pre-fills inputs from task", () => {
    renderWithClient(<LahanInfoSection task={task} columns={columns} />);
    expect(screen.getByDisplayValue("Refactor parser")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Make it streaming")).toBeInTheDocument();
  });

  it("save button calls update with only changed fields", async () => {
    let receivedBody: Record<string, unknown> | null = null;
    server.use(
      http.patch("/api/tasks/:id", async ({ request }) => {
        receivedBody = (await request.json()) as Record<string, unknown>;
        return new Response(
          JSON.stringify({ ...task, title: "New title" }),
          { headers: { "content-type": "application/json" } },
        );
      }),
    );
    renderWithClient(<LahanInfoSection task={task} columns={columns} />);
    const titleInput = screen.getByDisplayValue("Refactor parser");
    await userEvent.clear(titleInput);
    await userEvent.type(titleInput, "New title");
    await userEvent.click(screen.getByRole("button", { name: /save/i }));
    await waitFor(() => {
      expect(receivedBody).not.toBeNull();
    });
    expect(receivedBody).toEqual({ title: "New title" });
  });

  it("move dropdown fires update immediately on selection", async () => {
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
