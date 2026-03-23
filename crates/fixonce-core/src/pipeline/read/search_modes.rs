//! Search mode stages for the read pipeline.
//!
//! Each mode is a zero-sized struct that implements
//! [`PipelineStage`][super::pipeline_runner::PipelineStage].
//!
//! | Mode | Mechanism |
//! |------|-----------|
//! | [`HybridSearch`] | `hybrid_search` Supabase edge-function RPC |
//! | [`FtsSearch`] | Full-text search only |
//! | [`VectorSearch`] | Vector similarity only |
//! | [`MetadataFilter`] | Filter by version metadata fields |
//! | [`GraphAssisted`] | Follow lineage / contradiction links |
//! | [`PassageCompression`] | Claude compresses long result passages |

use crate::{
    api::{memories::search_memories, ApiClient},
    memory::types::{SearchHit, SearchMemoryRequest},
    pipeline::{claude::ClaudeClient, PipelineError},
};

use super::pipeline_runner::{PipelineContext, PipelineStage};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Return the effective search query.
///
/// Uses the last rewritten query when available, otherwise falls back to
/// the original query.
fn effective_query(ctx: &PipelineContext) -> &str {
    ctx.rewritten_queries
        .last()
        .map_or(&ctx.original_query, String::as_str)
}

// ---------------------------------------------------------------------------
// Version filter parsed from `--version key=value` args
// ---------------------------------------------------------------------------

/// A single version metadata filter (e.g. `compact_compiler=0.15`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionFilter {
    pub key: String,
    pub value: String,
}

impl VersionFilter {
    /// Parse `"key=value"`.
    ///
    /// Returns `None` if the string does not contain `=`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let (key, value) = s.split_once('=')?;
        Some(Self {
            key: key.trim().to_owned(),
            value: value.trim().to_owned(),
        })
    }
}

// ---------------------------------------------------------------------------
// 1. HybridSearch
// ---------------------------------------------------------------------------

/// Calls the `hybrid_search` Supabase edge-function RPC, which combines FTS
/// and vector similarity.
///
/// When no `ApiClient` is present in the pipeline context this stage is a
/// no-op; callers that need search must inject the client into a wrapper or
/// pass it via [`PipelineContext::metadata`].
pub struct HybridSearch;

/// Build the hybrid search request.
#[must_use]
pub fn build_hybrid_search_request(ctx: &PipelineContext, limit: u32) -> SearchMemoryRequest {
    SearchMemoryRequest {
        query: effective_query(ctx).to_owned(),
        limit: Some(limit),
        threshold: None,
        language: None,
    }
}

impl PipelineStage for HybridSearch {
    fn name(&self) -> &'static str {
        "hybrid_search"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        // The API client is not stored in PipelineContext by default.
        // When running in a real CLI command, callers inject it via metadata.
        // In the pipeline runner this stage is a structural no-op when no
        // client is available — it signals that the runner needs wiring.
        //
        // In production the CLI command calls this after constructing a client:
        //   ctx.metadata["_api_client_serialised_url"] = ...
        // For now the stage records its intent and returns successfully so
        // the pipeline composition still works end-to-end.
        ctx.metadata["last_search_mode"] = serde_json::Value::String("hybrid".to_owned());
        ctx.metadata["search_query"] = serde_json::Value::String(effective_query(ctx).to_owned());
        Ok(())
    }
}

/// Execute a real hybrid search using the provided API client.
///
/// This function is called by the CLI command; it is not a stage itself
/// because `ApiClient` cannot be stored in the context without ownership
/// complications.
///
/// # Errors
///
/// Returns [`PipelineError::Api`] if the search request fails.
pub async fn execute_hybrid_search(
    client: &ApiClient,
    ctx: &mut PipelineContext,
    limit: u32,
) -> Result<Vec<SearchHit>, PipelineError> {
    let req = build_hybrid_search_request(ctx, limit);
    let response = search_memories(client, &req)
        .await
        .map_err(|e| PipelineError::Api(e.to_string()))?;
    Ok(response.hits)
}

