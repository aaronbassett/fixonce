//! Claude-powered quality gate.
//!
//! Sends a structured prompt to the Claude CLI and parses the structured JSON
//! response to decide whether a memory is worth storing.

use crate::pipeline::{claude::ClaudeClient, PipelineError};

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Numerical quality scores returned by Claude.
///
/// All scores are in the range `0.0` (worst) to `1.0` (best).
#[derive(Debug, Clone, PartialEq)]
pub struct QualityScores {
    /// Can someone act on this information right now?
    pub actionability: f64,
    /// Is this specific enough to be unambiguously useful?
    pub specificity: f64,
    /// Does the signal outweigh filler / boilerplate?
    pub signal_to_noise: f64,
}

/// The verdict returned by the quality gate.
#[derive(Debug, Clone)]
pub struct QualityResult {
    /// `true` → memory passes the gate; `false` → reject.
    pub accepted: bool,
    /// Human-readable rationale from Claude.
    pub rationale: String,
    /// Detailed scores.
    pub scores: QualityScores,
}

// ---------------------------------------------------------------------------
// JSON response shape expected from Claude
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct ClaudeQualityResponse {
    accepted: bool,
    rationale: String,
    scores: ClaudeScores,
}

#[derive(Debug, serde::Deserialize)]
struct ClaudeScores {
    actionability: f64,
    specificity: f64,
    signal_to_noise: f64,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Assess the quality of a memory using the Claude CLI.
///
/// Claude is asked to rate the memory on three axes (actionability,
/// specificity, signal-to-noise) and to emit a JSON verdict.  The caller
/// should treat the result as advisory — the pipeline wires up the accept /
/// reject behaviour.
///
/// # Errors
///
/// Returns [`PipelineError`] if the Claude CLI is unavailable, times out, or
/// returns an unparseable response.
pub async fn quality_gate(
    claude: &ClaudeClient,
    title: &str,
    content: &str,
    summary: &str,
) -> Result<QualityResult, PipelineError> {
    let prompt = build_quality_prompt(title, content, summary);
    let raw = claude.prompt(&prompt).await?;

    // Claude occasionally wraps JSON in a markdown code fence — strip it.
    let json_str = strip_code_fence(&raw);

    let parsed: ClaudeQualityResponse = serde_json::from_str(json_str).map_err(|e| {
        PipelineError::ClaudeOutputParse(format!(
            "quality gate response parse failure: {e} — raw: {json_str}"
        ))
    })?;

    Ok(QualityResult {
        accepted: parsed.accepted,
        rationale: parsed.rationale,
        scores: QualityScores {
            actionability: parsed.scores.actionability.clamp(0.0, 1.0),
            specificity: parsed.scores.specificity.clamp(0.0, 1.0),
            signal_to_noise: parsed.scores.signal_to_noise.clamp(0.0, 1.0),
        },
    })
}

// ---------------------------------------------------------------------------
// Prompt construction (public for testing)
// ---------------------------------------------------------------------------

/// Build the quality-gate prompt sent to Claude.
///
/// Exposed publicly so tests can assert on its structure without needing a
/// live Claude process.
#[must_use]
pub fn build_quality_prompt(title: &str, content: &str, summary: &str) -> String {
    format!(
        r#"You are a strict quality gate for a developer knowledge base called FixOnce.

Evaluate the memory below and decide if it is worth storing permanently.

## Memory
Title: {title}
Summary: {summary}
Content:
{content}

## Scoring criteria

Rate each axis from 0.0 (worst) to 1.0 (best):

- **actionability**: Can a developer act on this information right now, without
  needing to look anything else up?  Generic advice scores low; specific,
  reproducible steps score high.

- **specificity**: Is this specific enough to be unambiguously useful?  Vague
  observations score low; concrete details (versions, error messages, exact
  commands) score high.

- **signal_to_noise**: Does real, useful information outweigh padding,
  boilerplate, or repetition?  Pure noise scores 0; dense, essential facts
  score 1.

## Decision rule

Accept the memory (`"accepted": true`) if the **average** of the three scores
is ≥ 0.5 **and** no individual score is below 0.3.  Reject otherwise.

## Output format

Reply with **only** valid JSON, no prose:

```json
{{
  "accepted": <boolean>,
  "rationale": "<one sentence explanation>",
  "scores": {{
    "actionability": <0.0–1.0>,
    "specificity": <0.0–1.0>,
    "signal_to_noise": <0.0–1.0>
  }}
}}
```"#
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Remove an optional ```json … ``` or ``` … ``` fence from Claude's output.
fn strip_code_fence(s: &str) -> &str {
    let trimmed = s.trim();
    let inner = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(str::trim_start)
        .and_then(|s| s.strip_suffix("```"))
        .map(str::trim);
    inner.unwrap_or(trimmed)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Prompt construction tests ---

    #[test]
    fn prompt_contains_title() {
        let p = build_quality_prompt("My Title", "body text", "short summary");
        assert!(p.contains("My Title"), "prompt must embed the title");
    }

    #[test]
    fn prompt_contains_content() {
        let p = build_quality_prompt("t", "unique content body goes here", "s");
        assert!(
            p.contains("unique content body goes here"),
            "prompt must embed the content"
        );
    }

    #[test]
    fn prompt_contains_summary() {
        let p = build_quality_prompt("t", "c", "the summary text");
        assert!(
            p.contains("the summary text"),
            "prompt must embed the summary"
        );
    }

    #[test]
    fn prompt_instructs_json_output() {
        let p = build_quality_prompt("t", "c", "s");
        assert!(
            p.contains("valid JSON"),
            "prompt must ask for valid JSON output"
        );
        assert!(
            p.contains("actionability"),
            "prompt must mention actionability"
        );
        assert!(p.contains("specificity"), "prompt must mention specificity");
        assert!(
            p.contains("signal_to_noise"),
            "prompt must mention signal_to_noise"
        );
    }

    // --- Response parsing tests (no Claude needed) ---

    #[test]
    fn quality_result_parses_accepted_response() {
        let raw = r#"{"accepted":true,"rationale":"Great memory.","scores":{"actionability":0.9,"specificity":0.8,"signal_to_noise":0.7}}"#;
        let parsed: ClaudeQualityResponse = serde_json::from_str(raw).expect("must parse");
        assert!(parsed.accepted);
        assert_eq!(parsed.rationale, "Great memory.");
        assert!((parsed.scores.actionability - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn quality_result_parses_rejected_response() {
        let raw = r#"{"accepted":false,"rationale":"Too vague.","scores":{"actionability":0.2,"specificity":0.1,"signal_to_noise":0.3}}"#;
        let parsed: ClaudeQualityResponse = serde_json::from_str(raw).expect("must parse");
        assert!(!parsed.accepted);
    }

    #[test]
    fn scores_are_clamped() {
        // Even if Claude returns out-of-range values we clamp them.
        let scores = QualityScores {
            actionability: 1.5_f64.clamp(0.0, 1.0),
            specificity: (-0.1_f64).clamp(0.0, 1.0),
            signal_to_noise: 0.5_f64.clamp(0.0, 1.0),
        };
        assert!((scores.actionability - 1.0).abs() < f64::EPSILON);
        assert!((scores.specificity - 0.0).abs() < f64::EPSILON);
    }

    // --- Code fence stripping ---

    #[test]
    fn strips_json_code_fence() {
        let fenced = "```json\n{\"accepted\":true}\n```";
        assert_eq!(strip_code_fence(fenced), "{\"accepted\":true}");
    }

    #[test]
    fn strips_plain_code_fence() {
        let fenced = "```\n{\"accepted\":false}\n```";
        assert_eq!(strip_code_fence(fenced), "{\"accepted\":false}");
    }

    #[test]
    fn leaves_plain_json_untouched() {
        let json = r#"{"accepted":true,"rationale":"ok","scores":{"actionability":1.0,"specificity":1.0,"signal_to_noise":1.0}}"#;
        assert_eq!(strip_code_fence(json), json);
    }
}
