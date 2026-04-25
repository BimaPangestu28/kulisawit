import { useDraggable } from "@dnd-kit/core";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { useUiStore } from "@/store/ui";
import { cn } from "@/lib/utils";
import type { Task } from "@/types/api";

interface Props {
  task: Task;
  /** When true, this card is rendered inside DragOverlay — no drag handlers, distinct styling. */
  dragging?: boolean;
}

export function TaskCard({ task, dragging }: Props) {
  const openDetail = useUiStore((s) => s.openDetail);
  const drag = useDraggable({ id: task.id });

  if (dragging) {
    return (
      <div className="text-left w-full block">
        <Card className="p-3 shadow-lg cursor-grabbing">
          <CardBody task={task} />
        </Card>
      </div>
    );
  }

  return (
    <button
      ref={drag.setNodeRef}
      {...drag.attributes}
      {...drag.listeners}
      type="button"
      onClick={() => openDetail(task.id)}
      className={cn(
        "text-left w-full block focus:outline-none focus:ring-2 focus:ring-ring rounded",
        drag.isDragging && "opacity-50",
      )}
    >
      <Card className="p-3 hover:bg-accent/50 transition-colors cursor-pointer">
        <CardBody task={task} />
      </Card>
    </button>
  );
}

function CardBody({ task }: { task: Task }) {
  return (
    <>
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
    </>
  );
}
