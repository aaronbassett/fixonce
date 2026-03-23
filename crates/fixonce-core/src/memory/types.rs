//! Core memory model types and API request/response shapes.

use serde::{Deserialize, Serialize};

use crate::memory::metadata::VersionMetadata;

/// Anti-memory: explicitly marks something as WRONG or DANGEROUS.
///
/// Present on memories whose [`MemoryType`] is [`MemoryType::AntiPattern`].
/// During search, anti-memories receive a priority boost when their
/// [`version_constraints`](AntiMemory::version_constraints) match the
/// requester's environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiMemory {
    /// Human-readable description of the wrong / dangerous pattern.
    pub description: String,
    /// Why the pattern is harmful.
    pub reason: String,
    /// A suggested alternative, if one exists.
    pub alternative: Option<String>,
    /// Optional version constraints; when set, the anti-memory is only boosted
    /// when the caller's version metadata matches.
    pub version_constraints: Option<VersionMetadata>,
}

/// A memory record as returned by the `FixOnce` API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub title: String,
    pub content: String,
    pub summary: String,
    pub memory_type: MemoryType,
    pub source_type: SourceType,
    pub language: Option<String>,
    pub compact_pragma: Option<String>,
    pub compact_compiler: Option<String>,
    pub midnight_js: Option<String>,
    pub indexer_version: Option<String>,
    pub node_version: Option<String>,
    pub source_url: Option<String>,
    pub repo_url: Option<String>,
    pub task_summary: Option<String>,
    pub session_id: Option<String>,
    pub decay_score: f64,
    pub reinforcement_score: f64,
    pub last_accessed_at: Option<String>,
    pub embedding_status: EmbeddingStatus,
    pub pipeline_status: PipelineStatus,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: String,
    /// Anti-memory payload; present when `memory_type == AntiPattern`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anti_memory: Option<AntiMemory>,
}

/// Classification of the kind of knowledge a memory represents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Gotcha,
    BestPractice,
    Correction,
    AntiPattern,
    Discovery,
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Gotcha => "gotcha",
            Self::BestPractice => "best_practice",
            Self::Correction => "correction",
            Self::AntiPattern => "anti_pattern",
            Self::Discovery => "discovery",
        };
        f.write_str(s)
    }
}

/// How the memory was originally captured.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Correction,
    Observation,
    PrFeedback,
    Manual,
    Harvested,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Correction => "correction",
            Self::Observation => "observation",
            Self::PrFeedback => "pr_feedback",
            Self::Manual => "manual",
            Self::Harvested => "harvested",
        };
        f.write_str(s)
    }
}

/// Whether the embedding vector has been computed and stored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingStatus {
    Complete,
    Pending,
    Failed,
}

impl std::fmt::Display for EmbeddingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Complete => "complete",
            Self::Pending => "pending",
            Self::Failed => "failed",
        };
        f.write_str(s)
    }
}

/// Whether all post-creation pipeline steps have finished.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    Complete,
    Incomplete,
}

impl std::fmt::Display for PipelineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Complete => "complete",
            Self::Incomplete => "incomplete",
        };
        f.write_str(s)
    }
}

/// User feedback record for a memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    pub id: String,
    pub memory_id: String,
    pub user_id: String,
    pub rating: FeedbackRating,
    pub context: Option<String>,
    pub created_at: String,
}

/// How the user rated the memory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackRating {
    Helpful,
    Outdated,
    Damaging,
}

impl std::fmt::Display for FeedbackRating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Helpful => "helpful",
            Self::Outdated => "outdated",
            Self::Damaging => "damaging",
        };
        f.write_str(s)
    }
}

// ---------------------------------------------------------------------------
// API request / response shapes
// ---------------------------------------------------------------------------

/// Payload sent to the create-memory endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMemoryRequest {
    pub title: String,
    pub content: String,
    pub summary: String,
    pub memory_type: MemoryType,
    pub source_type: SourceType,
    pub language: Option<String>,
    /// Optional pre-computed 1 024-dimensional embedding vector.
    ///
    /// When supplied the backend stores it directly and marks `embedding_status`
    /// as `complete`, skipping the async pipeline step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f64>>,
    pub compact_pragma: Option<String>,
    pub compact_compiler: Option<String>,
    pub midnight_js: Option<String>,
    pub indexer_version: Option<String>,
    pub node_version: Option<String>,
    pub source_url: Option<String>,
    pub repo_url: Option<String>,
    pub task_summary: Option<String>,
    pub session_id: Option<String>,
}

/// Response returned after a memory is created.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMemoryResponse {
    pub id: String,
    pub created_at: String,
}

/// Payload sent to update a memory (partial — use [`serde_json::Value`] for ad-hoc patches).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMemoryResponse {
    pub id: String,
    pub updated_at: String,
}

/// Response after a soft-delete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMemoryResponse {
    pub id: String,
    pub deleted_at: String,
}

/// Payload sent to the vector-search endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMemoryRequest {
    /// The query text to embed and search against.
    pub query: String,
    /// Maximum number of results to return (default determined by backend).
    pub limit: Option<u32>,
    /// Minimum cosine similarity threshold (0..1).
    pub threshold: Option<f64>,
    /// Optional language filter.
    pub language: Option<String>,
}

/// A single hit in a similarity search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub memory: Memory,
    pub similarity: f64,
}

/// Response from the vector-search endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMemoryResponse {
    pub hits: Vec<SearchHit>,
    pub total: usize,
}
