import { useEffect, useRef, useState } from "react";

export interface ActivityEvent {
  id: string;
  operation: string;
  memory_id: string | null;
  details: Record<string, unknown>;
  created_at: string;
}

export function useActivityStream(onEvent: (event: ActivityEvent) => void): {
  connected: boolean;
} {
  const [connected, setConnected] = useState(false);
  const onEventRef = useRef(onEvent);
  onEventRef.current = onEvent;

  useEffect(() => {
    let es: EventSource | null = null;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

    function connect() {
      es = new EventSource("/api/activity/stream");

      es.onopen = () => {
        setConnected(true);
      };

      es.onmessage = (event) => {
        try {
          const parsed = JSON.parse(event.data) as ActivityEvent;
          onEventRef.current(parsed);
        } catch {
          // ignore malformed events
        }
      };

      es.onerror = () => {
        setConnected(false);
        es?.close();
        reconnectTimer = setTimeout(connect, 3000);
      };
    }

    connect();

    return () => {
      es?.close();
      if (reconnectTimer) clearTimeout(reconnectTimer);
    };
  }, []);

  return { connected };
}
