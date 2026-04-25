import { cn } from "@/lib/utils";
import type { Attempt } from "@/types/api";

interface Props {
  attempt: Attempt;
}

export function VerificationSection({ attempt }: Props) {
  const status = attempt.verification_status;
  const output = attempt.verification_output;

  return (
    <section className="border rounded p-3 mb-3">
      <header className="flex items-center gap-2 mb-2">
        <span className="text-xs font-semibold uppercase text-muted-foreground">
          Verification
        </span>
        <span
          className={cn(
            "text-xs font-medium",
            status === "passed" && "text-green-600 dark:text-green-400",
            status === "failed" && "text-red-600 dark:text-red-400",
            status === "skipped" && "text-muted-foreground",
            status === "pending" && "text-muted-foreground italic",
            status === null && "text-muted-foreground italic",
          )}
        >
          {status === null ? "not yet run" : status}
        </span>
        {status === "pending" && (
          <span
            className="inline-block h-2 w-2 rounded-full bg-zinc-400 animate-pulse"
            aria-label="Running"
          />
        )}
      </header>
      {output && (
        <pre className="text-xs font-mono bg-muted/30 rounded p-2 max-h-64 overflow-auto whitespace-pre-wrap">
          {output}
        </pre>
      )}
      {!output && status === "pending" && (
        <p className="text-xs text-muted-foreground italic">Sortir running…</p>
      )}
    </section>
  );
}
