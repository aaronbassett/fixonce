import { useEffect, useState, useCallback } from "react";
import { Link } from "react-router-dom";
import { getActivityApi } from "../api/client.ts";
import { useActivityStream } from "../hooks/useActivityStream.ts";
import type { ActivityLog, OperationType } from "@fixonce/shared";
import type { ActivityEvent } from "../hooks/useActivityStream.ts";

export function RecentActivity() {
  const [logs, setLogs] = useState<ActivityLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filterOp, setFilterOp] = useState("");

  const loadLogs = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const params: Record<string, string> = { limit: "100" };
      if (filterOp) params.operation = filterOp;
      const data = await getActivityApi(params);
      setLogs(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load activity");
    } finally {
      setLoading(false);
    }
  }, [filterOp]);

  useEffect(() => {
    void loadLogs();
  }, [loadLogs]);

  const handleStreamEvent = useCallback((event: ActivityEvent) => {
    const newLog: ActivityLog = {
      id: event.id,
      operation: event.operation as OperationType,
      memory_id: event.memory_id,
      details: event.details,
      created_at: event.created_at,
    };
    setLogs((prev) => [newLog, ...prev]);
  }, []);

  const { connected } = useActivityStream(handleStreamEvent);

  return (
    <div>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
        }}
      >
        <h1>Activity Log</h1>
        <span
          style={{
            fontSize: "0.8rem",
            padding: "0.2rem 0.5rem",
            borderRadius: "3px",
            background: connected ? "#e8f5e9" : "#fce4ec",
            color: connected ? "#2e7d32" : "#c62828",
          }}
        >
          {connected ? "Live" : "Disconnected"}
        </span>
      </div>

      {error && <p style={{ color: "#c00" }}>{error}</p>}

      <div style={{ marginBottom: "1rem" }}>
        <label htmlFor="filterOp" style={{ marginRight: "0.5rem" }}>
          Filter by operation:
        </label>
        <select
          id="filterOp"
          value={filterOp}
          onChange={(e) => setFilterOp(e.target.value)}
          style={inputStyle}
        >
          <option value="">All</option>
          <option value="query">Query</option>
          <option value="create">Create</option>
          <option value="update">Update</option>
          <option value="feedback">Feedback</option>
          <option value="detect">Detect</option>
        </select>
      </div>

      {loading ? (
        <p>Loading activity...</p>
      ) : logs.length > 0 ? (
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              <th style={thStyle}>Operation</th>
              <th style={thStyle}>Memory</th>
              <th style={thStyle}>Details</th>
              <th style={thStyle}>Time</th>
            </tr>
          </thead>
          <tbody>
            {logs.map((log) => (
              <tr key={log.id}>
                <td style={tdStyle}>
                  <span style={opBadge(log.operation)}>{log.operation}</span>
                </td>
                <td style={tdStyle}>
                  {log.memory_id ? (
                    <Link to={`/memories/${log.memory_id}`}>
                      {log.memory_id.slice(0, 8)}...
                    </Link>
                  ) : (
                    "-"
                  )}
                </td>
                <td style={tdStyle}>
                  <code style={{ fontSize: "0.8rem" }}>
                    {JSON.stringify(log.details, null, 0).slice(0, 100)}
                  </code>
                </td>
                <td style={tdStyle}>
                  {new Date(log.created_at).toLocaleString()}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <p>No activity logged yet.</p>
      )}
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  padding: "0.4rem 0.5rem",
  border: "1px solid #ccc",
  borderRadius: "3px",
  fontSize: "0.9rem",
};

const thStyle: React.CSSProperties = {
  textAlign: "left",
  padding: "0.5rem",
  borderBottom: "2px solid #ddd",
};

const tdStyle: React.CSSProperties = {
  padding: "0.5rem",
  borderBottom: "1px solid #eee",
};

const OP_COLORS: Record<string, string> = {
  query: "#1565c0",
  create: "#2e7d32",
  update: "#ef6c00",
  feedback: "#6a1b9a",
  detect: "#00838f",
};

function opBadge(operation: string): React.CSSProperties {
  return {
    display: "inline-block",
    padding: "0.15rem 0.5rem",
    borderRadius: "3px",
    fontSize: "0.75rem",
    fontWeight: 600,
    color: "#fff",
    background: OP_COLORS[operation] ?? "#666",
  };
}
