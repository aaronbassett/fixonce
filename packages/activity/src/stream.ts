type ActivityListener = (event: { id: string; operation: string; memory_id: string | null; details: Record<string, unknown>; created_at: string }) => void;

const listeners = new Set<ActivityListener>();

export function subscribeToActivity(listener: ActivityListener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function emitActivity(event: { id: string; operation: string; memory_id: string | null; details: Record<string, unknown>; created_at: string }): void {
  for (const listener of listeners) {
    try {
      listener(event);
    } catch (err) {
      console.error("Activity listener error:", err);
    }
  }
}
