import { describe, it, expect } from "vitest";
import { render, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useAttemptEvents } from "@/hooks/useAttemptEvents";
import { MockEventSource } from "@/test/setup";

function HookHarness({ id }: { id: string }) {
  useAttemptEvents(id);
  return null;
}

function renderWithClient(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

describe("useAttemptEvents close rule", () => {
  it("closes EventSource on Status with detail starting 'sortir:'", () => {
    renderWithClient(<HookHarness id="aX" />);
    const es = MockEventSource.instances.find((i) => i.url.endsWith("/aX/events"));
    expect(es).toBeDefined();
    act(() => {
      es!.emit({
        attempt_id: "aX",
        event: { type: "status", status: "succeeded", detail: "sortir:passed" },
        ts_ms: 1,
      });
    });
    expect(es!.close).toHaveBeenCalled();
  });

  it("does NOT close on bare terminal Status (detail null)", () => {
    renderWithClient(<HookHarness id="aY" />);
    const es = MockEventSource.instances.find((i) => i.url.endsWith("/aY/events"));
    expect(es).toBeDefined();
    act(() => {
      es!.emit({
        attempt_id: "aY",
        event: { type: "status", status: "succeeded", detail: null },
        ts_ms: 1,
      });
    });
    expect(es!.close).not.toHaveBeenCalled();
  });
});
