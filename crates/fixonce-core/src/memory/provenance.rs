//! Source provenance information attached to a memory.

use serde::{Deserialize, Serialize};

/// Tracks where a memory originated so it can be correlated with source
/// material, PRs, tasks, or interactive sessions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Provenance {
    /// URL of the primary source (e.g. a documentation page or forum thread).
    pub source_url: Option<String>,
    /// Git repository URL where the issue was observed.
    pub repo_url: Option<String>,
    /// Short description of the task or ticket that surfaced this memory.
    pub task_summary: Option<String>,
    /// Identifier for the interactive session during which the memory was created.
    pub session_id: Option<String>,
}
