//! Embedding-based deduplication with Claude comparison.
//!
//! The stage runs in two steps:
//!
//! 1. Search for the top-5 most similar memories by cosine similarity.
//! 2. If any candidates are found, ask Claude to compare them with the
//!    incoming memory and return a structured verdict.
//!
//! Five outcomes are possible:
//!
//! | Outcome | Meaning |
//! |---------|---------|
//! | [`DedupOutcome::New`] | No duplicate found — store as a new memory. |
//! | [`DedupOutcome::Discard`] | The new memory is already covered; drop it. |
//! | [`DedupOutcome::Replace`] | Replace the existing memory with the new one. |
//! | [`DedupOutcome::Update`] | Merge the new information into an existing memory. |
//! | [`DedupOutcome::Merge`] | Semantically fuse both memories. |
//!
//! Edge case EC-27: when the search returns no candidates the outcome is always
//! [`DedupOutcome::New`] and Claude is not consulted.

use crate::{
    api::{memories::search_memories, ApiClient},
    memory::types::{CreateMemoryRequest, SearchMemoryRequest},
    pipeline::{claude::ClaudeClient, PipelineError},
};

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// One of the five actions the dedup stage can recommend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DedupOutcome {
    /// No duplicate detected — store as a new memory.
    New,
    /// The incoming memory is already covered by an existing one; discard it.
    Discard,
    /// Overwrite the identified memory with the new content.
    /// The `String` is the UUID of the memory to replace.
    Replace(String),
    /// Update (extend / refine) the identified memory with the new content.
    /// The `String` is the UUID of the memory to update.
    Update(String),
    /// Merge both memories into a single enriched record.
    /// The `String` is the UUID of the memory to merge with.
    Merge(String),
}

/// The full result returned by the dedup stage.
#[derive(Debug, Clone)]
pub struct DedupResult {
    /// What action should be taken.
    pub outcome: DedupOutcome,
    /// Human-readable explanation from Claude (or a fixed string for `New`).
    pub rationale: String,
}

