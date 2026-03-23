//! TOON (Token-Optimised Output Notation) formatters for memory types.
//!
//! TOON is a compact key-value notation designed to minimise token usage when
//! memories are injected into LLM context windows.
//!
//! Example output:
//! ```text
//! [Memory id=abc123]
//! title: How to handle map iteration in Compact
//! type: gotcha
//! language: compact
//! decay: 0.85
//! summary: Map iteration requires...
//! content: When iterating...
//! [/Memory]
//! ```

use std::fmt::Write as _;

use crate::memory::types::{Memory, SearchMemoryResponse};

/// Format a single [`Memory`] in TOON notation.
#[must_use]
pub fn format_memory_toon(memory: &Memory) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "[Memory id={}]", memory.id);
    let _ = writeln!(out, "title: {}", memory.title);
    let _ = writeln!(out, "type: {}", memory.memory_type);
    let _ = writeln!(out, "source: {}", memory.source_type);

    if let Some(lang) = &memory.language {
        let _ = writeln!(out, "language: {lang}");
    }

    if let Some(pragma) = &memory.compact_pragma {
        let _ = writeln!(out, "compact_pragma: {pragma}");
    }

    if let Some(compiler) = &memory.compact_compiler {
        let _ = writeln!(out, "compact_compiler: {compiler}");
    }

    if let Some(mjs) = &memory.midnight_js {
        let _ = writeln!(out, "midnight_js: {mjs}");
    }

    if let Some(indexer) = &memory.indexer_version {
        let _ = writeln!(out, "indexer_version: {indexer}");
    }

    if let Some(node) = &memory.node_version {
        let _ = writeln!(out, "node_version: {node}");
    }

    let _ = writeln!(out, "decay: {:.4}", memory.decay_score);
    let _ = writeln!(out, "reinforce: {:.4}", memory.reinforcement_score);

    if !memory.summary.is_empty() {
        let _ = writeln!(out, "summary: {}", memory.summary);
    }

    if !memory.content.is_empty() {
        let _ = writeln!(out, "content: {}", memory.content);
    }

    out.push_str("[/Memory]\n");

    out
}

/// Format a [`SearchMemoryResponse`] in TOON notation.
///
/// Each hit is preceded by a similarity score comment.
#[must_use]
pub fn format_search_results_toon(results: &SearchMemoryResponse) -> String {
    if results.hits.is_empty() {
        return "[SearchResults total=0]\n[/SearchResults]\n".to_owned();
    }

    let mut out = format!("[SearchResults total={}]\n", results.total);

    for hit in &results.hits {
        let _ = writeln!(out, "# similarity={:.4}", hit.similarity);
        out.push_str(&format_memory_toon(&hit.memory));
    }

    out.push_str("[/SearchResults]\n");

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
            id: "toon001".to_owned(),
            title: "How to handle map iteration in Compact".to_owned(),
            content: "When iterating over maps, use the iter() method.".to_owned(),
            summary: "Map iteration requires iter().".to_owned(),
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
    fn toon_memory_has_correct_delimiters() {
        let m = sample_memory();
        let toon = format_memory_toon(&m);
        assert!(toon.starts_with("[Memory id=toon001]"));
        assert!(toon.ends_with("[/Memory]\n"));
    }

    #[test]
    fn toon_memory_contains_all_fields() {
        let m = sample_memory();
        let toon = format_memory_toon(&m);
        assert!(toon.contains("title: How to handle map iteration in Compact"));
        assert!(toon.contains("type: gotcha"));
        assert!(toon.contains("language: compact"));
        assert!(toon.contains("decay: 0.8500"));
        assert!(toon.contains("summary: Map iteration requires iter()."));
        assert!(toon.contains("content: When iterating over maps"));
    }

    #[test]
    fn toon_search_results_empty() {
        let results = SearchMemoryResponse {
            hits: vec![],
            total: 0,
        };
        let toon = format_search_results_toon(&results);
        assert!(toon.contains("[SearchResults total=0]"));
        assert!(toon.contains("[/SearchResults]"));
        assert!(!toon.contains("[Memory"));
    }

    #[test]
    fn toon_search_results_with_hits() {
        let hit = SearchHit {
            memory: sample_memory(),
            similarity: 0.92,
        };
        let results = SearchMemoryResponse {
            hits: vec![hit],
            total: 1,
        };
        let toon = format_search_results_toon(&results);
        assert!(toon.contains("[SearchResults total=1]"));
        assert!(toon.contains("# similarity=0.9200"));
        assert!(toon.contains("[Memory id=toon001]"));
        assert!(toon.contains("[/SearchResults]"));
    }

    #[test]
    fn toon_optional_fields_omitted_when_none() {
        let m = sample_memory(); // language is Some but no pragma/compiler/etc.
        let toon = format_memory_toon(&m);
        assert!(!toon.contains("compact_pragma:"));
        assert!(!toon.contains("compact_compiler:"));
        assert!(!toon.contains("midnight_js:"));
    }

    #[test]
    fn toon_optional_fields_present_when_some() {
        let mut m = sample_memory();
        m.compact_pragma = Some(">=0.4.0".to_owned());
        m.midnight_js = Some("3.0.0".to_owned());
        let toon = format_memory_toon(&m);
        assert!(toon.contains("compact_pragma: >=0.4.0"));
        assert!(toon.contains("midnight_js: 3.0.0"));
    }
}
