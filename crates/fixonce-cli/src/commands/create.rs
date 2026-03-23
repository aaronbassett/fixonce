/// `fixonce create` — create a new memory.
///
/// The write pipeline runs in this order before the memory is stored:
///
///   1. **Credential check** — reject if PII / secrets are detected.
///   2. **Quality gate** — reject if Claude deems the content low-signal.
///      On Claude CLI timeout or unavailability the memory is stored with
///      `pipeline_status = incomplete` (EC-26, EC-37).
///   3. **Dedup** — compare against similar memories and handle the outcome.
///      Skipped when Claude is unavailable (stores with `incomplete`).
///   4. **Enrichment** — apply language / type suggestions and print warnings.
use anyhow::{Context, Result};
use clap::Args;
use fixonce_core::{
    api::{memories::create_memory, secrets::get_secret, ApiClient},
    auth::token::TokenManager,
    embeddings::VoyageClient,
    memory::types::{CreateMemoryRequest, MemoryType, SourceType},
    pipeline::{
        claude::ClaudeClient,
        write::{
            credential_check::check_for_credentials,
            dedup::{dedup_check, DedupOutcome},
            enrichment::enrich_metadata,
            quality_gate::quality_gate,
        },
        PipelineError,
    },
};

use crate::output::OutputFormat;

/// Arguments for `fixonce create`.
#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Memory title
    #[arg(long)]
    pub title: String,

    /// Full memory content
    #[arg(long)]
    pub content: String,

    /// Short summary (≤ 2 000 chars)
    #[arg(long)]
    pub summary: String,

    /// Memory type
    #[arg(long, value_name = "TYPE")]
    pub r#type: MemoryTypeArg,

    /// Source type
    #[arg(long, value_name = "SOURCE")]
    pub source: SourceTypeArg,

    /// Programming language tag (optional)
    #[arg(long)]
    pub language: Option<String>,

    /// Skip the Claude-powered pipeline stages (quality gate & dedup).
    ///
    /// Useful when the Claude CLI is not available or when running in
    /// automated / CI contexts.
    #[arg(long, default_value_t = false)]
    pub skip_pipeline: bool,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

/// Clap-friendly wrapper for [`MemoryType`].
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum MemoryTypeArg {
    Gotcha,
    BestPractice,
    Correction,
    AntiPattern,
    Discovery,
}

impl From<MemoryTypeArg> for MemoryType {
    fn from(a: MemoryTypeArg) -> Self {
        match a {
            MemoryTypeArg::Gotcha => MemoryType::Gotcha,
            MemoryTypeArg::BestPractice => MemoryType::BestPractice,
            MemoryTypeArg::Correction => MemoryType::Correction,
            MemoryTypeArg::AntiPattern => MemoryType::AntiPattern,
            MemoryTypeArg::Discovery => MemoryType::Discovery,
        }
    }
}

/// Clap-friendly wrapper for [`SourceType`].
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum SourceTypeArg {
    Correction,
    Observation,
    PrFeedback,
    Manual,
    Harvested,
}

impl From<SourceTypeArg> for SourceType {
    fn from(a: SourceTypeArg) -> Self {
        match a {
            SourceTypeArg::Correction => SourceType::Correction,
            SourceTypeArg::Observation => SourceType::Observation,
            SourceTypeArg::PrFeedback => SourceType::PrFeedback,
            SourceTypeArg::Manual => SourceType::Manual,
            SourceTypeArg::Harvested => SourceType::Harvested,
        }
    }
}

/// Whether the Claude-dependent pipeline stages ran to completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaudePipelineStatus {
    Complete,
    Incomplete,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Run the quality-gate stage, degrading gracefully when Claude is unavailable.
///
/// Returns `Ok(Complete)` when the memory passes, `Ok(Incomplete)` when Claude
/// could not be reached (EC-26 / EC-37), or an error when the memory is
/// rejected.
async fn run_quality_gate(
    claude: &ClaudeClient,
    req: &CreateMemoryRequest,
) -> Result<ClaudePipelineStatus> {
    match quality_gate(claude, &req.title, &req.content, &req.summary).await {
        Ok(result) => {
            if result.accepted {
                Ok(ClaudePipelineStatus::Complete)
            } else {
                anyhow::bail!(
                    "Quality gate rejected this memory.\n\
                     Rationale: {}\n\
                     Scores — actionability: {:.2}, specificity: {:.2}, \
                     signal-to-noise: {:.2}",
                    result.rationale,
                    result.scores.actionability,
                    result.scores.specificity,
                    result.scores.signal_to_noise,
                )
            }
        }
        Err(PipelineError::ClaudeNotFound) => {
            eprintln!(
                "warning: Claude CLI not found — skipping quality gate. \
                 Memory will be stored with pipeline_status=incomplete."
            );
            Ok(ClaudePipelineStatus::Incomplete)
        }
        Err(PipelineError::ClaudeTimeout { seconds }) => {
            eprintln!(
                "warning: Claude CLI timed out after {seconds}s — skipping quality gate. \
                 Memory will be stored with pipeline_status=incomplete."
            );
            Ok(ClaudePipelineStatus::Incomplete)
        }
        Err(e) => Err(anyhow::Error::from(e).context("Quality gate failed")),
    }
}

