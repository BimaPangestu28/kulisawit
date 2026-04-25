import { describe, it, expect } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { http, HttpResponse } from "msw";
import { server } from "@/test/server";
import { AddTaskInline } from "@/components/AddTaskInline";

function renderWithClient(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

const props = {
  projectId: "01900000-0000-0000-0000-000000000001",
  columnId: "c1",
};

describe("AddTaskInline", () => {
  it("toggles between button and input on click", async () => {
    renderWithClient(<AddTaskInline {...props} />);
    expect(screen.getByRole("button", { name: /\+ add lahan/i })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /\+ add lahan/i }));
    expect(screen.getByPlaceholderText(/lahan title/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /save/i })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /cancel/i }));
    expect(screen.getByRole("button", { name: /\+ add lahan/i })).toBeInTheDocument();
  });

  it("submit calls create mutation with project_id, column_id, title", async () => {
    renderWithClient(<AddTaskInline {...props} />);
    await userEvent.click(screen.getByRole("button", { name: /\+ add lahan/i }));
    await userEvent.type(screen.getByPlaceholderText(/lahan title/i), "ship it");
    await userEvent.click(screen.getByRole("button", { name: /save/i }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /\+ add lahan/i })).toBeInTheDocument();
    });
  });

  it("shows inline error when create fails", async () => {
    server.use(
      http.post("/api/tasks", () => new HttpResponse("server boom", { status: 500 })),
    );
    renderWithClient(<AddTaskInline {...props} />);
    await userEvent.click(screen.getByRole("button", { name: /\+ add lahan/i }));
    await userEvent.type(screen.getByPlaceholderText(/lahan title/i), "x");
    await userEvent.click(screen.getByRole("button", { name: /save/i }));
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent(/failed/i);
    });
    expect(screen.getByPlaceholderText(/lahan title/i)).toHaveValue("x");
  });
});
