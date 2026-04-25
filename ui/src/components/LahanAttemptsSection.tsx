import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
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

export function LahanAttemptsSection({ task }: Props) {
  const { data, isLoading, isError } = useTaskAttempts(task.id);
  const expandedAttemptId = useUiStore((s) => s.expandedAttemptId);
  const expandAttempt = useUiStore((s) => s.expandAttempt);

  if (isLoading) return <Skeleton className="h-12 w-full" />;
  if (isError)
    return (
      <div className="text-xs text-destructive" role="alert">
        Failed to load attempts
      </div>
    );
  if (!data || data.length === 0) {
    return (
      <div className="text-xs text-muted-foreground italic">
        No attempts yet — plant a tandan above.
      </div>
    );
  }

  const activeValue = expandedAttemptId ?? data[data.length - 1].id;

  return (
    <Tabs
      value={activeValue}
      onValueChange={(v) => expandAttempt(v)}
      className="w-full"
    >
      <TabsList className="w-full justify-start overflow-x-auto">
        {data.map((a) => (
          <TabsTrigger key={a.id} value={a.id} className="flex items-center gap-2 text-xs">
            <Badge variant={STATUS_VARIANT[a.status]} className="text-xs">
              {a.status}
            </Badge>
            <span>{a.agent_id}</span>
          </TabsTrigger>
        ))}
      </TabsList>
      {data.map((a) => (
        <TabsContent key={a.id} value={a.id}>
          <AttemptLogViewer attemptId={a.id} />
        </TabsContent>
      ))}
    </Tabs>
  );
}
