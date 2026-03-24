//! `fixonce analyze` — extract learnable memories from a Claude Code session log.
//!
//! # Workflow
//!
//! 1. Read the session log file (EC-39: warn if > 100 MB).
//! 2. Parse the log format (EC-40: error on unrecognised format).
//! 3. Send the transcript to Claude with a structured extraction prompt.
//! 4. Parse the JSON candidates returned by Claude.
//! 5. In TTY mode: interactive accept / edit / skip / reject loop.
//!    In non-TTY mode: print candidates as structured data.
//! 6. Accepted candidates are printed as `fixonce create` invocations
//!    (or, when an API client is wired in, enter the write pipeline directly).

use std::io::{self, IsTerminal as _, Write as _};
use std::path::Path;

use anyhow::{Context, Result};
use fixonce_core::{
    memory::types::MemoryType,
    pipeline::{claude::ClaudeClient, PipelineError},
};

use crate::output::OutputFormat;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Warn the user when the session log exceeds this size (EC-39).
const LARGE_LOG_THRESHOLD_BYTES: u64 = 100 * 1024 * 1024; // 100 MB

/// Supported session log formats.
const SUPPORTED_FORMATS: &[&str] = &["claude-code-jsonl", "plain-text"];

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A memory candidate extracted from a session transcript.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalysisCandidate {
    pub title: String,
    pub content: String,
    pub summary: String,
    pub memory_type: MemoryType,
    /// Confidence score in `[0.0, 1.0]`.
    pub confidence: f64,
}

// ---------------------------------------------------------------------------
// Log format detection
// ---------------------------------------------------------------------------

/// Recognised session log formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogFormat {
    /// Claude Code JSONL format (`{"type":"...","content":"..."}` per line).
    ClaudeCodeJsonl,
    /// Plain text (no structured format).
    PlainText,
}

/// Detect the log format from the first few bytes (EC-40).
///
/// Returns `Err` when no known format is detected.
fn detect_log_format(content: &str) -> Result<LogFormat> {
    // Claude Code JSONL: each non-empty line is a JSON object.
    let first_nonempty = content.lines().find(|l| !l.trim().is_empty());
    if let Some(line) = first_nonempty {
        if line.trim_start().starts_with('{')
            && serde_json::from_str::<serde_json::Value>(line).is_ok()
        {
            return Ok(LogFormat::ClaudeCodeJsonl);
        }
    }

    // Plain text: always accepted as a fallback.
    if !content.is_empty() {
        return Ok(LogFormat::PlainText);
    }

    anyhow::bail!(
        "Unrecognised session log format. \
         Supported formats: {}",
        SUPPORTED_FORMATS.join(", ")
    )
}

// ---------------------------------------------------------------------------
// Transcript extraction
// ---------------------------------------------------------------------------

/// Extract the human-readable transcript from a JSONL session log.
///
/// Concatenates `content` fields from message objects where `role` is
/// `"user"` or `"assistant"`.
fn extract_jsonl_transcript(content: &str) -> String {
    let mut parts: Vec<String> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };

        // Support `{"type":"message","role":"user","content":"..."}` shape
        // and the simplified `{"role":"user","content":"..."}` shape.
        let role = obj.get("role").and_then(|v| v.as_str()).unwrap_or_default();

        if !matches!(role, "user" | "assistant") {
            continue;
        }

        let text = match obj.get("content") {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Array(arr)) => {
                // Content blocks array: extract `text` from each block.
                arr.iter()
                    .filter_map(|block| {
                        block
                            .get("text")
                            .and_then(|t| t.as_str())
                            .map(ToOwned::to_owned)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            _ => continue,
        };

        if !text.is_empty() {
            parts.push(format!("[{role}]: {text}"));
        }
    }

    parts.join("\n\n")
}

