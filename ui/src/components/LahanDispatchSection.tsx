import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useDispatchTask } from "@/hooks/useDispatchTask";
import type { Task } from "@/types/api";

const AGENTS = ["mock"] as const;

interface Props {
  task: Task;
}

export function LahanDispatchSection({ task }: Props) {
  const [agent, setAgent] = useState<string>("mock");
  const [batchStr, setBatchStr] = useState<string>("1");
  const batch = Number(batchStr);
  const dispatch = useDispatchTask();

  const onPlant = () => {
    if (!batch || batch < 1 || batch > 10) return;
    dispatch.mutate({ id: task.id, body: { agent, batch } });
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="dispatch-agent">Agent</Label>
        <Select value={agent} onValueChange={setAgent}>
          <SelectTrigger id="dispatch-agent" aria-label="Agent">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {AGENTS.map((a) => (
              <SelectItem key={a} value={a}>
                {a}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="dispatch-batch">Batch</Label>
        <Input
          id="dispatch-batch"
          aria-label="Batch"
          type="number"
          min={1}
          max={10}
          value={batchStr}
          onChange={(e) => setBatchStr(e.target.value)}
        />
      </div>
      {dispatch.isError && (
        <div role="alert" className="text-xs text-destructive">
          Failed to plant tandan
        </div>
      )}
      <Button
        onClick={onPlant}
        disabled={dispatch.isPending || !batch || batch < 1 || batch > 10}
      >
        {dispatch.isPending ? "Planting…" : "Plant tandan"}
      </Button>
    </div>
  );
}
