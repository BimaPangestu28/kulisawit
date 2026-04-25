import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useCreateTask } from "@/hooks/useCreateTask";

interface Props {
  projectId: string;
  columnId: string;
}

export function AddTaskInline({ projectId, columnId }: Props) {
  const [editing, setEditing] = useState(false);
  const [title, setTitle] = useState("");
  const create = useCreateTask();

  const cancel = () => {
    setEditing(false);
    setTitle("");
    create.reset();
  };

  const submit = async () => {
    if (!title.trim()) return;
    try {
      await create.mutateAsync({
        project_id: projectId,
        column_id: columnId,
        title: title.trim(),
      });
      cancel();
    } catch {
      // Error displayed via create.isError below; keep editing state.
    }
  };

  if (!editing) {
    return (
      <Button
        variant="ghost"
        size="sm"
        className="w-full justify-start text-muted-foreground hover:text-foreground"
        onClick={() => setEditing(true)}
      >
        + Add lahan
      </Button>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      <Input
        autoFocus
        placeholder="Lahan title"
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void submit();
          if (e.key === "Escape") cancel();
        }}
        disabled={create.isPending}
      />
      {create.isError && (
        <div role="alert" className="text-xs text-destructive">
          Failed to create lahan
        </div>
      )}
      <div className="flex gap-2">
        <Button size="sm" onClick={submit} disabled={create.isPending || !title.trim()}>
          {create.isPending ? "Saving…" : "Save"}
        </Button>
        <Button size="sm" variant="ghost" onClick={cancel} disabled={create.isPending}>
          Cancel
        </Button>
      </div>
    </div>
  );
}