// ---------------------------------------------------------------------------
// 2. FtsSearch
// ---------------------------------------------------------------------------

/// Full-text search only (no vector component).
pub struct FtsSearch;

impl PipelineStage for FtsSearch {
    fn name(&self) -> &'static str {
        "fts_search"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        ctx.metadata["last_search_mode"] = serde_json::Value::String("fts".to_owned());
        ctx.metadata["search_query"] = serde_json::Value::String(effective_query(ctx).to_owned());
        Ok(())
    }
}

/// Execute an FTS search using the provided API client.
///
/// # Errors
///
/// Returns [`PipelineError::Api`] if the search request fails.
pub async fn execute_fts_search(
    client: &ApiClient,
    ctx: &mut PipelineContext,
    limit: u32,
) -> Result<Vec<SearchHit>, PipelineError> {
    let req = SearchMemoryRequest {
        query: effective_query(ctx).to_owned(),
        limit: Some(limit),
        threshold: Some(0.0), // FTS: accept any score
        language: None,
    };
    let response = search_memories(client, &req)
        .await
        .map_err(|e| PipelineError::Api(e.to_string()))?;
    Ok(response.hits)
}

// ---------------------------------------------------------------------------
// 3. VectorSearch
// ---------------------------------------------------------------------------

/// Vector similarity only (no FTS component).
pub struct VectorSearch;

impl PipelineStage for VectorSearch {
    fn name(&self) -> &'static str {
        "vector_search"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        ctx.metadata["last_search_mode"] = serde_json::Value::String("vector".to_owned());
        ctx.metadata["search_query"] = serde_json::Value::String(effective_query(ctx).to_owned());
        Ok(())
    }
}

/// Execute a vector similarity search.
///
/// # Errors
///
/// Returns [`PipelineError::Api`] if the search request fails.
pub async fn execute_vector_search(
    client: &ApiClient,
    ctx: &mut PipelineContext,
    limit: u32,
    threshold: f64,
) -> Result<Vec<SearchHit>, PipelineError> {
    let req = SearchMemoryRequest {
        query: effective_query(ctx).to_owned(),
        limit: Some(limit),
        threshold: Some(threshold),
        language: None,
    };
    let response = search_memories(client, &req)
        .await
        .map_err(|e| PipelineError::Api(e.to_string()))?;
    Ok(response.hits)
}

// ---------------------------------------------------------------------------
// 4. MetadataFilter
// ---------------------------------------------------------------------------

/// Filters current results by version metadata (e.g. `compact_compiler=0.15`).
///
/// This is a client-side filter applied to `ctx.results` after an initial
/// search has populated it.
pub struct MetadataFilter {
    pub filters: Vec<VersionFilter>,
}

impl MetadataFilter {
    #[must_use]
    pub fn new(filters: Vec<VersionFilter>) -> Self {
        Self { filters }
    }

    /// Test whether a memory passes all filters.
    #[must_use]
    pub fn matches(&self, hit: &SearchHit) -> bool {
        self.filters.iter().all(|f| {
            let mem = &hit.memory;
            let field_value: Option<&str> = match f.key.as_str() {
                "compact_pragma" => mem.compact_pragma.as_deref(),
                "compact_compiler" => mem.compact_compiler.as_deref(),
                "midnight_js" => mem.midnight_js.as_deref(),
                "indexer_version" => mem.indexer_version.as_deref(),
                "node_version" => mem.node_version.as_deref(),
                "language" => mem.language.as_deref(),
                _ => None,
            };
            field_value.is_some_and(|v| v == f.value)
        })
    }
}

impl PipelineStage for MetadataFilter {
    fn name(&self) -> &'static str {
        "metadata_filter"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        if self.filters.is_empty() {
            return Ok(());
        }

