import { useEffect, useState, useCallback } from "react";
import { useParams, useNavigate } from "react-router-dom";
import {
  getMemoryApi,
  updateMemoryApi,
  deleteMemoryApi,
  getFeedbackApi,
  submitFeedbackApi,
} from "../api/client.ts";
import type { GetMemoryResult, Feedback } from "@fixonce/shared";

export function MemoryDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [result, setResult] = useState<GetMemoryResult | null>(null);
  const [feedback, setFeedback] = useState<Feedback[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [editTitle, setEditTitle] = useState("");
  const [editContent, setEditContent] = useState("");
  const [editSummary, setEditSummary] = useState("");
  const [saving, setSaving] = useState(false);

  // Feedback form
  const [feedbackText, setFeedbackText] = useState("");
  const [feedbackTag, setFeedbackTag] = useState("");
  const [feedbackAction, setFeedbackAction] = useState("");
  const [submittingFeedback, setSubmittingFeedback] = useState(false);

  const loadMemory = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    setError(null);
    try {
      const [memResult, fb] = await Promise.all([
        getMemoryApi(id, "large"),
        getFeedbackApi(id),
      ]);
      setResult(memResult);
      setFeedback(fb);

      const m = memResult.memory;
      setEditTitle(m.title);
      setEditContent(m.content);
      setEditSummary(m.summary);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load memory");
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    void loadMemory();
  }, [loadMemory]);

  async function handleSave() {
    if (!id) return;
    setSaving(true);
    setError(null);
    try {
      await updateMemoryApi(id, {
        title: editTitle,
        content: editContent,
        summary: editSummary,
      });
      setEditing(false);
      void loadMemory();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save changes");
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete() {
    if (!id) return;
    if (!window.confirm("Permanently delete this memory? This cannot be undone.")) {
      return;
    }
    try {
      await deleteMemoryApi(id);
      navigate("/");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to delete memory");
    }
  }

  async function handleSubmitFeedback(e: React.FormEvent) {
    e.preventDefault();
    if (!id) return;
    setSubmittingFeedback(true);
    try {
      await submitFeedbackApi({
        memory_id: id,
        text: feedbackText || undefined,
        tags: feedbackTag ? [feedbackTag as Feedback["tags"][number]] : undefined,
        suggested_action: feedbackAction
          ? (feedbackAction as "keep" | "remove" | "fix")
          : undefined,
      });
      setFeedbackText("");
      setFeedbackTag("");
      setFeedbackAction("");
      const fb = await getFeedbackApi(id);
      setFeedback(fb);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to submit feedback");
    } finally {
      setSubmittingFeedback(false);
    }
  }

  if (loading) return <p>Loading memory...</p>;
  if (!result) return <p>Memory not found.</p>;

  const memory = result.memory;

  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <h1>{editing ? "Edit Memory" : memory.title}</h1>
        <div style={{ display: "flex", gap: "0.5rem" }}>
          {!editing && (
            <button onClick={() => setEditing(true)} style={btnStyle}>
              Edit
            </button>
          )}
          <button onClick={handleDelete} style={{ ...btnStyle, color: "#c00" }}>
            Delete
          </button>
        </div>
      </div>

      {error && <p style={{ color: "#c00" }}>{error}</p>}

      {editing ? (
        <div style={{ marginBottom: "2rem" }}>
          <div style={fieldRow}>
            <label htmlFor="editTitle">Title</label>
            <input
              id="editTitle"
              type="text"
              value={editTitle}
              onChange={(e) => setEditTitle(e.target.value)}
              style={inputStyle}
            />
          </div>
          <div style={fieldRow}>
            <label htmlFor="editSummary">Summary</label>
            <textarea
              id="editSummary"
              value={editSummary}
              onChange={(e) => setEditSummary(e.target.value)}
              rows={3}
              style={inputStyle}
            />
          </div>
          <div style={fieldRow}>
            <label htmlFor="editContent">Content</label>
            <textarea
              id="editContent"
              value={editContent}
              onChange={(e) => setEditContent(e.target.value)}
              rows={10}
              style={inputStyle}
            />
          </div>
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <button onClick={handleSave} disabled={saving} style={submitBtn}>
              {saving ? "Saving..." : "Save"}
            </button>
            <button onClick={() => setEditing(false)} style={btnStyle}>
              Cancel
            </button>
          </div>
        </div>
      ) : (
        <div style={{ marginBottom: "2rem" }}>
          <div style={metaRow}>
            <span>Type: {memory.memory_type}</span>
            {"tags" in memory && memory.tags.length > 0 && (
              <span>Tags: {memory.tags.join(", ")}</span>
            )}
            {"language" in memory && <span>Language: {memory.language}</span>}
            {"created_by" in memory && <span>Created by: {memory.created_by}</span>}
          </div>
          <h3>Summary</h3>
          <p>{memory.summary}</p>
          <h3>Content</h3>
          <pre style={preStyle}>{memory.content}</pre>
          {"confidence" in memory && (
            <p style={{ fontSize: "0.85rem", color: "#666" }}>
              Confidence: {memory.confidence} | Surfaced: {memory.surfaced_count} times
            </p>
          )}
        </div>
      )}

      <hr style={{ margin: "2rem 0" }} />

      <h2>Feedback ({feedback.length})</h2>

      <form onSubmit={handleSubmitFeedback} style={{ marginBottom: "1.5rem" }}>
        <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap", alignItems: "flex-end" }}>
          <div style={fieldRow}>
            <label htmlFor="fbTag">Tag</label>
            <select
              id="fbTag"
              value={feedbackTag}
              onChange={(e) => setFeedbackTag(e.target.value)}
              style={inputStyle}
            >
              <option value="">None</option>
              <option value="helpful">Helpful</option>
              <option value="not_helpful">Not helpful</option>
              <option value="damaging">Damaging</option>
              <option value="accurate">Accurate</option>
              <option value="inaccurate">Inaccurate</option>
              <option value="outdated">Outdated</option>
            </select>
          </div>
          <div style={fieldRow}>
            <label htmlFor="fbAction">Suggested Action</label>
            <select
              id="fbAction"
              value={feedbackAction}
              onChange={(e) => setFeedbackAction(e.target.value)}
              style={inputStyle}
            >
              <option value="">None</option>
              <option value="keep">Keep</option>
              <option value="fix">Fix</option>
              <option value="remove">Remove</option>
            </select>
          </div>
          <div style={{ ...fieldRow, flex: 1 }}>
            <label htmlFor="fbText">Comment</label>
            <input
              id="fbText"
              type="text"
              value={feedbackText}
              onChange={(e) => setFeedbackText(e.target.value)}
              placeholder="Optional comment..."
              style={inputStyle}
            />
          </div>
          <button type="submit" disabled={submittingFeedback} style={submitBtn}>
            {submittingFeedback ? "Submitting..." : "Submit"}
          </button>
        </div>
      </form>

      {feedback.length > 0 ? (
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              <th style={thStyle}>Tags</th>
              <th style={thStyle}>Action</th>
              <th style={thStyle}>Comment</th>
              <th style={thStyle}>Date</th>
            </tr>
          </thead>
          <tbody>
            {feedback.map((fb) => (
              <tr key={fb.id}>
                <td style={tdStyle}>{fb.tags.join(", ") || "-"}</td>
                <td style={tdStyle}>{fb.suggested_action ?? "-"}</td>
                <td style={tdStyle}>{fb.text ?? "-"}</td>
                <td style={tdStyle}>{new Date(fb.created_at).toLocaleString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <p style={{ color: "#888" }}>No feedback yet.</p>
      )}
    </div>
  );
}

const fieldRow: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "0.25rem",
  marginBottom: "0.5rem",
};

const inputStyle: React.CSSProperties = {
  padding: "0.4rem 0.5rem",
  border: "1px solid #ccc",
  borderRadius: "3px",
  fontSize: "0.9rem",
  width: "100%",
  boxSizing: "border-box",
};

const btnStyle: React.CSSProperties = {
  background: "none",
  border: "1px solid #ccc",
  borderRadius: "3px",
  padding: "0.4rem 0.75rem",
  cursor: "pointer",
  fontSize: "0.85rem",
};

const submitBtn: React.CSSProperties = {
  padding: "0.4rem 1rem",
  background: "#333",
  color: "#fff",
  border: "none",
  borderRadius: "3px",
  cursor: "pointer",
  fontSize: "0.85rem",
};

const metaRow: React.CSSProperties = {
  display: "flex",
  gap: "1.5rem",
  flexWrap: "wrap",
  fontSize: "0.85rem",
  color: "#666",
  marginBottom: "1rem",
};

const preStyle: React.CSSProperties = {
  background: "#f5f5f5",
  padding: "1rem",
  borderRadius: "4px",
  overflow: "auto",
  fontSize: "0.85rem",
  whiteSpace: "pre-wrap",
  wordBreak: "break-word",
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
