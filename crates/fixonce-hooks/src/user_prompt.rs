//! User-prompt hook: surfaces relevant memories when the user submits a prompt.
//!
//! Performs a lightweight hybrid search on the prompt text and returns the
//! top 3–5 matching memories as context for the agent.

use fixonce_core::{
    api::{search::search_memories, ApiClient},
    auth::token::TokenManager,
    memory::types::SearchMemoryRequest,
};

use crate::HookError;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Called when the user submits a prompt.
///
/// 1. Performs a basic query rewrite (trims, normalises whitespace).
/// 2. Runs a lightweight hybrid search (no deep pipeline).
/// 3. Returns the top 3–5 relevant memories as a formatted context block.
///
/// Returns an empty string when no relevant memories are found.
///
/// # Errors
///
/// Returns [`HookError::Unauthenticated`] when no token is stored (EC-43).
/// Returns [`HookError::Api`] on network failure.
pub async fn on_user_prompt(api_url: &str, prompt_text: &str) -> Result<String, HookError> {
    // EC-43: load token; skip silently when absent.
    let token = load_token()?;

    let client = ApiClient::new(api_url)
        .map_err(HookError::Api)?
        .with_token(&token);

    // 1. Basic query rewrite: trim and collapse whitespace.
    let query = rewrite_query(prompt_text);

    if query.is_empty() {
        return Ok(String::new());
    }

    // 2. Lightweight hybrid search — top 5 results, moderate threshold.
    let req = SearchMemoryRequest {
        query,
        limit: Some(5),
        threshold: Some(0.65),
        language: None,
    };

    let resp = search_memories(&client, &req)
        .await
        .map_err(HookError::Api)?;

    if resp.hits.is_empty() {
        return Ok(String::new());
    }

    // 3. Return formatted context block.
    Ok(format_prompt_context(&resp.hits))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Load the JWT from the OS keyring, mapping missing token to [`HookError::Unauthenticated`].
fn load_token() -> Result<String, HookError> {
    let mgr = TokenManager::new();
    match mgr.load_token().map_err(HookError::Auth)? {
        Some(t) => Ok(t),
        None => Err(HookError::Unauthenticated),
    }
}

/// Rewrite a prompt for search: trim, collapse whitespace, truncate to 512 chars.
///
/// Returns an empty string when the input is blank.
pub(crate) fn rewrite_query(prompt: &str) -> String {
    let collapsed: String = prompt.split_whitespace().collect::<Vec<_>>().join(" ");

    // Truncate long prompts to keep the search request lightweight.
    if collapsed.len() > 512 {
        collapsed[..512].to_owned()
    } else {
        collapsed
    }
}

/// Format search hits as a compact context block for the agent.
fn format_prompt_context(hits: &[fixonce_core::memory::types::SearchHit]) -> String {
    use std::fmt::Write as _;

    let mut out = String::from("[FixOnce] Relevant memories:\n");

    for hit in hits {
        let _ = writeln!(
            out,
            "- [{}] {}: {}",
            hit.memory.memory_type, hit.memory.title, hit.memory.summary,
        );

        if let Some(ref am) = hit.memory.anti_memory {
            let _ = writeln!(out, "  AVOID: {}", am.description);
            if let Some(ref alt) = am.alternative {
                let _ = writeln!(out, "  USE:   {alt}");
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

    fn make_hit(id: &str, mt: MemoryType) -> SearchHit {
        SearchHit {
            memory: Memory {
                id: id.to_owned(),
                title: format!("Title {id}"),
                content: "content".to_owned(),
                summary: format!("Summary {id}"),
                memory_type: mt,
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
                decay_score: 0.8,
                reinforcement_score: 0.8,
                last_accessed_at: None,
                embedding_status: EmbeddingStatus::Complete,
                pipeline_status: PipelineStatus::Complete,
                deleted_at: None,
                created_at: "2026-01-01T00:00:00Z".to_owned(),
                updated_at: "2026-01-01T00:00:00Z".to_owned(),
                created_by: "user-1".to_owned(),
                anti_memory: None,
            },
            similarity: 0.8,
        }
    }

    // --- rewrite_query ---

    #[test]
    fn rewrite_trims_whitespace() {
        assert_eq!(rewrite_query("  hello   world  "), "hello world");
    }

    #[test]
    fn rewrite_empty_returns_empty() {
        assert_eq!(rewrite_query("   "), "");
        assert_eq!(rewrite_query(""), "");
    }

    #[test]
    fn rewrite_truncates_long_prompt() {
        let long = "a ".repeat(300); // 600 chars
        let result = rewrite_query(&long);
        assert!(result.len() <= 512);
    }

    #[test]
    fn rewrite_short_prompt_unchanged_length() {
        let short = "how do I fix this bug?";
        let result = rewrite_query(short);
        assert_eq!(result, short);
    }

    // --- format_prompt_context ---

    #[test]
    fn format_context_has_header() {
        let hits = vec![make_hit("x", MemoryType::Gotcha)];
        let msg = format_prompt_context(&hits);
        assert!(msg.starts_with("[FixOnce]"));
    }

    #[test]
    fn format_context_includes_title_and_summary() {
        let hits = vec![make_hit("y", MemoryType::BestPractice)];
        let msg = format_prompt_context(&hits);
        assert!(msg.contains("Title y"));
        assert!(msg.contains("Summary y"));
    }

    #[test]
    fn format_context_shows_anti_memory_alternative() {
        let mut hit = make_hit("z", MemoryType::AntiPattern);
        hit.memory.anti_memory = Some(AntiMemory {
            description: "Do not use foo".to_owned(),
            reason: "It is broken".to_owned(),
            alternative: Some("Use bar instead".to_owned()),
            version_constraints: None,
        });
        let msg = format_prompt_context(&[hit]);
        assert!(msg.contains("AVOID: Do not use foo"));
        assert!(msg.contains("USE:   Use bar instead"));
    }
}