        ctx.results.retain(|hit| self.matches(hit));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 5. GraphAssisted
// ---------------------------------------------------------------------------

/// Augments the result set by following lineage / contradiction links encoded
/// in memory metadata.  For now this is a structural placeholder — actual
/// graph traversal depends on the backend exposing relationship endpoints.
pub struct GraphAssisted;

impl PipelineStage for GraphAssisted {
    fn name(&self) -> &'static str {
        "graph_assisted"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        // Mark that graph-assisted search was requested.
        ctx.metadata["last_search_mode"] = serde_json::Value::String("graph".to_owned());
        // In a full implementation this would:
        //   1. For each result in ctx.results, fetch linked memories.
        //   2. Append unique linked memories to ctx.results.
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 6. PassageCompression
// ---------------------------------------------------------------------------

/// Uses Claude to compress long memory content into concise passages that
/// fit within LLM context windows.
pub struct PassageCompression;

/// Build the passage compression prompt (exposed for testing).
#[must_use]
pub fn build_compression_prompt(query: &str, content: &str) -> String {
    format!(
        r"You are a passage extractor for a developer knowledge base called FixOnce.

Given the user's query and the memory content below, extract only the sentences
and paragraphs that are directly relevant to the query.  Preserve all technical
details (version numbers, error messages, commands, code).  Return a concise
passage of at most 5 sentences.

Query: {query}

Memory content:
{content}

Reply with **only** the extracted passage — no preamble, no JSON."
    )
}

impl PipelineStage for PassageCompression {
    fn name(&self) -> &'static str {
        "passage_compression"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        if ctx.results.is_empty() {
            return Ok(());
        }

        let client = ClaudeClient::new();
        let query = ctx.original_query.clone();

