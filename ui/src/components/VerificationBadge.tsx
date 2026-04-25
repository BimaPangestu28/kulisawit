import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import type { VerificationStatus } from "@/types/api";

interface Props {
  status: VerificationStatus | null;
}

const COLOR: Record<NonNullable<VerificationStatus> | "null", string> = {
  passed: "bg-green-500",
  failed: "bg-red-500",
  pending: "bg-zinc-400 animate-pulse",
  skipped: "bg-zinc-500",
  null: "bg-zinc-700",
};

const LABEL: Record<NonNullable<VerificationStatus> | "null", string> = {
  passed: "Verification: passed",
  failed: "Verification: failed",
  pending: "Verification: running…",
  skipped: "Verification: skipped",
  null: "Verification: not yet run",
};

export function VerificationBadge({ status }: Props) {
  const key = (status ?? "null") as NonNullable<VerificationStatus> | "null";
  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <span
            aria-label={LABEL[key]}
            className={cn("inline-block h-2 w-2 rounded-full", COLOR[key])}
          />
        </TooltipTrigger>
        <TooltipContent>{LABEL[key]}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
