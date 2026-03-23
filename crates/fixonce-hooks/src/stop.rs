//! Stop hook: surfaces critical reminders when a Claude Code session ends.
//!
//! Queries for critical memories related to the current project and formats
//! them as brief session-end reminders for the agent.

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

/// Called when a Claude Code session ends.
///
/// 1. Queries for critical memories related to the current project.
/// 2. Formats them as brief session-end reminders for the agent.
///
/// Returns an empty string when no relevant memories are found.
///
/// # Errors
///
/// Returns [`HookError::Unauthenticated`] when no token is stored (EC-43).
/// Returns [`HookError::Api`] on network failure.
pub async fn on_stop(api_url: &str) -> Result<String, HookError> {
    // EC-43: load token; skip silently when absent.
    let token = load_token()?;

    let client = ApiClient::new(api_url)
        .map_err(HookError::Api)?
        .with_token(&token);

    // Detect project context for a focused query.
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let versions = detect_midnight_versions(Path::new(&cwd));
    let query = build_stop_query(&versions);

    let req = SearchMemoryRequest {
        query,
        limit: Some(5),
        threshold: Some(0.55),
        language: None,
    };

    let resp = search_memories(&client, &req)
        .await
        .map_err(HookError::Api)?;

    // Keep only high-priority memory types for session-end reminders.
    let reminders: Vec<_> = resp
        .hits
        .into_iter()
        .filter(|h| {
            matches!(
                h.memory.memory_type,
                MemoryType::Gotcha | MemoryType::AntiPattern | MemoryType::Correction
            )
        })
        .take(5)
        .collect();

    if reminders.is_empty() {
        return Ok(String::new());
    }

    Ok(format_stop_reminders(&reminders))
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

/// Build a session-end query from detected Midnight versions.
fn build_stop_query(versions: &fixonce_core::detect::midnight::MidnightVersions) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(ref v) = versions.compact_compiler {
        parts.push(format!("compact compiler {v}"));
    }
    if let Some(ref v) = versions.midnight_js {
        parts.push(format!("midnight-js {v}"));
    }

    if parts.is_empty() {
        "important reminder session end critical".to_owned()
    } else {
        // Append session-end context to the version query.
        parts.push("important reminder".to_owned());
        parts.join(" ")
    }
}

/// Format reminder hits as a concise session-end advisory.
fn format_stop_reminders(hits: &[fixonce_core::memory::types::SearchHit]) -> String {
    use std::fmt::Write as _;

    let mut out = String::from("[FixOnce] Session-end reminders:\n");

    for (i, hit) in hits.iter().enumerate() {
        let _ = writeln!(
            out,
            "{}. [{}] {}: {}",
            i + 1,
            hit.memory.memory_type,
            hit.memory.title,
            hit.memory.summary,
        );
    }

    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use fixonce_core::{
        detect::midnight::MidnightVersions,
        memory::types::{
            EmbeddingStatus, Memory, MemoryType, PipelineStatus, SearchHit, SourceType,
        },
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
            similarity: 0.75,
        }
    }

    // --- build_stop_query ---

    #[test]
    fn stop_query_fallback_when_no_versions() {
        let v = MidnightVersions::default();
        let q = build_stop_query(&v);
        assert!(q.contains("reminder") || q.contains("critical"));
    }

    #[test]
    fn stop_query_includes_detected_versions() {
        let v = MidnightVersions {
            compact_compiler: Some("0.15.0".to_owned()),
            midnight_js: Some("1.0.0".to_owned()),
            compact_pragma: None,
            indexer_version: None,
            node_version: None,
        };
        let q = build_stop_query(&v);
        assert!(q.contains("0.15.0"));
        assert!(q.contains("1.0.0"));
        assert!(q.contains("reminder"));
    }

    // --- format_stop_reminders ---

    #[test]
    fn reminders_has_header() {
        let hits = vec![make_hit("a", MemoryType::Gotcha)];
        let msg = format_stop_reminders(&hits);
        assert!(msg.starts_with("[FixOnce] Session-end reminders:"));
    }

    #[test]
    fn reminders_numbered() {
        let hits = vec![
            make_hit("a", MemoryType::Gotcha),
            make_hit("b", MemoryType::Correction),
        ];
        let msg = format_stop_reminders(&hits);
        assert!(msg.contains("1."));
        assert!(msg.contains("2."));
    }

    #[test]
    fn reminders_includes_type_and_summary() {
        let hits = vec![make_hit("z", MemoryType::AntiPattern)];
        let msg = format_stop_reminders(&hits);
        assert!(msg.contains("anti_pattern"));
        assert!(msg.contains("Summary z"));
    }
}
