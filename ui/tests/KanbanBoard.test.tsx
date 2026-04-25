import { describe, it, expect } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { DndContext } from "@dnd-kit/core";
import { http, HttpResponse } from "msw";
import { server } from "@/test/server";
import { KanbanBoard } from "@/components/KanbanBoard";

function renderWithClient(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <DndContext>{ui}</DndContext>
    </QueryClientProvider>,
  );
}

describe("KanbanBoard", () => {
  it("renders 5 columns in order Backlog -> Done", async () => {
    renderWithClient(
      <KanbanBoard projectId="01900000-0000-0000-0000-000000000001" />,
    );
    await waitFor(() => {
      expect(screen.getByText("Backlog")).toBeInTheDocument();
    });
    const headings = screen.getAllByTestId("column-name").map((el) => el.textContent);
    expect(headings).toEqual(["Backlog", "Todo", "Doing", "Review", "Done"]);
  });

  it("renders task cards in correct columns", async () => {
    renderWithClient(
      <KanbanBoard projectId="01900000-0000-0000-0000-000000000001" />,
    );
    await waitFor(() => {
      expect(screen.getByText("First task")).toBeInTheDocument();
    });
    const todoColumn = screen.getByTestId("column-Todo");
    expect(todoColumn).toHaveTextContent("First task");
  });

  it("shows error card when board fetch returns 500", async () => {
    server.use(
      http.get("/api/projects/:id/board", () => new HttpResponse(null, { status: 500 })),
    );
    renderWithClient(
      <KanbanBoard projectId="01900000-0000-0000-0000-000000000001" />,
    );
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent(/Failed to load board/i);
    });
  });
});
