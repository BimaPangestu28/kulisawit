import { describe, it, expect } from "vitest";
import { render, screen, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AttemptLogViewer } from "@/components/AttemptLogViewer";
import { MockEventSource } from "@/test/setup";
import type { Attempt } from "@/types/api";

const baseAttempt: Attempt = {
  id: "a1",
  task_id: "t1",
  agent_id: "mock",
  status: "running",
  prompt_variant: null,
  worktree_path: "/x",
  branch_name: "b",
  started_at: 0,
  completed_at: null,
  verification_status: null,
  verification_output: null,
};

function renderWithClient(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

describe("AttemptLogViewer", () => {
  it("renders received SSE frames", () => {
    renderWithClient(<AttemptLogViewer attempt={baseAttempt} />);
    expect(screen.getByText(/connecting/i)).toBeInTheDocument();
    const es = MockEventSource.instances.find((i) => i.url.endsWith("/a1/events"));
    expect(es).toBeDefined();
    act(() => {
      es!.emit({
        attempt_id: "a1",
        event: { type: "stdout", text: "compiling..." },
        ts_ms: 1745000000000,
      });
    });
    expect(screen.getByText(/compiling/i)).toBeInTheDocument();
  });

  it("closes EventSource on Status with detail 'sortir:passed'", () => {
    renderWithClient(<AttemptLogViewer attempt={baseAttempt} />);
    const es = MockEventSource.instances.find((i) => i.url.endsWith("/a1/events"));
    expect(es).toBeDefined();
    act(() => {
      es!.emit({
        attempt_id: "a1",
        event: { type: "status", status: "succeeded", detail: "sortir:passed" },
        ts_ms: 1745000000001,
      });
    });
    expect(es!.close).toHaveBeenCalled();
  });
});
