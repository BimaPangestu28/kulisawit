import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import type { Task } from "@/types/api";

interface Props {
  task: Task;
}

export function TaskCard({ task }: Props) {
  return (
    <Card className="p-3 hover:bg-accent/50 transition-colors">
      <div className="font-medium text-sm">{task.title}</div>
      {task.description && (
        <div className="mt-1 text-xs text-muted-foreground line-clamp-2">
          {task.description}
        </div>
      )}
      {task.tags.length > 0 && (
        <div className="mt-2 flex flex-wrap gap-1">
          {task.tags.map((tag) => (
            <Badge key={tag} variant="secondary" className="text-xs">
              {tag}
            </Badge>
          ))}
        </div>
      )}
      {task.linked_files.length > 0 && (
        <div className="mt-2 text-xs text-muted-foreground">
          📎 {task.linked_files.length} file{task.linked_files.length === 1 ? "" : "s"}
        </div>
      )}
    </Card>
  );
}
