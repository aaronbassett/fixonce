import { useEffect, useState, useCallback } from "react";
import { Link } from "react-router-dom";
import { fetchMemories, getFeedbackApi } from "../api/client.ts";
import type { Feedback } from "@fixonce/shared";

interface FeedbackWithMemoryTitle extends Feedback {
  memory_title: string;
}

export function RecentFeedback() {
  const [items, setItems] = useState<FeedbackWithMemoryTitle[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filterTag, setFilterTag] = useState("");
  const [filterAction, setFilterAction] = useState("");

  const loadFeedback = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      // Get recent memories and then fetch feedback for each
      const memResult = await fetchMemories({
        query: "",
        verbosity: "small",
        max_results: "50",
      });

      const allFeedback: FeedbackWithMemoryTitle[] = [];
      for (const memory of memResult.results) {
        const fb = await getFeedbackApi(memory.id);
        for (const f of fb) {
          allFeedback.push({ ...f, memory_title: memory.title });
        }
      }

      // Sort by date descending
      allFeedback.sort(
        (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
      );

      setItems(allFeedback);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load feedback");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadFeedback();
  }, [loadFeedback]);

  const filteredItems = items.filter((fb) => {
    if (filterTag && !fb.tags.includes(filterTag as Feedback["tags"][number])) return false;
    if (filterAction && fb.suggested_action !== filterAction) return false;
    return true;
  });

  if (loading) return <p>Loading feedback...</p>;

  return (
    <div>
      <h1>Recent Feedback</h1>

      {error && <p style={{ color: "#c00" }}>{error}</p>}

      <div style={{ display: "flex", gap: "1rem", marginBottom: "1rem" }}>
        <div style={fieldRow}>
          <label htmlFor="filterTag">Filter by Tag</label>
          <select
            id="filterTag"
            value={filterTag}
            onChange={(e) => setFilterTag(e.target.value)}
            style={inputStyle}
          >
            <option value="">All</option>
            <option value="helpful">Helpful</option>
            <option value="not_helpful">Not helpful</option>
            <option value="damaging">Damaging</option>
            <option value="accurate">Accurate</option>
            <option value="inaccurate">Inaccurate</option>
            <option value="outdated">Outdated</option>
          </select>
        </div>
        <div style={fieldRow}>
          <label htmlFor="filterAction">Filter by Action</label>
          <select
            id="filterAction"
            value={filterAction}
            onChange={(e) => setFilterAction(e.target.value)}
            style={inputStyle}
          >
            <option value="">All</option>
            <option value="keep">Keep</option>
            <option value="fix">Fix</option>
            <option value="remove">Remove</option>
          </select>
        </div>
      </div>

      <p style={{ fontSize: "0.85rem", color: "#666" }}>
        Showing {filteredItems.length} of {items.length} feedback entries
      </p>

      {filteredItems.length > 0 ? (
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              <th style={thStyle}>Memory</th>
              <th style={thStyle}>Tags</th>
              <th style={thStyle}>Action</th>
              <th style={thStyle}>Comment</th>
              <th style={thStyle}>Date</th>
            </tr>
          </thead>
          <tbody>
            {filteredItems.map((fb) => (
              <tr key={fb.id}>
                <td style={tdStyle}>
                  <Link to={`/memories/${fb.memory_id}`}>{fb.memory_title}</Link>
                </td>
                <td style={tdStyle}>{fb.tags.join(", ") || "-"}</td>
                <td style={tdStyle}>
                  {fb.suggested_action ? (
                    <span
                      style={{
                        color:
                          fb.suggested_action === "remove"
                            ? "#c00"
                            : fb.suggested_action === "fix"
                              ? "#c60"
                              : "#060",
                        fontWeight: 600,
                      }}
                    >
                      {fb.suggested_action}
                    </span>
                  ) : (
                    "-"
                  )}
                </td>
                <td style={tdStyle}>{fb.text ?? "-"}</td>
                <td style={tdStyle}>{new Date(fb.created_at).toLocaleString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <p>No feedback entries found.</p>
      )}
    </div>
  );
}

const fieldRow: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "0.25rem",
};

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
