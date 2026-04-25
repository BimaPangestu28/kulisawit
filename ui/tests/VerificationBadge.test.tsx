import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { VerificationBadge } from "@/components/VerificationBadge";

describe("VerificationBadge", () => {
  it("renders correct color class per status", () => {
    const { rerender } = render(<VerificationBadge status="passed" />);
    let dot = screen.getByLabelText(/verification: passed/i);
    expect(dot.className).toMatch(/bg-green-500/);

    rerender(<VerificationBadge status="failed" />);
    dot = screen.getByLabelText(/verification: failed/i);
    expect(dot.className).toMatch(/bg-red-500/);

    rerender(<VerificationBadge status="pending" />);
    dot = screen.getByLabelText(/verification: running/i);
    expect(dot.className).toMatch(/animate-pulse/);

    rerender(<VerificationBadge status={null} />);
    dot = screen.getByLabelText(/verification: not yet run/i);
    expect(dot).toBeInTheDocument();
  });

  it("aria-label matches status text", () => {
    render(<VerificationBadge status="skipped" />);
    expect(screen.getByLabelText("Verification: skipped")).toBeInTheDocument();
  });
});