// ---------------------------------------------------------------------------
// JSON response shape expected from Claude
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct ClaudeDedupResponse {
    /// One of: "new", "discard", "replace", "update", "merge"
    outcome: String,
    /// UUID of the existing memory to act on (required for replace/update/merge).
    existing_id: Option<String>,
    rationale: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run the dedup check for `new_memory`.
///
/// `embedding` must be the pre-computed embedding vector for the new memory's
/// content.
///
/// # Errors
///
/// Returns [`PipelineError::Api`] if the similarity search fails, or the
/// appropriate `Claude*` error if the LLM comparison fails.
pub async fn dedup_check(
    claude: &ClaudeClient,
    api_client: &ApiClient,
    new_memory: &CreateMemoryRequest,
    embedding: &[f64],
) -> Result<DedupResult, PipelineError> {
    // 1. Search for top-5 similar memories.
    let search_req = SearchMemoryRequest {
        query: new_memory.content.clone(),
        limit: Some(5),
        threshold: Some(0.75),
        language: new_memory.language.clone(),
    };

    // We delegate to the embedding search; the raw embedding is not directly
    // used by the API call here (the API re-embeds the query text), but it is
    // accepted so callers can pass what they already have.
    let _ = embedding; // acknowledged — used implicitly via query text

    let search_result = search_memories(api_client, &search_req)
        .await
        .map_err(|e| PipelineError::Api(e.to_string()))?;

    // EC-27: no candidates → always "new", skip Claude entirely.
    if search_result.hits.is_empty() {
        return Ok(DedupResult {
            outcome: DedupOutcome::New,
            rationale: "No similar memories found in the knowledge base.".to_owned(),
        });
    }

    // 2. Ask Claude to compare.
    let prompt = build_dedup_prompt(new_memory, &search_result.hits);
    let raw = claude.prompt(&prompt).await?;
    let json_str = strip_code_fence(&raw);

    let parsed: ClaudeDedupResponse = serde_json::from_str(json_str).map_err(|e| {
        PipelineError::ClaudeOutputParse(format!(
            "dedup response parse failure: {e} — raw: {json_str}"
        ))
    })?;

    let outcome = parse_outcome(&parsed.outcome, parsed.existing_id)?;

    Ok(DedupResult {
        outcome,
        rationale: parsed.rationale,
    })
}

// ---------------------------------------------------------------------------
// Prompt construction (public for testing)
// ---------------------------------------------------------------------------

/// Build the dedup comparison prompt.
///
/// Exposed for unit testing so we can assert on its shape without a live
/// Claude process.
#[must_use]
pub fn build_dedup_prompt(
    new_memory: &CreateMemoryRequest,
    candidates: &[crate::memory::types::SearchHit],
) -> String {
    let candidates_text: String = candidates
        .iter()
        .enumerate()
        .map(|(i, hit)| {
            format!(
                "### Candidate {} (id: {})\nTitle: {}\nSummary: {}\nContent:\n{}\nSimilarity: {:.3}\n",
                i + 1,
                hit.memory.id,
                hit.memory.title,
                hit.memory.summary,
                hit.memory.content,
                hit.similarity,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"You are a deduplication assistant for FixOnce, a developer knowledge base.

Compare the **incoming memory** below with the **existing candidates** and decide
what action to take.

## Incoming memory

Title: {title}
Summary: {summary}
Content:
{content}

## Existing candidates

{candidates_text}

## Actions (pick exactly one)

| Action | When to use |
|--------|-------------|
| `new` | The incoming memory covers genuinely new ground not found in any candidate. |
| `discard` | The incoming memory is already fully covered by a candidate; no new information. |
| `replace` | The incoming memory is strictly better / more up-to-date than a candidate. |
| `update` | The incoming memory adds incremental detail to a candidate but should be merged in. |
| `merge` | Both memories cover complementary aspects of the same topic and should be combined. |

For `replace`, `update`, and `merge` you **must** supply the UUID of the existing
memory to act on in `existing_id`.

## Output format

Reply with **only** valid JSON, no prose:

```json
{{
  "outcome": "<new|discard|replace|update|merge>",
  "existing_id": "<uuid or null>",
  "rationale": "<one sentence>"
}}
```"#,
        title = new_memory.title,
        summary = new_memory.summary,
        content = new_memory.content,
        candidates_text = candidates_text,
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_outcome(
    outcome: &str,
    existing_id: Option<String>,
) -> Result<DedupOutcome, PipelineError> {
    match outcome {
        "new" => Ok(DedupOutcome::New),
        "discard" => Ok(DedupOutcome::Discard),
        "replace" => {
            let id = existing_id.ok_or_else(|| {
                PipelineError::ClaudeOutputParse(
                    "dedup: 'replace' outcome requires existing_id".to_owned(),
                )
            })?;
            Ok(DedupOutcome::Replace(id))
        }
        "update" => {
            let id = existing_id.ok_or_else(|| {
                PipelineError::ClaudeOutputParse(
                    "dedup: 'update' outcome requires existing_id".to_owned(),
                )
            })?;
            Ok(DedupOutcome::Update(id))
        }
        "merge" => {
            let id = existing_id.ok_or_else(|| {
                PipelineError::ClaudeOutputParse(
                    "dedup: 'merge' outcome requires existing_id".to_owned(),
                )
            })?;
            Ok(DedupOutcome::Merge(id))
        }
        other => Err(PipelineError::ClaudeOutputParse(format!(
            "dedup: unknown outcome '{other}'"
        ))),
    }
}

fn strip_code_fence(s: &str) -> &str {
    let trimmed = s.trim();
    let inner = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(str::trim_start)
        .and_then(|s| s.strip_suffix("```"))
        .map(str::trim);
    inner.unwrap_or(trimmed)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Outcome parsing ---

    #[test]
    fn parses_new_outcome() {
        let o = parse_outcome("new", None).unwrap();
        assert_eq!(o, DedupOutcome::New);
    }

    #[test]
    fn parses_discard_outcome() {
        let o = parse_outcome("discard", None).unwrap();
        assert_eq!(o, DedupOutcome::Discard);
    }

    #[test]
    fn parses_replace_outcome() {
        let o = parse_outcome("replace", Some("abc-123".to_owned())).unwrap();
        assert_eq!(o, DedupOutcome::Replace("abc-123".to_owned()));
    }

    #[test]
    fn parses_update_outcome() {
        let o = parse_outcome("update", Some("def-456".to_owned())).unwrap();
        assert_eq!(o, DedupOutcome::Update("def-456".to_owned()));
    }

    #[test]
    fn parses_merge_outcome() {
        let o = parse_outcome("merge", Some("ghi-789".to_owned())).unwrap();
        assert_eq!(o, DedupOutcome::Merge("ghi-789".to_owned()));
    }

    #[test]
    fn replace_without_id_is_error() {
        let err = parse_outcome("replace", None).unwrap_err();
        assert!(matches!(err, PipelineError::ClaudeOutputParse(_)));
    }

    #[test]
    fn update_without_id_is_error() {
        let err = parse_outcome("update", None).unwrap_err();
        assert!(matches!(err, PipelineError::ClaudeOutputParse(_)));
    }

    #[test]
    fn merge_without_id_is_error() {
        let err = parse_outcome("merge", None).unwrap_err();
        assert!(matches!(err, PipelineError::ClaudeOutputParse(_)));
    }

    #[test]
    fn unknown_outcome_is_error() {
        let err = parse_outcome("frobnicate", None).unwrap_err();
        assert!(matches!(err, PipelineError::ClaudeOutputParse(_)));
    }

    // --- Code fence stripping ---

    #[test]
    fn strips_json_fence() {
        let fenced = "```json\n{\"outcome\":\"new\"}\n```";
        assert_eq!(strip_code_fence(fenced), "{\"outcome\":\"new\"}");
    }

    // --- Prompt construction ---

    #[test]
    fn dedup_prompt_contains_incoming_title() {
        let req = CreateMemoryRequest {
            title: "Unique Title XYZ".to_owned(),
            content: "body".to_owned(),
            summary: "sum".to_owned(),
            memory_type: crate::memory::types::MemoryType::Gotcha,
            source_type: crate::memory::types::SourceType::Manual,
            language: None,
            embedding: None,
            compact_pragma: None,
            compact_compiler: None,
            midnight_js: None,
            indexer_version: None,
            node_version: None,
            source_url: None,
            repo_url: None,
            task_summary: None,
            session_id: None,
        };
        let prompt = build_dedup_prompt(&req, &[]);
        assert!(
            prompt.contains("Unique Title XYZ"),
            "prompt must include the incoming title"
        );
    }

    #[test]
    fn dedup_prompt_lists_all_five_outcomes() {
        let req = CreateMemoryRequest {
            title: "t".to_owned(),
            content: "c".to_owned(),
            summary: "s".to_owned(),
            memory_type: crate::memory::types::MemoryType::Discovery,
            source_type: crate::memory::types::SourceType::Observation,
            language: None,
            embedding: None,
            compact_pragma: None,
            compact_compiler: None,
            midnight_js: None,
            indexer_version: None,
            node_version: None,
            source_url: None,
            repo_url: None,
            task_summary: None,
            session_id: None,
        };
        let prompt = build_dedup_prompt(&req, &[]);
        for outcome in &["new", "discard", "replace", "update", "merge"] {
            assert!(
                prompt.contains(outcome),
                "prompt must list outcome '{outcome}'"
            );
        }
    }

    // --- Full response JSON parsing ---

    #[test]
    fn full_json_response_parses() {
        let raw =
            r#"{"outcome":"replace","existing_id":"uuid-001","rationale":"Strictly better."}"#;
        let parsed: ClaudeDedupResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.outcome, "replace");
        assert_eq!(parsed.existing_id.as_deref(), Some("uuid-001"));
    }

    #[test]
    fn full_json_response_null_existing_id() {
        let raw = r#"{"outcome":"new","existing_id":null,"rationale":"No match."}"#;
        let parsed: ClaudeDedupResponse = serde_json::from_str(raw).unwrap();
        assert!(parsed.existing_id.is_none());
    }
}