        for hit in &mut ctx.results {
            // Only compress long content (> 500 chars).
            if hit.memory.content.len() > 500 {
                let prompt = build_compression_prompt(&query, &hit.memory.content);
                match client.prompt(&prompt).await {
                    Ok(compressed) => {
                        let compressed = compressed.trim().to_owned();
                        if !compressed.is_empty() {
                            hit.memory.content = compressed;
                        }
                    }
                    Err(PipelineError::ClaudeNotFound | PipelineError::ClaudeTimeout { .. }) => {
                        // Degrade: leave content as-is.
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::{
        EmbeddingStatus, Memory, MemoryType, PipelineStatus, SearchHit, SourceType,
    };

    fn sample_hit(id: &str) -> SearchHit {
        SearchHit {
            memory: Memory {
                id: id.to_owned(),
                title: "Test Memory".to_owned(),
                content: "content".to_owned(),
                summary: "summary".to_owned(),
                memory_type: MemoryType::Gotcha,
                source_type: SourceType::Manual,
                language: None,
                compact_pragma: None,
                compact_compiler: Some("0.15".to_owned()),
                midnight_js: None,
                indexer_version: None,
                node_version: None,
                source_url: None,
                repo_url: None,
                task_summary: None,
                session_id: None,
                decay_score: 0.9,
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
            similarity: 0.85,
        }
    }

    // --- VersionFilter ---

    #[test]
    fn version_filter_parses_key_value() {
        let f = VersionFilter::parse("compact_compiler=0.15").expect("must parse");
        assert_eq!(f.key, "compact_compiler");
        assert_eq!(f.value, "0.15");
    }

    #[test]
    fn version_filter_parse_returns_none_without_equals() {
        assert!(VersionFilter::parse("no-equals-sign").is_none());
    }

    #[test]
    fn version_filter_parse_trims_whitespace() {
        let f = VersionFilter::parse("  key  =  val  ").expect("must parse");
        assert_eq!(f.key, "key");
        assert_eq!(f.value, "val");
    }

    // --- MetadataFilter ---

    #[test]
    fn metadata_filter_matches_known_field() {
        let filter = MetadataFilter::new(vec![VersionFilter {
            key: "compact_compiler".to_owned(),
            value: "0.15".to_owned(),
        }]);
        let hit = sample_hit("id-1");
        assert!(filter.matches(&hit));
    }

    #[test]
    fn metadata_filter_rejects_wrong_value() {
        let filter = MetadataFilter::new(vec![VersionFilter {
            key: "compact_compiler".to_owned(),
            value: "0.99".to_owned(),
        }]);
        let hit = sample_hit("id-1");
        assert!(!filter.matches(&hit));
    }

    #[test]
    fn metadata_filter_rejects_unknown_key() {
        let filter = MetadataFilter::new(vec![VersionFilter {
            key: "unknown_field".to_owned(),
            value: "any".to_owned(),
        }]);
        let hit = sample_hit("id-1");
        assert!(!filter.matches(&hit));
    }

    #[tokio::test]
    async fn metadata_filter_removes_non_matching_results() {
        let mut ctx = PipelineContext::default();
        ctx.results.push(sample_hit("id-match")); // compact_compiler=0.15
        let mut other = sample_hit("id-no-match");
        other.memory.compact_compiler = Some("0.99".to_owned());
        ctx.results.push(other);

        let filter = MetadataFilter::new(vec![VersionFilter {
            key: "compact_compiler".to_owned(),
            value: "0.15".to_owned(),
        }]);
        filter.execute(&mut ctx).await.expect("must succeed");

        assert_eq!(ctx.results.len(), 1);
        assert_eq!(ctx.results[0].memory.id, "id-match");
    }

    // --- HybridSearch stage metadata ---

    #[tokio::test]
    async fn hybrid_search_records_mode_in_metadata() {
        let mut ctx = PipelineContext::new("test query");
        HybridSearch.execute(&mut ctx).await.expect("must succeed");
        assert_eq!(ctx.metadata["last_search_mode"], "hybrid");
    }

    #[tokio::test]
    async fn fts_search_records_mode_in_metadata() {
        let mut ctx = PipelineContext::new("test query");
        FtsSearch.execute(&mut ctx).await.expect("must succeed");
        assert_eq!(ctx.metadata["last_search_mode"], "fts");
    }

    #[tokio::test]
    async fn vector_search_records_mode_in_metadata() {
        let mut ctx = PipelineContext::new("test query");
        VectorSearch.execute(&mut ctx).await.expect("must succeed");
        assert_eq!(ctx.metadata["last_search_mode"], "vector");
    }

    // --- HybridSearch request building ---

    #[test]
    fn hybrid_search_request_uses_last_rewritten_query() {
        let mut ctx = PipelineContext::new("original");
        ctx.rewritten_queries.push("rewritten".to_owned());
        let req = build_hybrid_search_request(&ctx, 10);
        assert_eq!(req.query, "rewritten");
        assert_eq!(req.limit, Some(10));
    }

    #[test]
    fn hybrid_search_request_falls_back_to_original_query() {
        let ctx = PipelineContext::new("original");
        let req = build_hybrid_search_request(&ctx, 5);
        assert_eq!(req.query, "original");
    }

    // --- PassageCompression prompt ---

    #[test]
    fn compression_prompt_contains_query_and_content() {
        let p = build_compression_prompt("my query", "long content here");
        assert!(p.contains("my query"));
        assert!(p.contains("long content here"));
    }

    // --- Stage names are unique ---

    #[test]
    fn all_search_mode_names_unique() {
        let mf = MetadataFilter::new(vec![]);
        let names = [
            PipelineStage::name(&HybridSearch),
            PipelineStage::name(&FtsSearch),
            PipelineStage::name(&VectorSearch),
            PipelineStage::name(&mf),
            PipelineStage::name(&GraphAssisted),
            PipelineStage::name(&PassageCompression),
        ];
        let mut seen = std::collections::HashSet::new();
        for name in &names {
            assert!(seen.insert(name), "duplicate stage name: {name}");
        }
    }
}
