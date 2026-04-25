import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ProjectSwitcher } from "@/components/ProjectSwitcher";

function renderWithClient(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

describe("ProjectSwitcher", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("renders project names from MSW response", async () => {
    const onSelect = vi.fn();
    renderWithClient(<ProjectSwitcher activeProjectId={null} onSelect={onSelect} />);
    const trigger = await screen.findByRole("combobox");
    await userEvent.click(trigger);
    expect(await screen.findByText("Demo Project")).toBeInTheDocument();
    expect(screen.getByText("Other Project")).toBeInTheDocument();
  });

  it("persists selection to localStorage", async () => {
    const onSelect = vi.fn();
    renderWithClient(<ProjectSwitcher activeProjectId={null} onSelect={onSelect} />);
    const trigger = await screen.findByRole("combobox");
    await userEvent.click(trigger);
    await userEvent.click(await screen.findByText("Demo Project"));
    await waitFor(() => {
      expect(onSelect).toHaveBeenCalledWith("01900000-0000-0000-0000-000000000001");
    });
    expect(localStorage.getItem("kulisawit.activeProject")).toBe(
      "01900000-0000-0000-0000-000000000001",
    );
  });

  it("shows skeleton during loading", () => {
    const onSelect = vi.fn();
    renderWithClient(<ProjectSwitcher activeProjectId={null} onSelect={onSelect} />);
    expect(screen.getByTestId("project-switcher-skeleton")).toBeInTheDocument();
  });
});
