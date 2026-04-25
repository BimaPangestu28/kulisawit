import { useState } from "react";
import { Pencil } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

interface Props {
  value: string;
  onSave: (newValue: string) => void;
  pending?: boolean;
}

export function EditableTitle({ value, onSave, pending }: Props) {
  const [editing, setEditing] = useState(false);
  const [committed, setCommitted] = useState(value);
  const [draft, setDraft] = useState(value);

  if (!editing) {
    return (
      <div className="group flex items-center gap-2">
        <span className="text-lg font-semibold">{committed}</span>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          aria-label="Edit title"
          className="opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 h-6 w-6"
          onClick={() => {
            setDraft(committed);
            setEditing(true);
          }}
        >
          <Pencil className="h-3 w-3" />
        </Button>
      </div>
    );
  }

  const cancel = () => {
    setDraft(committed);
    setEditing(false);
  };

  const save = () => {
    const trimmed = draft.trim();
    if (trimmed && trimmed !== committed) {
      onSave(trimmed);
      setCommitted(trimmed);
    }
    setEditing(false);
  };

  return (
    <div className="flex items-center gap-2">
      <Input
        autoFocus
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") save();
          if (e.key === "Escape") cancel();
        }}
        disabled={pending}
        className="h-8"
      />
      <Button size="sm" onClick={save} disabled={pending || !draft.trim()}>
        Save
      </Button>
      <Button size="sm" variant="ghost" onClick={cancel} disabled={pending}>
        Cancel
      </Button>
    </div>
  );
}
