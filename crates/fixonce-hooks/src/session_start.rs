//! Session-start hook: surfaces critical memories when a Claude Code session begins.
//!
//! Runs environment detection, queries for critical memories matching the
//! detected Midnight ecosystem versions, and returns a brief message (top 3)
//! suitable for injecting into the agent's context.

use std::path::Path;

use fixonce_core::{
    api::{search::search_memories, ApiClient},
    auth::token::TokenManager,
    detect::midnight::detect_midnight_versions,
    memory::types::{MemoryType, SearchMemoryRequest},
};

use crate::HookError;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Called when a Claude Code session starts.
///
/// 1. Detects Midnight ecosystem versions in the current working directory.
/// 2. Searches for critical memories (gotchas, anti-patterns, corrections)
///    relevant to those versions.
/// 3. Formats the top 3 as a concise message for the agent.
///
/// Returns an empty string when no relevant memories are found.
///
/// # Errors
///
/// Returns [`HookError::Unauthenticated`] when no token is stored (EC-43).
/// Returns [`HookError::Api`] on network failure.
pub async fn on_session_start(api_url: &str) -> Result<String, HookError> {
    // EC-43: load token; skip silently when absent.
    let token = load_token()?;

    let client = ApiClient::new(api_url)
        .map_err(HookError::Api)?
        .with_token(&token);

    // 1. Detect Midnight ecosystem versions.
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let versions = detect_midnight_versions(Path::new(&cwd));

    // 2. Build a query that reflects the detected versions.
    let query = build_version_query(&versions);

    let req = SearchMemoryRequest {
        query,
        limit: Some(10),
        threshold: Some(0.6),
        language: None,
    };

    let resp = search_memories(&client, &req)
        .await
        .map_err(HookError::Api)?;

    // 3. Filter to high-priority types (gotcha, anti_pattern, correction) and
    //    take the top 3 by similarity score.
    let critical: Vec<_> = resp
        .hits
        .into_iter()
        .filter(|h| {
            matches!(
                h.memory.memory_type,
                MemoryType::Gotcha | MemoryType::AntiPattern | MemoryType::Correction
            )
        })
        .take(3)
        .collect();

    if critical.is_empty() {
        return Ok(String::new());
    }

    Ok(format_session_start_message(&critical))
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

/// Build a search query string from detected Midnight versions.
fn build_version_query(versions: &fixonce_core::detect::midnight::MidnightVersions) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(ref v) = versions.compact_compiler {
        parts.push(format!("compact compiler {v}"));
    }
    if let Some(ref v) = versions.midnight_js {
        parts.push(format!("midnight-js {v}"));
    }
    if let Some(ref v) = versions.compact_pragma {
        parts.push(format!("pragma {v}"));
    }
    if let Some(ref v) = versions.node_version {
        parts.push(format!("node {v}"));
    }

    if parts.is_empty() {
        "critical gotcha warning anti-pattern".to_owned()
    } else {
        parts.join(" ")
    }
}

/// Format a list of critical hits as a short session-start advisory.
fn format_session_start_message(hits: &[fixonce_core::memory::types::SearchHit]) -> String {
    use std::fmt::Write as _;

    let mut out = String::from("[FixOnce] Critical memories for this session:\n");

    for (i, hit) in hits.iter().enumerate() {
        let _ = writeln!(
            out,
            "{}. [{}] {} — {}",
            i + 1,
            hit.memory.memory_type,
            hit.memory.title,
            hit.memory.summary,
        );

        // Surface the anti-memory alternative when available.
        if let Some(ref am) = hit.memory.anti_memory {
            if let Some(ref alt) = am.alternative {
                let _ = writeln!(out, "   -> Instead: {alt}");
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
        EmbeddingStatus, Memory, MemoryType, PipelineStatus, SearchHit, SourceType,
    };

    use super::*;

    fn make_hit(id: &str, mt: MemoryType, similarity: f64) -> SearchHit {
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
            similarity,
        }
    }

    #[test]
    fn format_message_includes_all_hits() {
        let hits = vec![
            make_hit("a", MemoryType::Gotcha, 0.9),
            make_hit("b", MemoryType::AntiPattern, 0.8),
            make_hit("c", MemoryType::Correction, 0.7),
        ];
        let msg = format_session_start_message(&hits);
        assert!(msg.contains("[FixOnce]"), "should have header");
        assert!(msg.contains("Title a"), "should list first hit");
        assert!(msg.contains("Title b"), "should list second hit");
        assert!(msg.contains("Title c"), "should list third hit");
    }

    #[test]
    fn format_message_single_hit() {
        let hits = vec![make_hit("x", MemoryType::Gotcha, 0.95)];
        let msg = format_session_start_message(&hits);
        assert!(msg.contains("1."));
        assert!(!msg.contains("2."));
    }

    #[test]
    fn build_version_query_all_none_uses_fallback() {
        let versions = fixonce_core::detect::midnight::MidnightVersions::default();
        let q = build_version_query(&versions);
        assert!(q.contains("critical") || q.contains("gotcha"));
    }

    #[test]
    fn build_version_query_with_versions() {
        let versions = fixonce_core::detect::midnight::MidnightVersions {
            compact_compiler: Some("0.15.0".to_owned()),
            midnight_js: Some("1.2.3".to_owned()),
            compact_pragma: None,
            indexer_version: None,
            node_version: None,
        };
        let q = build_version_query(&versions);
        assert!(q.contains("0.15.0"));
        assert!(q.contains("1.2.3"));
    }

    #[test]
    #[serial_test::serial(keyring)]
    fn load_token_returns_unauthenticated_when_no_token() {
        // RAII guard ensures cleanup even on panic.
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                std::env::remove_var("FIXONCE_KEYRING_SERVICE");
            }
        }

        // Use an isolated keyring service so this test does not depend on
        // whether the developer has stored real credentials.
        let service = format!("fixonce-test-{}", std::process::id());
        std::env::set_var("FIXONCE_KEYRING_SERVICE", &service);
        let _guard = Guard;

        let result = load_token();
        assert!(
            result.is_err(),
            "should error when no token is stored in isolated keyring"
        );
    }
}
