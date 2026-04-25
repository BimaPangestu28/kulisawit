import { useState, useEffect } from "react";
import { Textarea } from "@/components/ui/textarea";

interface Props {
  value: string[];
  onChange: (files: string[]) => void;
}

export function LinkedFilesInput({ value, onChange }: Props) {
  const [text, setText] = useState(value.join("\n"));

  // Sync local text when parent value changes (e.g. drawer reopen for different task)
  useEffect(() => {
    setText(value.join("\n"));
  }, [value]);

  const commit = () => {
    const parsed = text
      .split("\n")
      .map((s) => s.trim())
      .filter(Boolean);
    if (JSON.stringify(parsed) !== JSON.stringify(value)) {
      onChange(parsed);
    }
  };

  return (
    <Textarea
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      placeholder={"src/parser.rs\nsrc/lexer.rs"}
      rows={3}
    />
  );
}
