import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { VerificationSection } from "@/components/VerificationSection";
import type { Attempt } from "@/types/api";

const baseAttempt: Attempt = {
  id: "a1",
  task_id: "t1",
  agent_id: "mock",
  status: "completed",
  prompt_variant: null,
  worktree_path: "/x",
  branch_name: "b",
  started_at: 0,
  completed_at: 1,
  verification_status: null,
  verification_output: null,
};

describe("VerificationSection", () => {
  it("pending state shows 'Sortir running…' and pulse dot", () => {
    const attempt: Attempt = { ...baseAttempt, verification_status: "pending" };
    render(<VerificationSection attempt={attempt} />);
    expect(screen.getByText(/sortir running/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/running/i)).toBeInTheDocument();
  });

  it("passed state shows the verification_output text", () => {
    const attempt: Attempt = {
      ...baseAttempt,
      verification_status: "passed",
      verification_output: "=== test (passed, 4250ms) ===\nrunning 142 tests",
    };
    render(<VerificationSection attempt={attempt} />);
    expect(screen.getByText(/test \(passed/)).toBeInTheDocument();
    expect(screen.getByText(/running 142 tests/)).toBeInTheDocument();
  });
});