/// Build the extraction prompt sent to Claude.
fn build_extraction_prompt(transcript: &str) -> String {
    format!(
        r#"You are a knowledge extraction assistant for FixOnce, a developer memory system.

Given the session transcript below, identify up to 10 learnable items that would be
valuable to store as persistent memories. Focus on:
- Corrections (mistakes discovered and fixed)
- Gotchas (surprising behaviours, version incompatibilities)
- Best practices (patterns that worked well)
- Discoveries (new API behaviours, undocumented features)
- Anti-patterns (approaches that should NOT be used)

For each candidate output a JSON object on its own line with these fields:
  title      : string  (short, ≤ 80 chars)
  content    : string  (full explanation, markdown ok)
  summary    : string  (1–2 sentences, ≤ 200 chars)
  memory_type: one of "gotcha" | "best_practice" | "correction" | "anti_pattern" | "discovery"
  confidence : number  (0.0–1.0, how confident you are this is worth storing)

Output ONLY the JSON objects, one per line — no preamble, no explanation.
If there are no learnable items, output an empty response.

Session transcript:
---
{transcript}
---"#
    )
}

/// Parse Claude's response into `AnalysisCandidate` values.
fn parse_candidates(response: &str) -> Vec<AnalysisCandidate> {
    let mut candidates = Vec::new();

    for line in response.lines() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with('{') {
            continue;
        }
        if let Ok(c) = serde_json::from_str::<AnalysisCandidate>(line) {
            candidates.push(c);
        }
    }

    candidates
}

// ---------------------------------------------------------------------------
// Output helpers
// ---------------------------------------------------------------------------

fn print_candidate_text(c: &AnalysisCandidate) {
    println!("  [{:.2}] {} ({})", c.confidence, c.title, c.memory_type);
    println!("         {}", c.summary);
}

