import { useQueryClient } from "@tanstack/react-query";
import { useBoard } from "@/hooks/useBoard";
import { KanbanColumn } from "@/components/KanbanColumn";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";

interface Props {
  projectId: string;
}

export function KanbanBoard({ projectId }: Props) {
  const queryClient = useQueryClient();
  const { data, isLoading, isError } = useBoard(projectId);

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
    <div
      className="grid gap-3 overflow-x-auto"
      style={{ gridTemplateColumns: "repeat(5, minmax(280px, 1fr))" }}
    >
      {data.columns.map((column) => (
        <KanbanColumn key={column.id} column={column} projectId={projectId} />
      ))}
    </div>
  );
}
