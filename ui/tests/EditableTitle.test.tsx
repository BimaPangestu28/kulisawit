import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { EditableTitle } from "@/components/EditableTitle";

describe("EditableTitle", () => {
  it("renders value with hover-only pencil button", () => {
    render(<EditableTitle value="My title" onSave={vi.fn()} />);
    expect(screen.getByText("My title")).toBeInTheDocument();
    const pencil = screen.getByRole("button", { name: /edit title/i });
    expect(pencil).toBeInTheDocument();
  });

  it("clicks pencil to swap text into autofocused input", async () => {
    render(<EditableTitle value="My title" onSave={vi.fn()} />);
    await userEvent.click(screen.getByRole("button", { name: /edit title/i }));
    const input = screen.getByDisplayValue("My title");
    expect(input).toBeInTheDocument();
    expect(input).toHaveFocus();
  });

  it("Enter calls onSave with trimmed value; Esc cancels and reverts", async () => {
    const onSave = vi.fn();
    render(<EditableTitle value="My title" onSave={onSave} />);
    await userEvent.click(screen.getByRole("button", { name: /edit title/i }));
    const input = screen.getByDisplayValue("My title");
    await userEvent.clear(input);
    await userEvent.type(input, "  New title  {Enter}");
    expect(onSave).toHaveBeenCalledWith("New title");
    expect(screen.getByText("New title")).toBeInTheDocument();

    onSave.mockClear();
    await userEvent.click(screen.getByRole("button", { name: /edit title/i }));
    const input2 = screen.getByDisplayValue("New title");
    await userEvent.clear(input2);
    await userEvent.type(input2, "abandon{Escape}");
    expect(onSave).not.toHaveBeenCalled();
    expect(screen.getByText("New title")).toBeInTheDocument();
  });
});
