import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import {
  TERMINAL_RUN_STATUSES,
  type EventEnvelope,
} from "@/types/api";

export interface ReceivedEvent {
  envelope: EventEnvelope;
  receivedAt: number;
}

/**
 * Opens an SSE connection to /api/attempts/:id/events for one attempt.
 * NOT a TanStack Query — EventSource lifecycle doesn't fit the cache model.
 * Auto-closes on terminal RunStatus and invalidates relevant query caches.
 */
export function useAttemptEvents(attemptId: string | null): ReceivedEvent[] {
  const [events, setEvents] = useState<ReceivedEvent[]>([]);
  const queryClient = useQueryClient();
  const sourceRef = useRef<EventSource | null>(null);

  useEffect(() => {
    if (!attemptId) return;
    setEvents([]);
    const es = new EventSource(`/api/attempts/${attemptId}/events`);
    sourceRef.current = es;

    es.onmessage = (msg: MessageEvent) => {
      try {
        const envelope = JSON.parse(msg.data) as EventEnvelope;
        setEvents((prev) => [
          ...prev,
          { envelope, receivedAt: Date.now() },
        ]);
        if (
          envelope.event.type === "status" &&
          TERMINAL_RUN_STATUSES.includes(envelope.event.status)
        ) {
          es.close();
          queryClient.invalidateQueries({ queryKey: ["attempt", attemptId] });
          queryClient.invalidateQueries({ queryKey: ["task-attempts"] });
          queryClient.invalidateQueries({ queryKey: ["board"] });
        }
      } catch {
        // Malformed frame — ignore. Production server only emits valid JSON.
      }
    };

    es.onerror = () => {
      es.close();
    };

    return () => {
      es.close();
      sourceRef.current = null;
    };
  }, [attemptId, queryClient]);

  return events;
}

/** Test-only helper to allow components to detect that no events have arrived yet. */
export function isStreamConnecting(events: ReceivedEvent[]): boolean {
  return events.length === 0;
}