/// Run the dedup stage, degrading gracefully when Claude is unavailable.
///
/// Returns `Ok(Complete)` when dedup succeeds (even if the outcome is
/// non-trivial), `Ok(Incomplete)` on Claude unavailability, or an error when
/// the memory must be discarded.
async fn run_dedup(
    claude: &ClaudeClient,
    client: &ApiClient,
    req: &CreateMemoryRequest,
    embedding: &[f64],
) -> Result<ClaudePipelineStatus> {
    match dedup_check(claude, client, req, embedding).await {
        Ok(result) => match result.outcome {
            DedupOutcome::New => Ok(ClaudePipelineStatus::Complete),
            DedupOutcome::Discard => anyhow::bail!(
                "Dedup: this memory is already covered by an existing entry.\n\
                 Rationale: {}",
                result.rationale
            ),
            DedupOutcome::Replace(id) => {
                eprintln!(
                    "info: dedup recommends replacing memory {id}. \
                     Rationale: {}",
                    result.rationale
                );
                Ok(ClaudePipelineStatus::Complete)
            }
            DedupOutcome::Update(id) => {
                eprintln!(
                    "info: dedup recommends updating memory {id}. \
                     Rationale: {}",
                    result.rationale
                );
                Ok(ClaudePipelineStatus::Complete)
            }
            DedupOutcome::Merge(id) => {
                eprintln!(
                    "info: dedup recommends merging with memory {id}. \
                     Rationale: {}",
                    result.rationale
                );
                Ok(ClaudePipelineStatus::Complete)
            }
        },
        Err(PipelineError::ClaudeNotFound | PipelineError::ClaudeTimeout { .. }) => {
            eprintln!(
                "warning: Claude CLI unavailable for dedup — \
                 memory will be stored with pipeline_status=incomplete."
            );
            Ok(ClaudePipelineStatus::Incomplete)
        }
        Err(e) => Err(anyhow::Error::from(e).context("Dedup check failed")),
    }
}

// ---------------------------------------------------------------------------
// Main command entry point
// ---------------------------------------------------------------------------

/// Execute `fixonce create`.
///
/// # Errors
///
/// Propagates errors from token loading, embedding generation, or the
/// create-memory API call.  Credential check failures and quality-gate
/// rejections are returned as normal (non-panic) errors with clear messages.
pub async fn run_create(api_url: &str, args: CreateArgs) -> Result<()> {
    // 1. Credential check (always runs — no I/O)
    let cred_matches = check_for_credentials(&args.content);
    if !cred_matches.is_empty() {
        let details: String = cred_matches
            .iter()
            .map(|m| {
                format!(
                    "  • line {}: {} — \"{}\"",
                    m.line, m.credential_type, m.pattern
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        anyhow::bail!(
            "Credential check failed — the content appears to contain sensitive information.\n\
             Remove the following before storing:\n{details}"
        );
    }

    // 2. Authenticate and build API client
    let mgr = TokenManager::new();
    let token = mgr
        .load_token()
        .context("Failed to read authentication token")?
        .context("Not authenticated — run `fixonce login` first")?;
    let client = ApiClient::new(api_url)
        .context("Failed to create API client")?
        .with_token(&token);

    // 3. Generate embedding
    let voyage_key = get_secret(&client, "VOYAGE_API_KEY")
        .await
        .context("Failed to retrieve VoyageAI API key from secrets")?;
    let voyage = VoyageClient::new().context("Failed to create VoyageAI client")?;
    let embedding = voyage
        .generate_embedding(&voyage_key, &args.content)
        .await
        .context("Failed to generate embedding")?;

    // 4. Build the initial request
    let mut req = CreateMemoryRequest {
        title: args.title.clone(),
        content: args.content.clone(),
        summary: args.summary.clone(),
        memory_type: args.r#type.into(),
        source_type: args.source.into(),
        language: args.language.clone(),
        compact_pragma: None,
        compact_compiler: None,
        midnight_js: None,
        indexer_version: None,
        node_version: None,
        source_url: None,
        repo_url: None,
        task_summary: None,
        session_id: None,
        embedding: Some(embedding.clone()),
    };

    // 5. Enrichment (pure heuristics — always runs, no I/O)
    let enrichment = enrich_metadata(&args.content, &req);
    if req.language.is_none() {
        req.language = enrichment.suggested_language.clone();
    }
    for warning in &enrichment.missing_metadata_warnings {
        eprintln!("warning: {warning}");
    }
    if let Some(ref suggested_type) = enrichment.suggested_memory_type {
        eprintln!("hint: content looks like a '{suggested_type}' memory — consider setting --type");
    }

    // 6. Claude-powered stages (quality gate + dedup)
    let mut claude_status = ClaudePipelineStatus::Complete;

    if !args.skip_pipeline {
        let claude = ClaudeClient::new();

        let qg_status = run_quality_gate(&claude, &req).await?;
        if qg_status == ClaudePipelineStatus::Complete {
            let dedup_status = run_dedup(&claude, &client, &req, &embedding).await?;
            if dedup_status == ClaudePipelineStatus::Incomplete {
                claude_status = ClaudePipelineStatus::Incomplete;
            }
        } else {
            claude_status = ClaudePipelineStatus::Incomplete;
        }
    }

    // 7. Store the memory
    let resp = create_memory(&client, &req)
        .await
        .context("Failed to create memory")?;

    // 8. Format and print result
    let pipeline_note = if claude_status == ClaudePipelineStatus::Incomplete {
        " (pipeline_status=incomplete — re-run with Claude CLI available)"
    } else {
        ""
    };

    match args.format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&resp).unwrap_or_default()
            );
        }
        OutputFormat::Text | OutputFormat::Toon => {
            println!("Memory created.{pipeline_note}");
            println!("  id         : {}", resp.id);
            println!("  created_at : {}", resp.created_at);
        }
    }

    Ok(())
}
