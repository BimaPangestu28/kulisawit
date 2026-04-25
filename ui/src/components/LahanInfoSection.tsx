import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { TagsInput } from "@/components/TagsInput";
import { LinkedFilesInput } from "@/components/LinkedFilesInput";
import { useUpdateTask } from "@/hooks/useUpdateTask";
import type { BoardColumn, Task, UpdateTaskRequest } from "@/types/api";

interface Props {
  task: Task;
  columns: BoardColumn[];
}

export function LahanInfoSection({ task, columns }: Props) {
  const [description, setDescription] = useState(task.description ?? "");
  const [tags, setTags] = useState<string[]>(task.tags);
  const [linkedFiles, setLinkedFiles] = useState<string[]>(task.linked_files);
  const update = useUpdateTask();

  // Reset local state when switching to a different task in the drawer.
  useEffect(() => {
    setDescription(task.description ?? "");
    setTags(task.tags);
    setLinkedFiles(task.linked_files);
  }, [task.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const descChanged = description !== (task.description ?? "");
  const tagsChanged = JSON.stringify(tags) !== JSON.stringify(task.tags);
  const filesChanged =
    JSON.stringify(linkedFiles) !== JSON.stringify(task.linked_files);
  const dirty = descChanged || tagsChanged || filesChanged;

  const save = () => {
    const body: UpdateTaskRequest = {};
    if (descChanged) body.description = description;
    if (tagsChanged) body.tags = tags;
    if (filesChanged) body.linked_files = linkedFiles;
    if (Object.keys(body).length === 0) return;
    update.mutate({ id: task.id, body });
  };

  const onColumnChange = (columnId: string) => {
    update.mutate({ id: task.id, body: { column_id: columnId } });
  };

  return (
    <div className="flex flex-col gap-3">
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
      <div className="flex flex-col gap-1.5">
        <Label>Tags</Label>
        <TagsInput value={tags} onChange={setTags} />
      </div>
      <div className="flex flex-col gap-1.5">
        <Label>Linked files</Label>
        <LinkedFilesInput value={linkedFiles} onChange={setLinkedFiles} />
      </div>
      {update.isError && (
        <div role="alert" className="text-xs text-destructive">
          Failed to save changes
        </div>
      )}
      <div className="flex justify-end">
        <Button onClick={save} disabled={!dirty || update.isPending}>
          {update.isPending ? "Saving…" : "Save"}
        </Button>
      </div>
    </div>
  );
}
