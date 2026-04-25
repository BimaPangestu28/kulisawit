import { useAttemptEvents, isStreamConnecting } from "@/hooks/useAttemptEvents";
import { VerificationSection } from "@/components/VerificationSection";
import type { Attempt } from "@/types/api";

interface Props {
  attempt: Attempt;
}

function formatLine(receivedAt: number, eventType: string, text: string) {
  const time = new Date(receivedAt).toISOString().slice(11, 19);
  return `[${time}] ${eventType}: ${text}`;
}

export function AttemptLogViewer({ attempt }: Props) {
  const events = useAttemptEvents(attempt.id);
  const connecting = isStreamConnecting(events);

  return (
    <div className="flex flex-col">
      <VerificationSection attempt={attempt} />
      {connecting ? (
        <div className="text-xs text-muted-foreground italic px-3 py-2">
          Connecting…
        </div>
      ) : (
        <pre className="text-xs font-mono bg-muted/30 rounded p-3 max-h-64 overflow-auto whitespace-pre-wrap">
          {events.map(({ envelope, receivedAt }, i) => {
            const evt = envelope.event;
            let text = "";
            switch (evt.type) {
              case "stdout":
              case "stderr":
                text = evt.text;
                break;
              case "tool_call":
                text = `${evt.name}(${JSON.stringify(evt.input)})`;
                break;
              case "tool_result":
                text = `${evt.name} -> ${JSON.stringify(evt.output)}`;
                break;
              case "file_edit":
                text = evt.path;
                break;
              case "status":
                text = `${evt.status}${evt.detail ? ` (${evt.detail})` : ""}`;
                break;
            }
            return <div key={i}>{formatLine(receivedAt, evt.type, text)}</div>;
          })}
        </pre>
      )}
    </div>
  );
}
