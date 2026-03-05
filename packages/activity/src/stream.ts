import type { OperationType } from "@fixonce/shared";

export interface ActivityEvent {
  id: string;
  operation: OperationType;
  memory_id: string | null;
  details: Record<string, unknown>;
  created_at: string;
}

type ActivityListener = (event: ActivityEvent) => void;

const listeners = new Set<ActivityListener>();

export function subscribeToActivity(listener: ActivityListener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function emitActivity(event: ActivityEvent): void {
  for (const listener of listeners) {
    try {
      listener(event);
    } catch (err) {
      console.error("Activity listener error:", err);
    }
  }
}
