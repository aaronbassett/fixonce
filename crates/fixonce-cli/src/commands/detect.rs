//! `fixonce detect` — scan the current project for Midnight ecosystem versions.
//!
//! Reads `package.json`, `.compact` files, and related config from the current
//! working directory and prints detected version strings.

use anyhow::{Context, Result};
use fixonce_core::detect::midnight::{detect_midnight_versions, MidnightVersions};

use crate::output::OutputFormat;

// ---------------------------------------------------------------------------
// Formatters
// ---------------------------------------------------------------------------

fn format_text(v: &MidnightVersions) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    let _ = writeln!(out, "Midnight ecosystem versions:");
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
    out
}

fn format_toon(v: &MidnightVersions) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    let _ = writeln!(out, "[ENV:midnight]");
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
    out
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Execute `fixonce detect`.
///
/// # Errors
///
/// Returns an error when the current working directory cannot be determined.
pub async fn run_detect(format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to determine current working directory")?;
    let versions = detect_midnight_versions(&cwd);

    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&versions).unwrap_or_default()
            );
        }
        OutputFormat::Toon => {
            print!("{}", format_toon(&versions));
        }
        OutputFormat::Text => {
            print!("{}", format_text(&versions));
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
    use fixonce_core::detect::midnight::MidnightVersions;

    fn full_versions() -> MidnightVersions {
        MidnightVersions {
            compact_pragma: Some("0.14".to_owned()),
            compact_compiler: Some("0.15.0".to_owned()),
            midnight_js: Some("1.2.3".to_owned()),
            indexer_version: Some("2.0.0".to_owned()),
            node_version: Some("20.11.0".to_owned()),
        }
    }

    #[test]
    fn text_format_includes_all_fields() {
        let text = format_text(&full_versions());
        assert!(text.contains("compact_pragma"));
        assert!(text.contains("0.14"));
        assert!(text.contains("compact_compiler"));
        assert!(text.contains("0.15.0"));
        assert!(text.contains("midnight_js"));
        assert!(text.contains("1.2.3"));
        assert!(text.contains("indexer_version"));
        assert!(text.contains("node_version"));
        assert!(text.contains("20.11.0"));
    }

    #[test]
    fn text_format_shows_not_detected_for_none_fields() {
        let text = format_text(&MidnightVersions::default());
        assert!(text.contains("(not detected)"));
    }

    #[test]
    fn toon_format_includes_detected_fields_only() {
        let toon = format_toon(&full_versions());
        assert!(toon.contains("[ENV:midnight]"));
        assert!(toon.contains("compact_pragma=0.14"));
        assert!(toon.contains("midnight_js=1.2.3"));
    }

    #[test]
    fn toon_format_omits_none_fields() {
        let mut v = MidnightVersions::default();
        v.midnight_js = Some("1.0.0".to_owned());
        let toon = format_toon(&v);
        assert!(toon.contains("midnight_js=1.0.0"));
        assert!(!toon.contains("compact_pragma"));
        assert!(!toon.contains("node_version"));
    }

    #[test]
    fn json_serialises_to_valid_json() {
        let v = full_versions();
        let json_str = serde_json::to_string_pretty(&v).expect("must serialise");
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("must be valid JSON");
        assert_eq!(parsed["midnight_js"], "1.2.3");
    }
}
