import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { LinkedFilesInput } from "@/components/LinkedFilesInput";

describe("LinkedFilesInput", () => {
  it("textarea reflects prop value joined by newlines", () => {
    render(
      <LinkedFilesInput
        value={["src/parser.rs", "src/lexer.rs"]}
        onChange={vi.fn()}
      />,
    );
    const ta = screen.getByRole("textbox") as HTMLTextAreaElement;
    expect(ta.value).toBe("src/parser.rs\nsrc/lexer.rs");
  });

  it("blur parses split lines and calls onChange", async () => {
    const onChange = vi.fn();
    render(<LinkedFilesInput value={["a"]} onChange={onChange} />);
    const ta = screen.getByRole("textbox");
    await userEvent.clear(ta);
    await userEvent.type(ta, "b{Enter}c{Enter}{Enter}  d  ");
    await userEvent.tab(); // triggers blur
    expect(onChange).toHaveBeenCalledWith(["b", "c", "d"]);
  });
});
