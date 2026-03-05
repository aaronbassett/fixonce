import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { createMemoryApi, previewDuplicatesApi } from "../api/client.ts";

export function CreateMemory() {
  const navigate = useNavigate();
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [summary, setSummary] = useState("");
  const [memoryType, setMemoryType] = useState("guidance");
  const [sourceType, setSourceType] = useState("instruction");
  const [language, setLanguage] = useState("typescript");
  const [tags, setTags] = useState("");
  const [sourceUrl, setSourceUrl] = useState("");
  const [projectName, setProjectName] = useState("");
  const [confidence, setConfidence] = useState("0.5");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Duplicate preview
  const [dupPreview, setDupPreview] = useState<{
    outcome: string;
    reason: string;
    target_memory_id?: string;
  } | null>(null);
  const [checkingDupes, setCheckingDupes] = useState(false);

  async function handleCheckDuplicates() {
    if (!title.trim() || !content.trim()) return;
    setCheckingDupes(true);
    setDupPreview(null);
    try {
      const result = await previewDuplicatesApi({
        title,
        content,
        summary: summary || title,
        language,
      });
      setDupPreview(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Duplicate check failed");
    } finally {
      setCheckingDupes(false);
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError(null);
    try {
      const result = await createMemoryApi({
        title,
        content,
        summary: summary || title,
        memory_type: memoryType as "guidance" | "anti_pattern",
        source_type: sourceType as "correction" | "discovery" | "instruction",
        language,
        tags: tags ? tags.split(",").map((t) => t.trim()).filter(Boolean) : undefined,
        source_url: sourceUrl || undefined,
        project_name: projectName || undefined,
        confidence: Number(confidence),
      });

      if (result.memory?.id) {
        navigate(`/memories/${result.memory.id}`);
      } else {
        navigate("/");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create memory");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div>
      <h1>Create Memory</h1>

      {error && <p style={{ color: "#c00" }}>{error}</p>}

      <form onSubmit={handleSubmit}>
        <div style={fieldRow}>
          <label htmlFor="title">Title *</label>
          <input
            id="title"
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            required
            style={inputStyle}
          />
        </div>

        <div style={fieldRow}>
          <label htmlFor="summary">Summary</label>
          <textarea
            id="summary"
            value={summary}
            onChange={(e) => setSummary(e.target.value)}
            rows={3}
            placeholder="Defaults to title if empty"
            style={inputStyle}
          />
        </div>

        <div style={fieldRow}>
          <label htmlFor="content">Content *</label>
          <textarea
            id="content"
            value={content}
            onChange={(e) => setContent(e.target.value)}
            rows={10}
            required
            style={inputStyle}
          />
        </div>

        <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap" }}>
          <div style={fieldRow}>
            <label htmlFor="memoryType">Memory Type</label>
            <select
              id="memoryType"
              value={memoryType}
              onChange={(e) => setMemoryType(e.target.value)}
              style={inputStyle}
            >
              <option value="guidance">Guidance</option>
              <option value="anti_pattern">Anti-pattern</option>
            </select>
          </div>

          <div style={fieldRow}>
            <label htmlFor="sourceType">Source Type</label>
            <select
              id="sourceType"
              value={sourceType}
              onChange={(e) => setSourceType(e.target.value)}
              style={inputStyle}
            >
              <option value="instruction">Instruction</option>
              <option value="correction">Correction</option>
              <option value="discovery">Discovery</option>
            </select>
          </div>

          <div style={fieldRow}>
            <label htmlFor="language">Language</label>
            <input
              id="language"
              type="text"
              value={language}
              onChange={(e) => setLanguage(e.target.value)}
              style={inputStyle}
            />
          </div>

          <div style={fieldRow}>
            <label htmlFor="confidence">Confidence</label>
            <input
              id="confidence"
              type="number"
              value={confidence}
              onChange={(e) => setConfidence(e.target.value)}
              min="0"
              max="1"
              step="0.1"
              style={inputStyle}
            />
          </div>
        </div>

        <div style={fieldRow}>
          <label htmlFor="tags">Tags (comma-separated)</label>
          <input
            id="tags"
            type="text"
            value={tags}
            onChange={(e) => setTags(e.target.value)}
            placeholder="react, hooks, state"
            style={inputStyle}
          />
        </div>

        <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap" }}>
          <div style={{ ...fieldRow, flex: 1 }}>
            <label htmlFor="sourceUrl">Source URL</label>
            <input
              id="sourceUrl"
              type="url"
              value={sourceUrl}
              onChange={(e) => setSourceUrl(e.target.value)}
              style={inputStyle}
            />
          </div>

          <div style={{ ...fieldRow, flex: 1 }}>
            <label htmlFor="projectName">Project Name</label>
            <input
              id="projectName"
              type="text"
              value={projectName}
              onChange={(e) => setProjectName(e.target.value)}
              style={inputStyle}
            />
          </div>
        </div>

        <div style={{ display: "flex", gap: "0.75rem", marginTop: "1rem" }}>
          <button type="submit" disabled={saving} style={submitBtn}>
            {saving ? "Creating..." : "Create Memory"}
          </button>
          <button
            type="button"
            onClick={handleCheckDuplicates}
            disabled={checkingDupes || !title.trim() || !content.trim()}
            style={secondaryBtn}
          >
            {checkingDupes ? "Checking..." : "Check Duplicates"}
          </button>
        </div>
      </form>

      {dupPreview && (
        <div
          style={{
            marginTop: "1rem",
            padding: "0.75rem",
            border: "1px solid #ddd",
            borderRadius: "4px",
            background: dupPreview.outcome === "new" ? "#e8f5e9" : "#fff3e0",
          }}
        >
          <strong>Duplicate Check:</strong> {dupPreview.outcome}
          <p style={{ margin: "0.25rem 0 0", fontSize: "0.85rem" }}>{dupPreview.reason}</p>
          {dupPreview.target_memory_id && (
            <p style={{ margin: "0.25rem 0 0", fontSize: "0.85rem" }}>
              Related memory: {dupPreview.target_memory_id}
            </p>
          )}
        </div>
      )}
    </div>
  );
}

const fieldRow: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "0.25rem",
  marginBottom: "0.75rem",
};

const inputStyle: React.CSSProperties = {
  padding: "0.4rem 0.5rem",
  border: "1px solid #ccc",
  borderRadius: "3px",
  fontSize: "0.9rem",
  width: "100%",
  boxSizing: "border-box",
};

const submitBtn: React.CSSProperties = {
  padding: "0.5rem 1.5rem",
  background: "#333",
  color: "#fff",
  border: "none",
  borderRadius: "3px",
  cursor: "pointer",
  fontSize: "0.9rem",
};

const secondaryBtn: React.CSSProperties = {
  padding: "0.5rem 1.5rem",
  background: "#fff",
  color: "#333",
  border: "1px solid #ccc",
  borderRadius: "3px",
  cursor: "pointer",
  fontSize: "0.9rem",
};
