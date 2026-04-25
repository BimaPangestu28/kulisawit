import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { useTaskAttempts } from "@/hooks/useTaskAttempts";
import { useUiStore } from "@/store/ui";
import type { AttemptStatus, Task } from "@/types/api";
import { AttemptLogViewer } from "@/components/AttemptLogViewer";

interface Props {
  task: Task;
}

const STATUS_VARIANT: Record<AttemptStatus, "secondary" | "default" | "destructive"> = {
  queued: "secondary",
  running: "default",
  completed: "default",
  failed: "destructive",
  cancelled: "secondary",
};

function relTime(ms: number | null): string {
  if (!ms) return "–";
  const diff = Date.now() - ms;
  if (diff < 60_000) return "just now";
  const mins = Math.floor(diff / 60_000);
  if (mins < 60) return `${mins}m ago`;
  return `${Math.floor(mins / 60)}h ago`;
}

export function LahanAttemptsSection({ task }: Props) {
  const { data, isLoading, isError } = useTaskAttempts(task.id);
  const expandedAttemptId = useUiStore((s) => s.expandedAttemptId);
  const expandAttempt = useUiStore((s) => s.expandAttempt);

  if (isLoading) return <Skeleton className="h-12 w-full" />;
  if (isError) return <div className="text-xs text-destructive" role="alert">Failed to load attempts</div>;
  if (!data || data.length === 0) {
    return (
      <div className="text-xs text-muted-foreground italic">
        No attempts yet — plant a tandan above.
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {data.map((a) => {
        const expanded = expandedAttemptId === a.id;
        return (
          <div key={a.id} className="border rounded">
            <button
              type="button"
              className="w-full text-left flex items-center justify-between p-3 hover:bg-accent/30"
              onClick={() => expandAttempt(expanded ? null : a.id)}
            >
              <div className="flex items-center gap-2">
                <Badge variant={STATUS_VARIANT[a.status]} className="text-xs">
                  {a.status}
                </Badge>
                <span className="text-sm font-medium">{a.agent_id}</span>
                {a.prompt_variant && (
                  <span className="text-xs text-muted-foreground">
                    [{a.prompt_variant}]
                  </span>
                )}
              </div>
              <span className="text-xs text-muted-foreground">
                {relTime(a.started_at)}
              </span>
            </button>
            {expanded && <AttemptLogViewer attemptId={a.id} />}
          </div>
        );
      })}
    </div>
  );
}
