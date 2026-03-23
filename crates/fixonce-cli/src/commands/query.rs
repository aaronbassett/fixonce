//! `fixonce query` — search memories with the read pipeline.
//!
//! The command:
//!
//! 1. Loads the authentication token.
//! 2. Retrieves the `VoyageAI` API key from secrets.
//! 3. Generates an embedding for the query text.
//! 4. Builds and runs the read pipeline (default or deep).
//! 5. When Claude is unavailable, returns raw results sorted by `decay_score`
//!    (degraded mode, EC-29).
//! 6. Formats and prints the results.

use anyhow::{Context, Result};
use clap::Args;
use fixonce_core::{
    api::{search::search_memories, secrets::get_secret, ApiClient},
    auth::token::TokenManager,
    detect::midnight::detect_midnight_versions,
    embeddings::VoyageClient,
    memory::types::{SearchMemoryRequest, SearchMemoryResponse},
    pipeline::read::{
        pipeline_runner::{PipelineContext, PipelineRunner},
        search_modes::{MetadataFilter, VersionFilter},
    },
};

use crate::output::OutputFormat;

// ---------------------------------------------------------------------------
// CLI argument types
// ---------------------------------------------------------------------------

/// Arguments for `fixonce query`.
#[derive(Debug, Args)]
pub struct QueryArgs {
    /// The text to search for.
    pub text: String,

    /// Use the deep pipeline (multi-query → `HyDE` → hybrid → RRR → confidence →
    /// reranking → coverage).  Default pipeline: query rewriting → hybrid →
    /// reranking.
    #[arg(long, default_value_t = false)]
    pub deep: bool,

    /// Filter by a version metadata field, e.g. `compact_compiler=0.15`.
    /// May be specified multiple times.
    #[arg(long = "version", value_name = "KEY=VALUE")]
    pub version_filters: Vec<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Maximum number of results to return.
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
}

// ---------------------------------------------------------------------------
// Formatters
// ---------------------------------------------------------------------------

/// Format query results as plain text.
fn format_results_text(hits: &[fixonce_core::memory::types::SearchHit], degraded: bool) -> String {
    use std::fmt::Write as _;

    if hits.is_empty() {
        return "No matching memories found.\n".to_owned();
    }

    let mut out = String::new();

    if degraded {
        let _ = writeln!(
            out,
            "note: unranked — Claude unavailable (showing results sorted by decay_score)\n"
        );
    }

    let _ = writeln!(
        out,
        "{} result{} found:\n",
        hits.len(),
        if hits.len() == 1 { "" } else { "s" }
    );

    for (i, hit) in hits.iter().enumerate() {
        let _ = writeln!(
            out,
            "{}. [{:.4}] {} (id={})",
            i + 1,
            hit.similarity,
            hit.memory.title,
            hit.memory.id,
        );
        let _ = writeln!(out, "   Type   : {}", hit.memory.memory_type);
        let _ = writeln!(out, "   Decay  : {:.4}", hit.memory.decay_score);
        if !hit.memory.summary.is_empty() {
            let _ = writeln!(out, "   Summary: {}", hit.memory.summary);
        }
        let _ = writeln!(out);
    }

    out
}

/// Format query results as TOON (Token-Optimised Output Notation).
fn format_results_toon(hits: &[fixonce_core::memory::types::SearchHit], degraded: bool) -> String {
    use std::fmt::Write as _;

    if hits.is_empty() {
        return "[QUERY: no results]\n".to_owned();
    }

    let mut out = String::new();
    if degraded {
        let _ = writeln!(out, "[QUERY:degraded unranked]");
    } else {
        let _ = writeln!(out, "[QUERY:ranked]");
    }

    for hit in hits {
        let _ = writeln!(
            out,
            "[MEM id={} type={} decay={:.3}] {}",
            hit.memory.id, hit.memory.memory_type, hit.memory.decay_score, hit.memory.summary,
        );
    }

    out
}

// ---------------------------------------------------------------------------
// Degraded-mode sort
// ---------------------------------------------------------------------------

