import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useUpdateTask } from "@/hooks/useUpdateTask";
import type { BoardColumn, Task, UpdateTaskRequest } from "@/types/api";

interface Props {
  task: Task;
  columns: BoardColumn[];
}

export function LahanInfoSection({ task, columns }: Props) {
  const [title, setTitle] = useState(task.title);
  const [description, setDescription] = useState(task.description ?? "");
  const update = useUpdateTask();

  const titleChanged = title !== task.title;
  const descChanged = description !== (task.description ?? "");
  const dirty = titleChanged || descChanged;

  const save = () => {
    const body: UpdateTaskRequest = {};
    if (titleChanged) body.title = title;
    if (descChanged) body.description = description;
    if (Object.keys(body).length === 0) return;
    update.mutate({ id: task.id, body });
  };

  const onColumnChange = (columnId: string) => {
    update.mutate({ id: task.id, body: { column_id: columnId } });
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="lahan-title">Title</Label>
        <Input
          id="lahan-title"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
        />
      </div>
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="lahan-description">Description</Label>
        <Textarea
          id="lahan-description"
          rows={4}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
      </div>
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="lahan-column">Column</Label>
        <Select value={task.column_id} onValueChange={onColumnChange}>
          <SelectTrigger id="lahan-column" aria-label="Column">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {columns.map((c) => (
              <SelectItem key={c.id} value={c.id}>
                {c.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      {update.isError && (
        <div role="alert" className="text-xs text-destructive">
          Failed to save changes
        </div>
      )}
      <div className="flex justify-end">
        <Button
          onClick={save}
          disabled={!dirty || update.isPending}
        >
          {update.isPending ? "Saving…" : "Save"}
        </Button>
      </div>
    </div>
  );
}
