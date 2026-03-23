//! Human-readable plain-text formatters for memory types.

use std::fmt::Write as _;

use crate::memory::types::{Memory, SearchMemoryResponse};

/// Format a single memory as human-readable text.
#[must_use]
pub fn format_memory_text(memory: &Memory) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "Memory: {}", memory.title);
    let _ = writeln!(out, "  ID       : {}", memory.id);
    let _ = writeln!(out, "  Type     : {}", memory.memory_type);
    let _ = writeln!(out, "  Source   : {}", memory.source_type);

    if let Some(lang) = &memory.language {
        let _ = writeln!(out, "  Language : {lang}");
    }

    let _ = writeln!(out, "  Decay    : {:.4}", memory.decay_score);
    let _ = writeln!(out, "  Reinforce: {:.4}", memory.reinforcement_score);
    let _ = writeln!(out, "  Embedding: {}", memory.embedding_status);
    let _ = writeln!(out, "  Pipeline : {}", memory.pipeline_status);
    let _ = writeln!(out, "  Created  : {}", memory.created_at);
    let _ = writeln!(out, "  Updated  : {}", memory.updated_at);

    if !memory.summary.is_empty() {
        let _ = writeln!(out, "\nSummary:\n  {}", memory.summary);
    }

    if !memory.content.is_empty() {
        let _ = writeln!(out, "\nContent:\n{}", memory.content);
    }

    out
}

/// Format a slice of memories as a numbered list.
#[must_use]
pub fn format_memory_list_text(memories: &[Memory]) -> String {
    if memories.is_empty() {
        return "No memories found.\n".to_owned();
    }

    memories
        .iter()
        .enumerate()
        .fold(String::new(), |mut acc, (i, m)| {
            let _ = writeln!(
                acc,
                "{}. [{}] {} (id={})\n   {}",
                i + 1,
                m.memory_type,
                m.title,
                m.id,
                m.summary,
            );
            acc
        })
}

/// Format a [`SearchMemoryResponse`] as human-readable text.
#[must_use]
pub fn format_search_results_text(results: &SearchMemoryResponse) -> String {
    if results.hits.is_empty() {
        return "No matching memories found.\n".to_owned();
    }

    let mut out = format!(
        "{} result{} found:\n\n",
        results.total,
        if results.total == 1 { "" } else { "s" }
    );

    for (i, hit) in results.hits.iter().enumerate() {
        let _ = writeln!(
            out,
            "{}. [{:.4}] {} (id={})\n   Type: {}\n   {}\n",
            i + 1,
            hit.similarity,
            hit.memory.title,
            hit.memory.id,
            hit.memory.memory_type,
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
    use super::*;
    use crate::memory::types::{
        EmbeddingStatus, MemoryType, PipelineStatus, SearchHit, SourceType,
    };

    fn sample_memory() -> Memory {
        Memory {
            id: "abc123".to_owned(),
            title: "Test memory".to_owned(),
            content: "Some content here.".to_owned(),
            summary: "Short summary.".to_owned(),
            memory_type: MemoryType::Gotcha,
            source_type: SourceType::Manual,
            language: Some("compact".to_owned()),
            compact_pragma: None,
            compact_compiler: None,
            midnight_js: None,
            indexer_version: None,
            node_version: None,
            source_url: None,
            repo_url: None,
            task_summary: None,
            session_id: None,
            decay_score: 0.85,
            reinforcement_score: 0.5,
            last_accessed_at: None,
            embedding_status: EmbeddingStatus::Complete,
            pipeline_status: PipelineStatus::Complete,
            deleted_at: None,
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            updated_at: "2026-01-02T00:00:00Z".to_owned(),
            created_by: "user-1".to_owned(),
            anti_memory: None,
        }
    }

    #[test]
    fn format_single_memory_contains_title() {
        let m = sample_memory();
        let text = format_memory_text(&m);
        assert!(text.contains("Test memory"));
        assert!(text.contains("abc123"));
        assert!(text.contains("gotcha"));
        assert!(text.contains("0.8500"));
    }

    #[test]
    fn format_empty_list_returns_placeholder() {
        let text = format_memory_list_text(&[]);
        assert_eq!(text, "No memories found.\n");
    }

    #[test]
    fn format_list_numbers_entries() {
        let memories = vec![sample_memory(), sample_memory()];
        let text = format_memory_list_text(&memories);
        assert!(text.contains("1."));
        assert!(text.contains("2."));
    }

    #[test]
    fn format_search_results_empty() {
        let results = SearchMemoryResponse {
            hits: vec![],
            total: 0,
        };
        let text = format_search_results_text(&results);
        assert!(text.contains("No matching memories"));
    }

    #[test]
    fn format_search_results_with_hits() {
        let hit = SearchHit {
            memory: sample_memory(),
            similarity: 0.92,
        };
        let results = SearchMemoryResponse {
            hits: vec![hit],
            total: 1,
        };
        let text = format_search_results_text(&results);
        assert!(text.contains("1 result"));
        assert!(text.contains("0.9200"));
        assert!(text.contains("Test memory"));
    }
}
