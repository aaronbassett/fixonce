//! Shared output format selection for CLI commands.
//!
//! All commands that produce memory data accept a `--format` flag whose value
//! is one of these variants.  The default is [`OutputFormat::Text`].

/// The display format for a command's output.
#[derive(Debug, Clone, clap::ValueEnum, Default)]
pub enum OutputFormat {
    /// Human-readable, multi-line plain text (default)
    #[default]
    Text,
    /// Pretty-printed JSON
    Json,
    /// TOON — Token-Optimised Output Notation for LLM context injection
    Toon,
}
