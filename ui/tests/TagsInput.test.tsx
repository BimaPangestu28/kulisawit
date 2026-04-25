import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TagsInput } from "@/components/TagsInput";

describe("TagsInput", () => {
  it("renders existing tags as removable chips", () => {
    render(<TagsInput value={["alpha", "beta"]} onChange={vi.fn()} />);
    expect(screen.getByText("alpha")).toBeInTheDocument();
    expect(screen.getByText("beta")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /remove alpha/i })).toBeInTheDocument();
  });

  it("Enter adds new tag (trimmed, deduped)", async () => {
    const onChange = vi.fn();
    render(<TagsInput value={["alpha"]} onChange={onChange} />);
    const input = screen.getByPlaceholderText(/add a tag/i);
    await userEvent.type(input, "  beta  {Enter}");
    expect(onChange).toHaveBeenCalledWith(["alpha", "beta"]);

    onChange.mockClear();
    // Typing duplicate doesn't fire onChange
    await userEvent.type(input, "alpha{Enter}");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("× removes a tag", async () => {
    const onChange = vi.fn();
    render(<TagsInput value={["alpha", "beta"]} onChange={onChange} />);
    await userEvent.click(screen.getByRole("button", { name: /remove alpha/i }));
    expect(onChange).toHaveBeenCalledWith(["beta"]);
  });
});
