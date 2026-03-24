//! Write pipeline for processing memories before storage.
//!
//! The pipeline runs in order:
//!   credential check → quality gate → dedup → enrichment
//!
//! Each stage is independently testable.  The Claude CLI wrapper lives in
//! `claude.rs` and is shared by the quality gate and dedup stages.

pub mod claude;
pub mod read;
pub mod write;

// ---------------------------------------------------------------------------
// Pipeline-level error type
// ---------------------------------------------------------------------------

/// All errors that can occur inside the write pipeline.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    /// The `claude` CLI was not found on `PATH` (EC-37).
    #[error(
        "Claude CLI not found — install it from https://claude.ai/code and ensure it is on PATH"
    )]
    ClaudeNotFound,

    /// The `claude` process did not finish within the allowed timeout (EC-26).
    #[error("Claude CLI timed out after {seconds}s")]
    ClaudeTimeout { seconds: u64 },

    /// The `claude` process exited with a non-zero status.
    #[error("Claude CLI exited with status {code}: {stderr}")]
    ClaudeExitFailure { code: i32, stderr: String },

    /// The JSON returned by `claude` could not be parsed.
    #[error("Failed to parse Claude CLI output: {0}")]
    ClaudeOutputParse(String),

    /// An I/O error occurred while spawning or communicating with the subprocess.
    #[error("I/O error communicating with Claude CLI: {0}")]
    Io(#[from] std::io::Error),

    /// A downstream API call failed during dedup.
    #[error("API error during pipeline: {0}")]
    Api(String),
}
