import { describe, it, expect } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { http, HttpResponse } from "msw";
import { server } from "@/test/server";
import { LahanDispatchSection } from "@/components/LahanDispatchSection";
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
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

describe("LahanDispatchSection", () => {
  it("agent picker shows mock", async () => {
    renderWithClient(<LahanDispatchSection task={task} />);
    const trigger = screen.getByLabelText(/agent/i);
    await userEvent.click(trigger);
    expect(await screen.findByRole("option", { name: "mock" })).toBeInTheDocument();
  });

  it("plant button calls dispatch with {agent, batch}", async () => {
    let receivedBody: Record<string, unknown> | null = null;
    server.use(
      http.post("/api/tasks/:id/dispatch", async ({ request }) => {
        receivedBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ attempt_ids: ["a1", "a2"] });
      }),
    );
    renderWithClient(<LahanDispatchSection task={task} />);
    const batchInput = screen.getByLabelText(/batch/i);
    await userEvent.clear(batchInput);
    await userEvent.type(batchInput, "2");
    await userEvent.click(screen.getByRole("button", { name: /plant tandan/i }));
    await waitFor(() => {
      expect(receivedBody).toEqual({ agent: "mock", batch: 2 });
    });
  });
});
