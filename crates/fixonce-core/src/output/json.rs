//! JSON output formatters for memory types.
//!
//! These wrappers exist so callers can choose pretty-printing or compact output
//! without touching `serde_json` directly.

use crate::memory::types::{Memory, SearchMemoryResponse};

/// Serialize a single [`Memory`] to a pretty-printed JSON string.
///
/// Returns a compact `{}` object on serialisation failure (which should never
/// happen for well-formed types).
#[must_use]
pub fn format_memory_json(memory: &Memory) -> String {
    serde_json::to_string_pretty(memory).unwrap_or_else(|_| "{}".to_owned())
}

/// Serialize a [`SearchMemoryResponse`] to a pretty-printed JSON string.
#[must_use]
pub fn format_search_results_json(results: &SearchMemoryResponse) -> String {
    serde_json::to_string_pretty(results).unwrap_or_else(|_| "{}".to_owned())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::{
        EmbeddingStatus, MemoryType, PipelineStatus, SearchHit, SourceType,
    };

    fn sample_memory() -> Memory {
        Memory {
            id: "def456".to_owned(),
            title: "JSON test memory".to_owned(),
            content: "Content.".to_owned(),
            summary: "Summary.".to_owned(),
            memory_type: MemoryType::BestPractice,
            source_type: SourceType::Observation,
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
            decay_score: 1.0,
            reinforcement_score: 0.0,
            last_accessed_at: None,
            embedding_status: EmbeddingStatus::Pending,
            pipeline_status: PipelineStatus::Incomplete,
            deleted_at: None,
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            updated_at: "2026-01-01T00:00:00Z".to_owned(),
            created_by: "user-2".to_owned(),
            anti_memory: None,
        }
    }

    #[test]
    fn memory_json_is_valid_json() {
        let m = sample_memory();
        let json_str = format_memory_json(&m);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("must be valid JSON");
        assert_eq!(parsed["id"], "def456");
        assert_eq!(parsed["memory_type"], "best_practice");
        assert_eq!(parsed["embedding_status"], "pending");
    }

    #[test]
    fn search_results_json_is_valid_json() {
        let results = SearchMemoryResponse {
            hits: vec![SearchHit {
                memory: sample_memory(),
                similarity: 0.75,
            }],
            total: 1,
        };
        let json_str = format_search_results_json(&results);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("must be valid JSON");
        assert_eq!(parsed["total"], 1);
        assert!(parsed["hits"].as_array().is_some());
    }

    #[test]
    fn memory_round_trips_through_json() {
        let original = sample_memory();
        let json_str = format_memory_json(&original);
        let round_tripped: Memory =
            serde_json::from_str(&json_str).expect("must deserialise cleanly");
        assert_eq!(round_tripped.id, original.id);
        assert_eq!(round_tripped.memory_type, original.memory_type);
    }
}