fn print_candidate_toon(c: &AnalysisCandidate) {
    println!(
        "[CANDIDATE confidence={:.2} type={}] {}",
        c.confidence, c.memory_type, c.title
    );
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Execute `fixonce analyze`.
///
/// # Errors
///
/// | Condition | Error |
/// |-----------|-------|
/// | File not found | propagated I/O error |
/// | Log > 100 MB (EC-39) | warning printed; continue unless user declines |
/// | Unrecognised log format (EC-40) | error with supported format list |
/// | Claude unavailable | error |
#[allow(clippy::too_many_lines)]
pub async fn run_analyze(api_url: &str, session_log: &str, format: OutputFormat) -> Result<()> {
    // -----------------------------------------------------------------------
    // 1. Read the session log.
    // -----------------------------------------------------------------------
    let log_path = Path::new(session_log);
    let metadata = std::fs::metadata(log_path)
        .with_context(|| format!("Failed to read session log: {session_log}"))?;

    // EC-39: warn on large files.
    if metadata.len() > LARGE_LOG_THRESHOLD_BYTES {
        let size_mb = metadata.len() / (1024 * 1024);
        eprintln!(
            "warning: session log is {size_mb} MB (>{} MB threshold).",
            LARGE_LOG_THRESHOLD_BYTES / (1024 * 1024)
        );
        eprintln!("         Processing the full file may be slow.");
        eprintln!(
            "         Consider splitting the log and running analyze on each chunk separately."
        );

        // In TTY mode ask the user whether to continue.
        if io::stdin().is_terminal() {
            print!("Continue anyway? [y/N]: ");
            let _ = io::stdout().flush();
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .context("Failed to read user input")?;
            if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                println!("Aborting.");
                return Ok(());
            }
        }
    }

    let content = std::fs::read_to_string(log_path)
        .with_context(|| format!("Failed to read session log: {session_log}"))?;

    // -----------------------------------------------------------------------
    // 2. Detect and validate the log format (EC-40).
    // -----------------------------------------------------------------------
    let log_format = detect_log_format(&content)?;

    // -----------------------------------------------------------------------
    // 3. Extract the human-readable transcript.
    // -----------------------------------------------------------------------
    let transcript = match log_format {
        LogFormat::ClaudeCodeJsonl => extract_jsonl_transcript(&content),
        LogFormat::PlainText => content.clone(),
    };

    if transcript.is_empty() {
        println!("Session log contains no extractable content.");
        return Ok(());
    }

    // -----------------------------------------------------------------------
    // 4. Send to Claude for candidate extraction.
    // -----------------------------------------------------------------------
    let claude = ClaudeClient::new();
    let prompt = build_extraction_prompt(&transcript);

    let response = match claude.prompt(&prompt).await {
        Ok(r) => r,
        Err(PipelineError::ClaudeNotFound) => {
            anyhow::bail!(
                "Claude CLI not found. Install it from https://claude.ai/code \
                 and ensure it is on PATH."
            );
        }
        Err(PipelineError::ClaudeTimeout { seconds }) => {
            anyhow::bail!("Claude CLI timed out after {seconds}s while analysing the transcript.");
        }
        Err(e) => {
            return Err(anyhow::Error::from(e).context("Claude failed during transcript analysis"));
        }
    };

    // -----------------------------------------------------------------------
    // 5. Parse candidates.
    // -----------------------------------------------------------------------
    let candidates = parse_candidates(&response);

    if candidates.is_empty() {
        println!("No learnable memories found in this session transcript.");
        return Ok(());
    }

    // -----------------------------------------------------------------------
    // 6. Present candidates.
    // -----------------------------------------------------------------------
    let is_tty = io::stdin().is_terminal();

    if is_tty {
        // Interactive mode: accept / skip / reject.
        println!("Found {} candidate(s).", candidates.len());
        let mut accepted: Vec<&AnalysisCandidate> = Vec::new();

        let total = candidates.len();
        let mut quit = false;

        for (i, c) in candidates.iter().enumerate() {
            if quit {
                break;
            }

            // Re-read the decision (quit check).
            print!(
                "\n[{}/{}] Candidate (confidence={:.2}): {}",
                i + 1,
                total,
                c.confidence,
                c.title
            );
            println!("\n  Type   : {}", c.memory_type);
            println!("  Summary: {}", c.summary);
            println!("  Content:\n    {}", c.content.replace('\n', "\n    "));
            print!("\nAccept? [y/n/q(quit)]: ");
            let _ = io::stdout().flush();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                break;
            }
            match input.trim().to_lowercase().as_str() {
                "y" | "yes" => accepted.push(c),
                "q" | "quit" => quit = true,
                _ => {} // skip
            }
        }

        if accepted.is_empty() {
            println!("No candidates accepted.");
        } else {
            println!("\n{} candidate(s) accepted.", accepted.len());
            println!("Run the following commands to store them (or pipe to your shell):\n");
            for c in &accepted {
                println!(
                    "fixonce --api-url {api_url} create \\\n  \
                     --title {:?} \\\n  \
                     --content {:?} \\\n  \
                     --summary {:?} \\\n  \
                     --type {} \\\n  \
                     --source harvested",
                    c.title, c.content, c.summary, c.memory_type
                );
                println!();
            }
        }
    } else {
        // Non-TTY: output structured data.
        match format {
            OutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&candidates).unwrap_or_default()
                );
            }
            OutputFormat::Toon => {
                for c in &candidates {
                    print_candidate_toon(c);
                }
            }
            OutputFormat::Text => {
                println!("Found {} candidate(s):", candidates.len());
                for c in &candidates {
                    print_candidate_text(c);
                }
            }
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

    // --- Log format detection (EC-40) ---

    #[test]
    fn detects_jsonl_format() {
        let content = r#"{"role":"user","content":"hello"}"#;
        let fmt = detect_log_format(content).expect("must detect");
        assert_eq!(fmt, LogFormat::ClaudeCodeJsonl);
    }

    #[test]
    fn detects_plain_text_format() {
        let content = "This is a plain text session log.\nNo JSON here.";
        let fmt = detect_log_format(content).expect("must detect");
        assert_eq!(fmt, LogFormat::PlainText);
    }

    #[test]
    fn empty_content_returns_error() {
        let result = detect_log_format("");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Unrecognised"),
            "expected unrecognised error, got: {msg}"
        );
        assert!(
            msg.contains("claude-code-jsonl"),
            "expected supported formats listed"
        );
    }

    // --- JSONL extraction ---

    #[test]
    fn extracts_user_and_assistant_turns_from_jsonl() {
        let log = r#"{"role":"user","content":"What is compact?"}
{"role":"assistant","content":"Compact is a DSL for Midnight contracts."}
{"role":"user","content":"Thanks!"}"#;
        let transcript = extract_jsonl_transcript(log);
        assert!(transcript.contains("[user]: What is compact?"));
        assert!(transcript.contains("[assistant]: Compact is a DSL for Midnight contracts."));
        assert!(transcript.contains("[user]: Thanks!"));
    }

    #[test]
    fn skips_non_message_jsonl_lines() {
        let log = r#"{"type":"tool_use","name":"bash","input":"ls"}
{"role":"user","content":"hello"}"#;
        let transcript = extract_jsonl_transcript(log);
        assert!(!transcript.contains("tool_use"));
        assert!(transcript.contains("[user]: hello"));
    }

    #[test]
    fn handles_content_blocks_array() {
        let log = r#"{"role":"assistant","content":[{"type":"text","text":"block content"}]}"#;
        let transcript = extract_jsonl_transcript(log);
        assert!(transcript.contains("block content"));
    }

    #[test]
    fn empty_jsonl_returns_empty_transcript() {
        let transcript = extract_jsonl_transcript("");
        assert!(transcript.is_empty());
    }

    // --- Candidate parsing ---

    #[test]
    fn parses_valid_candidate_json_lines() {
        let response = r#"{"title":"Use pragma compiler","content":"Always declare pragma compiler version.","summary":"pragma compiler version required","memory_type":"best_practice","confidence":0.9}
{"title":"Don't use deprecated API","content":"The old API is broken.","summary":"deprecated API broken","memory_type":"gotcha","confidence":0.7}"#;

        let candidates = parse_candidates(response);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].title, "Use pragma compiler");
        assert_eq!(candidates[0].memory_type, MemoryType::BestPractice);
        assert!((candidates[0].confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(candidates[1].memory_type, MemoryType::Gotcha);
    }

    #[test]
    fn skips_non_json_lines_in_response() {
        let response = r#"Here are my findings:
{"title":"A gotcha","content":"details","summary":"short","memory_type":"gotcha","confidence":0.8}
And that's all."#;
        let candidates = parse_candidates(response);
        assert_eq!(candidates.len(), 1);
    }

    #[test]
    fn returns_empty_vec_for_empty_response() {
        let candidates = parse_candidates("");
        assert!(candidates.is_empty());
    }

    #[test]
    fn returns_empty_vec_for_no_json_response() {
        let candidates = parse_candidates("No learnable memories found.");
        assert!(candidates.is_empty());
    }

    // --- Extraction prompt ---

    #[test]
    fn extraction_prompt_contains_transcript() {
        let prompt = build_extraction_prompt("some session content here");
        assert!(prompt.contains("some session content here"));
        assert!(prompt.contains("memory_type"));
        assert!(prompt.contains("confidence"));
    }

    // --- Candidate serialisation ---

    #[test]
    fn candidate_serialises_and_deserialises() {
        let c = AnalysisCandidate {
            title: "test".to_owned(),
            content: "full content".to_owned(),
            summary: "summary".to_owned(),
            memory_type: MemoryType::Correction,
            confidence: 0.85,
        };
        let json = serde_json::to_string(&c).expect("must serialise");
        let back: AnalysisCandidate = serde_json::from_str(&json).expect("must deserialise");
        assert_eq!(back.title, "test");
        assert_eq!(back.memory_type, MemoryType::Correction);
    }
}
