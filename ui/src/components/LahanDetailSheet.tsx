import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Skeleton } from "@/components/ui/skeleton";
import { useTask } from "@/hooks/useTask";
import { useBoard } from "@/hooks/useBoard";
import { useUiStore } from "@/store/ui";
import { LahanInfoSection } from "@/components/LahanInfoSection";
import { LahanDispatchSection } from "@/components/LahanDispatchSection";
import { LahanAttemptsSection } from "@/components/LahanAttemptsSection";

export function LahanDetailSheet() {
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const isDetailOpen = useUiStore((s) => s.isDetailOpen);
  const closeDetail = useUiStore((s) => s.closeDetail);

  const { data: task, isLoading } = useTask(selectedTaskId);
  const projectId = task?.project_id ?? null;
  const { data: board } = useBoard(projectId);

  return (
    <Sheet open={isDetailOpen} onOpenChange={(o) => { if (!o) closeDetail(); }}>
      <SheetContent side="right" className="w-full sm:max-w-[480px] overflow-y-auto">
        <SheetHeader>
          <SheetTitle>{task?.title ?? "Loading…"}</SheetTitle>
        </SheetHeader>
        {isLoading || !task || !board ? (
          <Skeleton className="h-32 w-full mt-4" />
        ) : (
          <div className="flex flex-col gap-6 py-4">
            <LahanInfoSection task={task} columns={board.columns} />
            <div className="border-t pt-4">
              <h3 className="text-sm font-semibold mb-3">Dispatch</h3>
              <LahanDispatchSection task={task} />
            </div>
            <div className="border-t pt-4">
              <h3 className="text-sm font-semibold mb-3">Attempts (buah)</h3>
              <LahanAttemptsSection task={task} />
            </div>
          </div>
        )}
      </SheetContent>
    </Sheet>
  );
}
