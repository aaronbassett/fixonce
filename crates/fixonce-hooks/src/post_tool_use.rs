//! Post-tool-use hook: checks written content against anti-memory patterns.
//!
//! Runs after a tool has executed. If the tool output closely matches a known
//! anti-memory (similarity > 0.5), an advisory is returned. Otherwise `None`
//! is returned.
//!
//! The threshold is lower than [`pre_tool_use`](crate::pre_tool_use) (0.5 vs
//! 0.7) because post-hoc review is less disruptive — the action has already
//! happened and the advisory is informational.
//!
//! This hook is **warn-only**: the shell wrapper always exits 0 so the agent
//! is never blocked.

use fixonce_core::{
    api::{search::search_memories, ApiClient},
    memory::types::{MemoryType, SearchMemoryRequest},
};

use crate::HookError;

/// Score above which a matching anti-memory triggers an advisory.
const POST_TOOL_ADVISE_THRESHOLD: f64 = 0.5;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Called after a tool is used.
///
/// 1. Searches for anti-memories that match the tool output.
/// 2. Returns an advisory message when any match scores above 0.5.
/// 3. Returns `None` when the output looks safe.
///
/// # Errors
///
/// Returns [`HookError::Unauthenticated`] when no token is stored or the
/// token has expired (EC-43).
/// Returns [`HookError::Api`] on network failure.
pub async fn on_post_tool_use(
    api_url: &str,
    tool_output: &str,
) -> Result<Option<String>, HookError> {
    // EC-43: load token; skip silently when absent or expired.
    let token = crate::load_valid_token()?;

    let client = ApiClient::new(api_url)
        .map_err(HookError::Api)?
        .with_token(&token);

    // Truncate very long outputs so the search stays lightweight.
    let query = truncate(tool_output, 512);

    if query.is_empty() {
        return Ok(None);
    }

    let req = SearchMemoryRequest {
        query,
        limit: Some(5),
        threshold: Some(POST_TOOL_ADVISE_THRESHOLD),
        language: None,
    };

    let resp = search_memories(&client, &req)
        .await
        .map_err(HookError::Api)?;

    // Filter to anti-pattern memories that exceed the threshold.
    let advisories: Vec<_> = resp
        .hits
        .into_iter()
        .filter(|h| {
            h.memory.memory_type == MemoryType::AntiPattern
                && h.similarity >= POST_TOOL_ADVISE_THRESHOLD
        })
        .collect();

    if advisories.is_empty() {
        return Ok(None);
    }

    Ok(Some(format_post_tool_advisory(&advisories)))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Truncate `s` to at most `max_len` bytes (on a character boundary).
pub(crate) fn truncate(s: &str, max_len: usize) -> String {
    let trimmed = s.trim();
    if trimmed.len() <= max_len {
        trimmed.to_owned()
    } else {
        let mut end = max_len;
        while !trimmed.is_char_boundary(end) {
            end -= 1;
        }
        trimmed[..end].to_owned()
    }
}

/// Format advisory messages for post-tool anti-memory matches.
fn format_post_tool_advisory(hits: &[fixonce_core::memory::types::SearchHit]) -> String {
    use std::fmt::Write as _;

    let mut out =
        String::from("[FixOnce] Advisory — written content resembles known anti-pattern(s):\n");

    for hit in hits {
        let _ = writeln!(
            out,
            "- [{:.2}] {}: {}",
            hit.similarity, hit.memory.title, hit.memory.summary,
        );
        if let Some(ref am) = hit.memory.anti_memory {
            let _ = writeln!(out, "  Issue: {}", am.reason);
            if let Some(ref alt) = am.alternative {
                let _ = writeln!(out, "  Consider: {alt}");
            }
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use fixonce_core::memory::types::{
        AntiMemory, EmbeddingStatus, Memory, MemoryType, PipelineStatus, SearchHit, SourceType,
    };

    use super::*;

    fn make_anti_hit(id: &str, similarity: f64) -> SearchHit {
        SearchHit {
            memory: Memory {
                id: id.to_owned(),
                title: format!("AntiPattern {id}"),
                content: "bad pattern".to_owned(),
                summary: format!("Do not do {id}"),
                memory_type: MemoryType::AntiPattern,
                source_type: SourceType::Manual,
                language: None,
                compact_pragma: None,
                compact_compiler: None,
                midnight_js: None,
                indexer_version: None,
                node_version: None,
                source_url: None,
                repo_url: None,
                task_summary: None,
                session_id: None,
                decay_score: 0.9,
                reinforcement_score: 0.9,
                last_accessed_at: None,
                embedding_status: EmbeddingStatus::Complete,
                pipeline_status: PipelineStatus::Complete,
                deleted_at: None,
                created_at: "2026-01-01T00:00:00Z".to_owned(),
                updated_at: "2026-01-01T00:00:00Z".to_owned(),
                created_by: "user-1".to_owned(),
                anti_memory: Some(AntiMemory {
                    description: format!("Bad thing {id}"),
                    reason: "It introduces a regression".to_owned(),
                    alternative: Some("Use the safe path".to_owned()),
                    version_constraints: None,
                }),
            },
            similarity,
        }
    }

    // --- truncate ---

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("short", 512), "short");
    }

    #[test]
    fn truncate_long_string_limited() {
        let s = "y".repeat(800);
        assert_eq!(truncate(&s, 512).len(), 512);
    }

    // --- format_post_tool_advisory ---

    #[test]
    fn advisory_includes_issue_and_alternative() {
        let hits = vec![make_anti_hit("thing", 0.55)];
        let msg = format_post_tool_advisory(&hits);
        assert!(msg.contains("Advisory"));
        assert!(msg.contains("regression"));
        assert!(msg.contains("Use the safe path"));
    }

    // --- threshold filtering (unit) ---

    #[test]
    fn below_threshold_would_not_advise() {
        let hits = vec![make_anti_hit("noop", 0.45)];
        let advisories: Vec<_> = hits
            .into_iter()
            .filter(|h| {
                h.memory.memory_type == MemoryType::AntiPattern
                    && h.similarity >= POST_TOOL_ADVISE_THRESHOLD
            })
            .collect();
        assert!(advisories.is_empty());
    }

    #[test]
    fn at_threshold_generates_advisory() {
        let hits = vec![make_anti_hit("exact", POST_TOOL_ADVISE_THRESHOLD)];
        let advisories: Vec<_> = hits
            .into_iter()
            .filter(|h| {
                h.memory.memory_type == MemoryType::AntiPattern
                    && h.similarity >= POST_TOOL_ADVISE_THRESHOLD
            })
            .collect();
        assert_eq!(advisories.len(), 1);
    }

    #[test]
    fn lower_threshold_than_pre_tool_use() {
        // Verify the architectural invariant: post < pre.
        assert!(
            POST_TOOL_ADVISE_THRESHOLD < crate::pre_tool_use::PRE_TOOL_WARN_THRESHOLD,
            "post-tool threshold must be lower than pre-tool threshold"
        );
    }
}
