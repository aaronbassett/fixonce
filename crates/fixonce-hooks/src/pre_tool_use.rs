//! Pre-tool-use hook: checks proposed tool input against anti-memory patterns.
//!
//! Runs before a tool is executed. If the proposed content closely matches
//! a known anti-memory (similarity > 0.7), a warning is returned. Otherwise
//! `None` is returned and the tool proceeds normally.
//!
//! This hook is **warn-only**: the shell wrapper always exits 0 so the agent
//! is never blocked.

use fixonce_core::{
    api::{search::search_memories, ApiClient},
    auth::token::TokenManager,
    memory::types::{MemoryType, SearchMemoryRequest},
};

use crate::HookError;

/// Score above which a matching anti-memory triggers a warning.
pub(crate) const PRE_TOOL_WARN_THRESHOLD: f64 = 0.7;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Called before a tool is used.
///
/// 1. Searches for anti-memories that match the tool input.
/// 2. Returns a warning message when any match scores above 0.7.
/// 3. Returns `None` when the tool input looks safe.
///
/// # Errors
///
/// Returns [`HookError::Unauthenticated`] when no token is stored (EC-43).
/// Returns [`HookError::Api`] on network failure.
pub async fn on_pre_tool_use(api_url: &str, tool_input: &str) -> Result<Option<String>, HookError> {
    // EC-43: load token; skip silently when absent.
    let token = load_token()?;

    let client = ApiClient::new(api_url)
        .map_err(HookError::Api)?
        .with_token(&token);

    // Truncate very long inputs so the search stays lightweight.
    let query = truncate(tool_input, 512);

    if query.is_empty() {
        return Ok(None);
    }

    let req = SearchMemoryRequest {
        query,
        limit: Some(5),
        threshold: Some(PRE_TOOL_WARN_THRESHOLD),
        language: None,
    };

    let resp = search_memories(&client, &req)
        .await
        .map_err(HookError::Api)?;

    // Filter to anti-pattern memories that exceed the threshold.
    let warnings: Vec<_> = resp
        .hits
        .into_iter()
        .filter(|h| {
            h.memory.memory_type == MemoryType::AntiPattern
                && h.similarity >= PRE_TOOL_WARN_THRESHOLD
        })
        .collect();

    if warnings.is_empty() {
        return Ok(None);
    }

    Ok(Some(format_pre_tool_warning(&warnings)))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Load the stored JWT, mapping missing token to [`HookError::Unauthenticated`].
fn load_token() -> Result<String, HookError> {
    let mgr = TokenManager::new();
    match mgr.load_token().map_err(HookError::Auth)? {
        Some(t) => Ok(t),
        None => Err(HookError::Unauthenticated),
    }
}

/// Truncate `s` to at most `max_len` bytes (on a character boundary).
pub(crate) fn truncate(s: &str, max_len: usize) -> String {
    let trimmed = s.trim();
    if trimmed.len() <= max_len {
        trimmed.to_owned()
    } else {
        // Walk back to a character boundary.
        let mut end = max_len;
        while !trimmed.is_char_boundary(end) {
            end -= 1;
        }
        trimmed[..end].to_owned()
    }
}

/// Format a list of warning anti-memory hits into a human-readable advisory.
fn format_pre_tool_warning(hits: &[fixonce_core::memory::types::SearchHit]) -> String {
    use std::fmt::Write as _;

    let mut out =
        String::from("[FixOnce] WARNING — proposed tool use matches known anti-pattern(s):\n");

    for hit in hits {
        let _ = writeln!(
            out,
            "- [{:.2}] {}: {}",
            hit.similarity, hit.memory.title, hit.memory.summary,
        );
        if let Some(ref am) = hit.memory.anti_memory {
            let _ = writeln!(out, "  Reason: {}", am.reason);
            if let Some(ref alt) = am.alternative {
                let _ = writeln!(out, "  Alternative: {alt}");
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
                    reason: "It causes issues".to_owned(),
                    alternative: Some("Use the safe alternative".to_owned()),
                    version_constraints: None,
                }),
            },
            similarity,
        }
    }

    // --- truncate ---

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hello world", 512), "hello world");
    }

    #[test]
    fn truncate_long_string_limited() {
        let s = "x".repeat(1000);
        assert_eq!(truncate(&s, 512).len(), 512);
    }

    #[test]
    fn truncate_trims_whitespace() {
        assert_eq!(truncate("  hi  ", 512), "hi");
    }

    // --- format_pre_tool_warning ---

    #[test]
    fn warning_includes_anti_pattern_details() {
        let hits = vec![make_anti_hit("foo", 0.85)];
        let msg = format_pre_tool_warning(&hits);
        assert!(msg.contains("WARNING"));
        assert!(msg.contains("AntiPattern foo"));
        assert!(msg.contains("It causes issues"));
        assert!(msg.contains("Use the safe alternative"));
    }

    #[test]
    fn warning_shows_similarity_score() {
        let hits = vec![make_anti_hit("bar", 0.92)];
        let msg = format_pre_tool_warning(&hits);
        assert!(msg.contains("0.92"));
    }

    // --- threshold filtering logic (unit) ---

    #[test]
    fn below_threshold_would_not_generate_warning() {
        // Simulate the filtering logic directly.
        let hits = vec![make_anti_hit("low", 0.65)];
        let warnings: Vec<_> = hits
            .into_iter()
            .filter(|h| {
                h.memory.memory_type == MemoryType::AntiPattern
                    && h.similarity >= PRE_TOOL_WARN_THRESHOLD
            })
            .collect();
        assert!(warnings.is_empty());
    }

    #[test]
    fn at_threshold_generates_warning() {
        let hits = vec![make_anti_hit("exact", PRE_TOOL_WARN_THRESHOLD)];
        let warnings: Vec<_> = hits
            .into_iter()
            .filter(|h| {
                h.memory.memory_type == MemoryType::AntiPattern
                    && h.similarity >= PRE_TOOL_WARN_THRESHOLD
            })
            .collect();
        assert_eq!(warnings.len(), 1);
    }
}
