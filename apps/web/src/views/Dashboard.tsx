import { useEffect, useState, useCallback } from "react";
import { Link } from "react-router-dom";
import {
  fetchMemories,
  deleteMemoryApi,
  updateMemoryApi,
} from "../api/client.ts";
import type { QueryMemoriesResult } from "@fixonce/shared";

export function Dashboard() {
  const [stats, setStats] = useState<{
    total: number;
    guidance: number;
    antiPattern: number;
  } | null>(null);
  const [flaggedResults, setFlaggedResults] =
    useState<QueryMemoriesResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [allResults, guidanceResults, antiPatternResults] =
        await Promise.all([
          fetchMemories({ query: "", verbosity: "small", max_results: "1" }),
          fetchMemories({
            query: "",
            memory_type: "guidance",
            verbosity: "small",
            max_results: "1",
          }),
          fetchMemories({
            query: "",
            memory_type: "anti_pattern",
            verbosity: "small",
            max_results: "1",
          }),
        ]);

      setStats({
        total: allResults.total_found,
        guidance: guidanceResults.total_found,
        antiPattern: antiPatternResults.total_found,
      });

      const flagged = await fetchMemories({
        query: "",
        verbosity: "medium",
        max_results: "20",
      });
      setFlaggedResults(flagged);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to load dashboard data",
      );
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  async function handleDisable(id: string) {
    try {
      await updateMemoryApi(id, { enabled: false });
      void loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to disable memory");
    }
  }

  async function handleDelete(id: string) {
    if (
      !window.confirm("Permanently delete this memory? This cannot be undone.")
    ) {
      return;
    }
    try {
      await deleteMemoryApi(id);
      void loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to delete memory");
    }
  }

  if (loading) {
    return <p>Loading dashboard...</p>;
  }

  return (
    <div>
      <h1>Dashboard</h1>

      {error && <p style={{ color: "#c00" }}>{error}</p>}

      {stats && (
        <div style={{ display: "flex", gap: "1rem", marginBottom: "2rem" }}>
          <StatCard label="Total Memories" value={stats.total} />
          <StatCard label="Guidance" value={stats.guidance} />
          <StatCard label="Anti-patterns" value={stats.antiPattern} />
        </div>
      )}

      <h2>Recent Memories</h2>
      {flaggedResults && flaggedResults.results.length > 0 ? (
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              <th style={thStyle}>Title</th>
              <th style={thStyle}>Type</th>
              <th style={thStyle}>Score</th>
              <th style={thStyle}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {flaggedResults.results.map((m) => (
              <tr key={m.id}>
                <td style={tdStyle}>
                  <Link to={`/memories/${m.id}`}>{m.title}</Link>
                </td>
                <td style={tdStyle}>{m.memory_type}</td>
                <td style={tdStyle}>{m.relevancy_score.toFixed(2)}</td>
                <td style={tdStyle}>
                  <button onClick={() => handleDisable(m.id)} style={btnStyle}>
                    Disable
                  </button>
                  <button
                    onClick={() => handleDelete(m.id)}
                    style={{ ...btnStyle, color: "#c00" }}
                  >
                    Delete
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <p>No memories found.</p>
      )}
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: number }) {
  return (
    <div
      style={{
        border: "1px solid #ddd",
        borderRadius: "4px",
        padding: "1rem 1.5rem",
        minWidth: "150px",
        textAlign: "center",
      }}
    >
      <div style={{ fontSize: "2rem", fontWeight: 700 }}>{value}</div>
      <div style={{ color: "#666", fontSize: "0.875rem" }}>{label}</div>
    </div>
  );
}

const thStyle: React.CSSProperties = {
  textAlign: "left",
  padding: "0.5rem",
  borderBottom: "2px solid #ddd",
};

const tdStyle: React.CSSProperties = {
  padding: "0.5rem",
  borderBottom: "1px solid #eee",
};

const btnStyle: React.CSSProperties = {
  background: "none",
  border: "1px solid #ccc",
  borderRadius: "3px",
  padding: "0.25rem 0.5rem",
  cursor: "pointer",
  marginRight: "0.25rem",
  fontSize: "0.8rem",
};
