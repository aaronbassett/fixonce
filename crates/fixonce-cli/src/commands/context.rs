//! `fixonce context` — gather full project context and print it.
//!
//! Combines Midnight ecosystem version detection with git metadata (branch,
//! remote, recent commits) and a top-level file-structure snapshot.

use anyhow::{Context, Result};
use fixonce_core::detect::context::{gather_context, ProjectContext};

use crate::output::OutputFormat;

// ---------------------------------------------------------------------------
// Formatters
// ---------------------------------------------------------------------------

fn format_text(ctx: &ProjectContext) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();

    let _ = writeln!(out, "Project context:");
    let _ = writeln!(out);

    // Versions
    let _ = writeln!(out, "Midnight ecosystem:");
    let v = &ctx.versions;
    let _ = writeln!(
        out,
        "  compact_pragma   : {}",
        v.compact_pragma.as_deref().unwrap_or("(not detected)")
    );
    let _ = writeln!(
        out,
        "  compact_compiler : {}",
        v.compact_compiler.as_deref().unwrap_or("(not detected)")
    );
    let _ = writeln!(
        out,
        "  midnight_js      : {}",
        v.midnight_js.as_deref().unwrap_or("(not detected)")
    );
    let _ = writeln!(
        out,
        "  indexer_version  : {}",
        v.indexer_version.as_deref().unwrap_or("(not detected)")
    );
    let _ = writeln!(
        out,
        "  node_version     : {}",
        v.node_version.as_deref().unwrap_or("(not detected)")
    );
    let _ = writeln!(out);

    // Git
    let _ = writeln!(out, "Git:");
    let _ = writeln!(
        out,
        "  remote : {}",
        ctx.git_remote.as_deref().unwrap_or("(local only)")
    );
    let _ = writeln!(
        out,
        "  branch : {}",
        ctx.git_branch.as_deref().unwrap_or("(not a git repo)")
    );

    if ctx.recent_commits.is_empty() {
        let _ = writeln!(out, "  commits: (none)");
    } else {
        let _ = writeln!(out, "  recent commits:");
        for c in &ctx.recent_commits {
            let _ = writeln!(out, "    - {c}");
        }
    }
    let _ = writeln!(out);

    // File structure
    if ctx.file_structure.is_empty() {
        let _ = writeln!(out, "File structure: (empty)");
    } else {
        let _ = writeln!(out, "File structure:");
        for entry in &ctx.file_structure {
            let _ = writeln!(out, "  {entry}");
        }
    }

    out
}

fn format_toon(ctx: &ProjectContext) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    let _ = writeln!(out, "[CTX:project]");

    // Versions
    let v = &ctx.versions;
    if let Some(ref s) = v.compact_pragma {
        let _ = writeln!(out, "compact_pragma={s}");
    }
    if let Some(ref s) = v.compact_compiler {
        let _ = writeln!(out, "compact_compiler={s}");
    }
    if let Some(ref s) = v.midnight_js {
        let _ = writeln!(out, "midnight_js={s}");
    }
    if let Some(ref s) = v.indexer_version {
        let _ = writeln!(out, "indexer_version={s}");
    }
    if let Some(ref s) = v.node_version {
        let _ = writeln!(out, "node_version={s}");
    }

    // Git
    let remote = ctx.git_remote.as_deref().unwrap_or("local_only");
    let _ = writeln!(out, "git_remote={remote}");
    if let Some(ref b) = ctx.git_branch {
        let _ = writeln!(out, "git_branch={b}");
    }
    if !ctx.recent_commits.is_empty() {
        let _ = writeln!(out, "recent_commits={}", ctx.recent_commits.join("|"));
    }

    // File structure (one line, pipe-separated)
    if !ctx.file_structure.is_empty() {
        let _ = writeln!(out, "files={}", ctx.file_structure.join("|"));
    }

    out
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Execute `fixonce context`.
///
/// # Errors
///
/// Returns an error when the current working directory cannot be determined.
pub async fn run_context(format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to determine current working directory")?;
    let ctx = gather_context(&cwd);

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&ctx).unwrap_or_default());
        }
        OutputFormat::Toon => {
            print!("{}", format_toon(&ctx));
        }
        OutputFormat::Text => {
            print!("{}", format_text(&ctx));
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
    use fixonce_core::detect::{context::ProjectContext, midnight::MidnightVersions};

    fn sample_ctx() -> ProjectContext {
        ProjectContext {
            versions: MidnightVersions {
                compact_pragma: Some("0.14".to_owned()),
                compact_compiler: Some("0.15.0".to_owned()),
                midnight_js: Some("1.2.3".to_owned()),
                indexer_version: None,
                node_version: Some("20.0.0".to_owned()),
            },
            git_remote: Some("https://github.com/example/repo.git".to_owned()),
            git_branch: Some("main".to_owned()),
            recent_commits: vec![
                "abc1234 initial commit".to_owned(),
                "def5678 add feature".to_owned(),
            ],
            file_structure: vec!["Cargo.toml".to_owned(), "src/".to_owned()],
        }
    }

    fn local_only_ctx() -> ProjectContext {
        ProjectContext {
            versions: MidnightVersions::default(),
            git_remote: None,
            git_branch: Some("main".to_owned()),
            recent_commits: vec![],
            file_structure: vec![],
        }
    }

    // --- text format ---

    #[test]
    fn text_format_shows_all_sections() {
        let text = format_text(&sample_ctx());
        assert!(text.contains("Midnight ecosystem:"));
        assert!(text.contains("Git:"));
        assert!(text.contains("File structure:"));
    }

    #[test]
    fn text_format_shows_local_only_for_no_remote() {
        let text = format_text(&local_only_ctx());
        assert!(text.contains("(local only)"));
    }

    #[test]
    fn text_format_shows_recent_commits() {
        let text = format_text(&sample_ctx());
        assert!(text.contains("initial commit"));
        assert!(text.contains("add feature"));
    }

    #[test]
    fn text_format_shows_file_structure() {
        let text = format_text(&sample_ctx());
        assert!(text.contains("Cargo.toml"));
        assert!(text.contains("src/"));
    }

    // --- toon format ---

    #[test]
    fn toon_format_starts_with_ctx_tag() {
        let toon = format_toon(&sample_ctx());
        assert!(toon.starts_with("[CTX:project]"));
    }

    #[test]
    fn toon_format_ec38_remote_shows_local_only() {
        let toon = format_toon(&local_only_ctx());
        assert!(toon.contains("git_remote=local_only"));
    }

    #[test]
    fn toon_format_includes_recent_commits_pipe_joined() {
        let toon = format_toon(&sample_ctx());
        assert!(toon.contains("recent_commits=abc1234 initial commit|def5678 add feature"));
    }

    #[test]
    fn toon_format_omits_none_version_fields() {
        let toon = format_toon(&sample_ctx());
        assert!(!toon.contains("indexer_version="));
    }

    // --- JSON ---

    #[test]
    fn json_round_trips() {
        let ctx = sample_ctx();
        let json_str = serde_json::to_string_pretty(&ctx).expect("must serialise");
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("must be valid JSON");
        assert_eq!(parsed["git_branch"], "main");
        assert_eq!(parsed["versions"]["midnight_js"], "1.2.3");
    }
}
