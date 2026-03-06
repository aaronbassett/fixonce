import { useState } from "react";
import { Link } from "react-router-dom";
import { fetchMemories } from "../api/client.ts";
import type { QueryMemoriesResult } from "@fixonce/shared";

export function MemoryQuery() {
  const [query, setQuery] = useState("");
  const [searchType, setSearchType] = useState("hybrid");
  const [memoryType, setMemoryType] = useState("");
  const [language, setLanguage] = useState("");
  const [verbosity, setVerbosity] = useState("medium");
  const [maxResults, setMaxResults] = useState("10");
  const [results, setResults] = useState<QueryMemoriesResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSearch(e: React.FormEvent) {
    e.preventDefault();
    if (!query.trim()) return;

    setLoading(true);
    setError(null);
    try {
      const params: Record<string, string> = {
        query,
        type: searchType,
        verbosity,
        max_results: maxResults,
      };
      if (memoryType) params.memory_type = memoryType;
      if (language) params.language = language;

      const data = await fetchMemories(params);
      setResults(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Search failed");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div>
      <h1>Search Memories</h1>

      <form onSubmit={handleSearch} style={{ marginBottom: "2rem" }}>
        <div style={fieldRow}>
          <label htmlFor="query">Query</label>
          <input
            id="query"
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search for memories..."
            style={inputStyle}
          />
        </div>

        <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap" }}>
          <div style={fieldRow}>
            <label htmlFor="searchType">Search Type</label>
            <select
              id="searchType"
              value={searchType}
              onChange={(e) => setSearchType(e.target.value)}
              style={inputStyle}
            >
              <option value="hybrid">Hybrid</option>
              <option value="vector">Vector</option>
              <option value="simple">Simple</option>
            </select>
          </div>

          <div style={fieldRow}>
            <label htmlFor="memoryType">Memory Type</label>
            <select
              id="memoryType"
              value={memoryType}
              onChange={(e) => setMemoryType(e.target.value)}
              style={inputStyle}
            >
              <option value="">All</option>
              <option value="guidance">Guidance</option>
              <option value="anti_pattern">Anti-pattern</option>
            </select>
          </div>

          <div style={fieldRow}>
            <label htmlFor="language">Language</label>
            <input
              id="language"
              type="text"
              value={language}
              onChange={(e) => setLanguage(e.target.value)}
              placeholder="e.g. typescript"
              style={inputStyle}
            />
          </div>

          <div style={fieldRow}>
            <label htmlFor="verbosity">Verbosity</label>
            <select
              id="verbosity"
              value={verbosity}
              onChange={(e) => setVerbosity(e.target.value)}
              style={inputStyle}
            >
              <option value="small">Small</option>
              <option value="medium">Medium</option>
              <option value="large">Large</option>
            </select>
          </div>

          <div style={fieldRow}>
            <label htmlFor="maxResults">Max Results</label>
            <input
              id="maxResults"
              type="number"
              value={maxResults}
              onChange={(e) => setMaxResults(e.target.value)}
              min="1"
              max="50"
              style={inputStyle}
            />
          </div>
        </div>

        <button type="submit" disabled={loading} style={submitBtn}>
          {loading ? "Searching..." : "Search"}
        </button>
      </form>

      {error && <p style={{ color: "#c00" }}>{error}</p>}

      {results && (
        <div>
          <p style={{ color: "#666", fontSize: "0.875rem" }}>
            Found {results.total_found} total | Showing {results.results.length}
            {" | "}Search: {results.pipeline.search_type}
            {results.pipeline.rewrite_used && " | Rewritten"}
            {results.pipeline.rerank_used && " | Reranked"}
          </p>

          {results.results.map((memory) => (
            <div key={memory.id} style={cardStyle}>
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                }}
              >
                <Link
                  to={`/memories/${memory.id}`}
                  style={{ fontWeight: 600, fontSize: "1.1rem" }}
                >
                  {memory.title}
                </Link>
                <span style={scoreBadge}>
                  {memory.relevancy_score.toFixed(2)}
                </span>
              </div>
              <p style={{ color: "#555", margin: "0.5rem 0" }}>
                {memory.summary}
              </p>
              <div style={{ fontSize: "0.8rem", color: "#888" }}>
                {memory.memory_type}
                {"tags" in memory &&
                  memory.tags.length > 0 &&
                  ` | ${memory.tags.join(", ")}`}
              </div>
            </div>
          ))}

          {results.overflow.length > 0 && (
            <div style={{ marginTop: "1rem" }}>
              <h3>Overflow ({results.overflow.length})</h3>
              {results.overflow.map((entry) => (
                <div
                  key={entry.id}
                  style={{ padding: "0.25rem 0", fontSize: "0.875rem" }}
                >
                  <Link to={`/memories/${entry.id}`}>{entry.title}</Link>
                  <span style={{ color: "#888", marginLeft: "0.5rem" }}>
                    ({entry.relevancy_score.toFixed(2)})
                  </span>
                </div>
              ))}
            </div>
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

const cardStyle: React.CSSProperties = {
  border: "1px solid #ddd",
  borderRadius: "4px",
  padding: "1rem",
  marginBottom: "0.75rem",
};

const scoreBadge: React.CSSProperties = {
  background: "#eee",
  padding: "0.15rem 0.5rem",
  borderRadius: "3px",
  fontSize: "0.8rem",
  fontWeight: 600,
};
