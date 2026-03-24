//! Thin wrapper around the `claude` CLI.
//!
//! Shells out to `claude -p --output-format json "<prompt>"` and parses the
//! JSON response into a plain text string.  A 30-second timeout is enforced
//! by default (EC-26).  Missing CLI is surfaced as [`PipelineError::ClaudeNotFound`]
//! (EC-37).

use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

use super::PipelineError;

/// Timeout for a single Claude CLI call.
const CLAUDE_TIMEOUT_SECS: u64 = 30;

// ---------------------------------------------------------------------------
// Response shape returned by `claude --output-format json`
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct ClaudeJsonOutput {
    /// The model's text response.
    result: String,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Stateless wrapper around the system `claude` CLI executable.
///
/// Construct once and share via reference; all state is held in the spawned
/// subprocess.
#[derive(Debug, Clone, Default)]
pub struct ClaudeClient;

impl ClaudeClient {
    /// Create a new client.  No I/O is performed until [`Self::prompt`] is
    /// called.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Send `prompt` to the Claude CLI and return the text response.
    ///
    /// # Errors
    ///
    /// | Condition | Error |
    /// |-----------|-------|
    /// | `claude` not on `PATH` | [`PipelineError::ClaudeNotFound`] |
    /// | No response within 30 s | [`PipelineError::ClaudeTimeout`] |
    /// | Non-zero exit code | [`PipelineError::ClaudeExitFailure`] |
    /// | Output is not valid JSON | [`PipelineError::ClaudeOutputParse`] |
    pub async fn prompt(&self, prompt: &str) -> Result<String, PipelineError> {
        let fut = async {
            let output = Command::new("claude")
                .args(["-p", "--output-format", "json", prompt])
                .output()
                .await;

            match output {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    Err(PipelineError::ClaudeNotFound)
                }
                Err(e) => Err(PipelineError::Io(e)),
                Ok(out) => {
                    if !out.status.success() {
                        let code = out.status.code().unwrap_or(-1);
                        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
                        return Err(PipelineError::ClaudeExitFailure { code, stderr });
                    }

                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let parsed: ClaudeJsonOutput = serde_json::from_str(&stdout)
                        .map_err(|e| PipelineError::ClaudeOutputParse(e.to_string()))?;

                    Ok(parsed.result)
                }
            }
        };

        match timeout(Duration::from_secs(CLAUDE_TIMEOUT_SECS), fut).await {
            Ok(result) => result,
            Err(_elapsed) => Err(PipelineError::ClaudeTimeout {
                seconds: CLAUDE_TIMEOUT_SECS,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// [`ClaudeClient::new()`] must not panic and must produce a usable value.
    #[test]
    fn client_constructs() {
        let _c = ClaudeClient::new();
        let _d = ClaudeClient::default();
    }

    /// JSON shape that the real CLI emits deserialises correctly.
    #[test]
    fn json_output_deserialises() {
        let raw = r#"{"result":"hello world","session_id":"abc"}"#;
        let parsed: ClaudeJsonOutput = serde_json::from_str(raw).expect("must parse");
        assert_eq!(parsed.result, "hello world");
    }

    /// Extraneous JSON fields do not cause a parse failure.
    #[test]
    fn json_output_ignores_extra_fields() {
        let raw = r#"{"result":"ok","extra_field":123,"nested":{"a":1}}"#;
        let parsed: ClaudeJsonOutput =
            serde_json::from_str(raw).expect("must parse with extra fields");
        assert_eq!(parsed.result, "ok");
    }
}