/// Sort hits by `decay_score` descending (highest-quality first).
fn sort_by_decay(hits: &mut [fixonce_core::memory::types::SearchHit]) {
    hits.sort_by(|a, b| {
        b.memory
            .decay_score
            .partial_cmp(&a.memory.decay_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Build version filters from auto-detected Midnight ecosystem versions.
///
/// Called when the user has not supplied any explicit `--version` flags.
fn auto_version_filters() -> Vec<VersionFilter> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let detected = detect_midnight_versions(&cwd);
    let mut filters: Vec<VersionFilter> = Vec::new();

    if let Some(ref v) = detected.compact_compiler {
        filters.push(VersionFilter {
            key: "compact_compiler".to_owned(),
            value: v.clone(),
        });
    }
    if let Some(ref v) = detected.midnight_js {
        filters.push(VersionFilter {
            key: "midnight_js".to_owned(),
            value: v.clone(),
        });
    }
    if let Some(ref v) = detected.compact_pragma {
        filters.push(VersionFilter {
            key: "compact_pragma".to_owned(),
            value: v.clone(),
        });
    }

    filters
}

/// Execute `fixonce query`.
///
/// # Errors
///
/// Propagates errors from token loading, embedding generation, or API calls.
pub async fn run_query(api_url: &str, args: QueryArgs) -> Result<()> {
    // 1. Load token and build API client.
    let mgr = TokenManager::new();
    let token = mgr
        .load_token()
        .context("Failed to read authentication token")?
        .context("Not authenticated — run `fixonce login` first")?;
    let client = ApiClient::new(api_url)
        .context("Failed to create API client")?
        .with_token(&token);

    // 2. Generate embedding for the query text.
    let voyage_key = get_secret(&client, "VOYAGE_API_KEY")
        .await
        .context("Failed to retrieve VoyageAI API key from secrets")?;
    let voyage = VoyageClient::new().context("Failed to create VoyageAI client")?;
    let _embedding = voyage
        .generate_embedding(&voyage_key, &args.text)
        .await
        .context("Failed to generate query embedding")?;

    // 3. Execute initial search directly (the pipeline stages record intent
    //    but actual API calls need the client injected here).
    let search_req = SearchMemoryRequest {
        query: args.text.clone(),
        limit: Some(args.limit),
        threshold: None,
        language: None,
    };

    let mut search_resp: SearchMemoryResponse = match search_memories(&client, &search_req).await {
        Ok(r) => r,
        Err(e) => {
            return Err(anyhow::Error::from(e).context("Search failed"));
        }
    };

    // 4. Build version filters.
    //
    // Explicit `--version` flags are honoured as-is.  When no explicit filters
    // are provided the detected Midnight ecosystem versions are injected
    // automatically as defaults (T225), allowing queries to be scoped to the
    // current project environment without extra flags.
    let explicit_filters: Vec<VersionFilter> = args
        .version_filters
        .iter()
        .filter_map(|s| VersionFilter::parse(s))
        .collect();

    let version_filters: Vec<VersionFilter> = if explicit_filters.is_empty() {
        auto_version_filters()
    } else {
        explicit_filters
    };

    if !version_filters.is_empty() {
        let filter = MetadataFilter::new(version_filters);
        search_resp.hits.retain(|hit| filter.matches(hit));
        search_resp.total = search_resp.hits.len();
    }

    // 5. Build and run the read pipeline.
    let mut ctx = PipelineContext::new(&args.text);
    ctx.results = search_resp.hits.clone();

    let runner = if args.deep {
        PipelineRunner::deep_pipeline()
    } else {
        PipelineRunner::default_pipeline()
    };

    runner.run(&mut ctx).await.context("Read pipeline failed")?;

    // 6. Determine final result set.
    let degraded = ctx.degraded;
    let mut final_hits: Vec<fixonce_core::memory::types::SearchHit> = if ctx.results.is_empty() {
        search_resp.hits
    } else {
        ctx.results
    };

    // In degraded mode sort by decay_score (EC-29).
    if degraded {
        sort_by_decay(&mut final_hits);
    }

    // 7. Format and print.
    match args.format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "total": final_hits.len(),
                "degraded": degraded,
                "hits": final_hits,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).unwrap_or_default()
            );
        }
        OutputFormat::Toon => {
            print!("{}", format_results_toon(&final_hits, degraded));
        }
        OutputFormat::Text => {
            print!("{}", format_results_text(&final_hits, degraded));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fixonce_core::memory::types::{
        EmbeddingStatus, Memory, MemoryType, PipelineStatus, SearchHit, SourceType,
    };

    fn sample_hit(id: &str, decay: f64, similarity: f64) -> SearchHit {
        SearchHit {
            memory: Memory {
                id: id.to_owned(),
                title: format!("Memory {id}"),
                content: "some content".to_owned(),
                summary: "short summary".to_owned(),
                memory_type: MemoryType::Gotcha,
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
                decay_score: decay,
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

    // --- Degraded mode sorting ---

    #[test]
    fn sort_by_decay_orders_highest_first() {
        let mut hits = vec![
            sample_hit("low", 0.3, 0.7),
            sample_hit("high", 0.9, 0.5),
            sample_hit("mid", 0.6, 0.6),
        ];
        sort_by_decay(&mut hits);
        assert_eq!(hits[0].memory.id, "high");
        assert_eq!(hits[1].memory.id, "mid");
        assert_eq!(hits[2].memory.id, "low");
    }

    #[test]
    fn sort_by_decay_empty_does_not_panic() {
        let mut hits: Vec<SearchHit> = vec![];
        sort_by_decay(&mut hits);
    }

    // --- Text formatting ---

    #[test]
    fn format_text_no_results() {
        let text = format_results_text(&[], false);
        assert!(text.contains("No matching"));
    }

    #[test]
    fn format_text_with_results() {
        let hits = vec![sample_hit("id-1", 0.9, 0.85)];
        let text = format_results_text(&hits, false);
        assert!(text.contains("Memory id-1"));
        assert!(text.contains("1 result"));
    }

    #[test]
    fn format_text_degraded_shows_warning() {
        let hits = vec![sample_hit("id-1", 0.9, 0.85)];
        let text = format_results_text(&hits, true);
        assert!(text.contains("unranked"));
        assert!(text.contains("Claude unavailable"));
    }

    // --- TOON formatting ---

    #[test]
    fn format_toon_no_results() {
        let text = format_results_toon(&[], false);
        assert!(text.contains("no results"));
    }

    #[test]
    fn format_toon_with_results() {
        let hits = vec![sample_hit("id-1", 0.9, 0.85)];
        let text = format_results_toon(&hits, false);
        assert!(text.contains("[MEM id=id-1"));
        assert!(text.contains("[QUERY:ranked]"));
    }

    #[test]
    fn format_toon_degraded() {
        let hits = vec![sample_hit("id-1", 0.9, 0.85)];
        let text = format_results_toon(&hits, true);
        assert!(text.contains("[QUERY:degraded"));
    }

    // --- Version filter parsing ---

    #[test]
    fn version_filter_parses_in_query_args() {
        let filters: Vec<VersionFilter> = ["compact_compiler=0.15", "language=rust"]
            .iter()
            .filter_map(|s| VersionFilter::parse(s))
            .collect();
        assert_eq!(filters.len(), 2);
        assert_eq!(filters[0].key, "compact_compiler");
        assert_eq!(filters[1].key, "language");
    }
}
