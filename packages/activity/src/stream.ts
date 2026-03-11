import type { OperationType } from "@fixonce/shared";
import { createSupabaseClient } from "@fixonce/storage";

export interface ActivityEvent {
  id: string;
  operation: OperationType;
  memory_id: string | null;
  details: Record<string, unknown>;
  created_at: string;
}

export type ActivityListener = (event: ActivityEvent) => void;

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

/**
 * Subscribe to activity events via Supabase Realtime (postgres_changes).
 * Unlike `subscribeToActivity`, this works across processes because it
 * listens to INSERT events on the `activity_log` table via Supabase's
 * built-in Realtime engine.
 */
export function subscribeToActivityRealtime(
  listener: ActivityListener,
): () => void {
  const supabase = createSupabaseClient();
  const channelName = `activity-realtime-${crypto.randomUUID()}`;

  const channel = supabase
    .channel(channelName)
    .on(
      "postgres_changes",
      { event: "INSERT", schema: "public", table: "activity_log" },
      (payload) => {
        try {
          listener(payload.new as ActivityEvent);
        } catch (err) {
          console.error("Realtime activity listener error:", err);
        }
      },
    )
    .subscribe((status, err) => {
      const errorStates = new Set(["CHANNEL_ERROR", "TIMED_OUT"]);
      if (errorStates.has(status as string)) {
        console.error("Realtime subscription error:", status, err);
      }
    });

  return () => {
    void supabase.removeChannel(channel);
  };
}
