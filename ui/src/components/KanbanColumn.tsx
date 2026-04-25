import { useDroppable } from "@dnd-kit/core";
import { Badge } from "@/components/ui/badge";
import { TaskCard } from "@/components/TaskCard";
import { AddTaskInline } from "@/components/AddTaskInline";
import { cn } from "@/lib/utils";
import type { BoardColumn } from "@/types/api";

interface Props {
  column: BoardColumn;
  projectId: string;
}

export function KanbanColumn({ column, projectId }: Props) {
  const { setNodeRef, isOver } = useDroppable({ id: `column:${column.id}` });
  return (
    <div
      ref={setNodeRef}
      data-testid={`column-${column.name}`}
      className={cn(
        "flex flex-col gap-2 min-w-[280px] bg-muted/30 rounded-md p-3 transition-colors",
        isOver && "ring-2 ring-primary/50",
      )}
    >
      <div className="flex items-center justify-between mb-2">
        <h2 data-testid="column-name" className="text-sm font-medium">
          {column.name}
        </h2>
        <Badge variant="secondary" className="text-xs">
          {column.tasks.length}
        </Badge>
      </div>
      {column.tasks.length === 0 ? (
        <div className="text-xs text-muted-foreground italic text-center py-4">
          No tasks
        </div>
      ) : (
        column.tasks.map((task) => <TaskCard key={task.id} task={task} />)
      )}
      <div className="mt-2">
        <AddTaskInline projectId={projectId} columnId={column.id} />
      </div>
    </div>
  );
}
