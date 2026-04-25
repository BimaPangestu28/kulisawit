import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import { useBoard } from "@/hooks/useBoard";
import { useUpdateTask } from "@/hooks/useUpdateTask";
import { KanbanColumn } from "@/components/KanbanColumn";
import { TaskCard } from "@/components/TaskCard";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import type { Task } from "@/types/api";

interface Props {
  projectId: string;
}

export function KanbanBoard({ projectId }: Props) {
  const queryClient = useQueryClient();
  const { data, isLoading, isError } = useBoard(projectId);
  const update = useUpdateTask();
  const [draggingTask, setDraggingTask] = useState<Task | null>(null);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    }),
  );

  const onDragStart = (evt: DragStartEvent) => {
    const taskId = evt.active.id as string;
    const task =
      data?.columns.flatMap((c) => c.tasks).find((t) => t.id === taskId) ?? null;
    setDraggingTask(task);
  };

  const onDragEnd = (evt: DragEndEvent) => {
    setDraggingTask(null);
    const taskId = evt.active.id as string;
    const overId = evt.over?.id as string | undefined;
    if (!overId || !overId.startsWith("column:")) return;
    const targetColumnId = overId.slice("column:".length);
    const sourceTask = data?.columns.flatMap((c) => c.tasks).find((t) => t.id === taskId);
    if (!sourceTask || sourceTask.column_id === targetColumnId) return;
    update.mutate({ id: taskId, body: { column_id: targetColumnId } });
  };

  if (isLoading) {
    return (
      <div className="grid grid-cols-5 gap-3">
        {Array.from({ length: 5 }).map((_, i) => (
          <Skeleton key={i} className="h-64" />
        ))}
      </div>
    );
  }

  if (isError) {
    return (
      <Card className="p-4 border-destructive" role="alert">
        <div className="text-sm text-destructive font-medium">Failed to load board</div>
        <Button
          className="mt-2"
          variant="outline"
          size="sm"
          onClick={() =>
            queryClient.invalidateQueries({ queryKey: ["board", projectId] })
          }
        >
          Retry
        </Button>
      </Card>
    );
  }

  if (!data) return null;

  return (
    <DndContext sensors={sensors} onDragStart={onDragStart} onDragEnd={onDragEnd}>
      <div
        className="grid gap-3 overflow-x-auto"
        style={{ gridTemplateColumns: "repeat(5, minmax(280px, 1fr))" }}
      >
        {data.columns.map((column) => (
          <KanbanColumn key={column.id} column={column} projectId={projectId} />
        ))}
      </div>
      <DragOverlay>
        {draggingTask ? <TaskCard task={draggingTask} dragging /> : null}
      </DragOverlay>
    </DndContext>
  );
}
