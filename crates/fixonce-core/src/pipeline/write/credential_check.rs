//! Regex-based credential and PII detection.
//!
//! No LLM is required — this stage runs purely on pattern matching so it is
//! always fast and never incurs an external API call.

use std::sync::LazyLock;

use regex::Regex;

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// A single credential pattern match found inside memory content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialMatch {
    /// Human-readable category, e.g. `"OpenAI API key"`.
    pub credential_type: String,
    /// The matched text (possibly truncated for safety).
    pub pattern: String,
    /// 1-based line number of the match.
    pub line: usize,
}

// ---------------------------------------------------------------------------
// Pattern registry — one static LazyLock<Regex> per pattern
// ---------------------------------------------------------------------------
//
// Patterns MUST be ordered most-specific → least-specific within any group
// that shares a textual prefix, so the first match per line wins the right
// label.

// PEM / private key blocks
static RE_PEM_PRIVATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-----BEGIN[A-Z ]+PRIVATE KEY-----").unwrap());
static RE_PEM_CERT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-----BEGIN CERTIFICATE-----").unwrap());

// GitHub / GitLab (most specific first)
static RE_GH_APP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"github_pat_[A-Za-z0-9_]{82}").unwrap());
static RE_GH_PAT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"ghp_[A-Za-z0-9]{36}").unwrap());
static RE_GH_OAUTH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"gho_[A-Za-z0-9]{36}").unwrap());
static RE_GL_PAT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"glpat-[A-Za-z0-9\-_]{20}").unwrap());

// Stripe
static RE_STRIPE_LIVE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"sk_live_[A-Za-z0-9]{24,}").unwrap());
static RE_STRIPE_TEST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"sk_test_[A-Za-z0-9]{24,}").unwrap());

// Slack
static RE_SLACK_BOT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"xoxb-[0-9A-Za-z\-]{24,}").unwrap());
static RE_SLACK_USER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"xoxp-[0-9A-Za-z\-]{24,}").unwrap());

// Anthropic (more specific than OpenAI — must come first)
static RE_ANTHROPIC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"sk-ant-[A-Za-z0-9\-_]{20,}").unwrap());
// OpenAI (generic sk- prefix)
static RE_OPENAI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"sk-[A-Za-z0-9]{20,}").unwrap());

// AWS
static RE_AWS_KEY_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|[^A-Z0-9])AKIA[0-9A-Z]{16}(?:[^A-Z0-9]|$)").unwrap());
static RE_AWS_SECRET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)aws_secret_access_key\s*[=:]\s*[A-Za-z0-9/+]{40}").unwrap());

// GCP
static RE_GCP_SA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""type"\s*:\s*"service_account""#).unwrap());

// Azure
static RE_AZURE_SAS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)sig=[A-Za-z0-9%/+]{20,}").unwrap());
static RE_AZURE_CONN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)DefaultEndpointsProtocol=https;AccountName=").unwrap());

// Generic password / secret / token assignments (after all specific patterns)
static RE_PASSWORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(?:password|passwd|pwd)\s*[=:]\s*['"]?[^\s'"]{8,}['"]?"#).unwrap()
});
static RE_SECRET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(?:secret|api_secret|client_secret)\s*[=:]\s*['"]?[^\s'"]{8,}['"]?"#).unwrap()
});
static RE_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(?:token|access_token|auth_token)\s*[=:]\s*['"]?[A-Za-z0-9\-_\.]{20,}['"]?"#)
        .unwrap()
});

// Email (PII)
static RE_EMAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}").unwrap());

// Long hex token (≥ 32 hex chars = 128-bit minimum)
static RE_HEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[^0-9A-Fa-f])([0-9A-Fa-f]{32,})(?:[^0-9A-Fa-f]|$)").unwrap()
});

// Long base64 token (≥ 40 chars)
static RE_B64: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|[\s=:])([A-Za-z0-9+/]{40,}={0,2})(?:\s|$)").unwrap());

// ---------------------------------------------------------------------------
// Ordered pattern table
// ---------------------------------------------------------------------------

/// Returns the ordered list of `(label, regex)` pairs.
///
/// Order matters: the first pattern to match a line wins.
fn patterns() -> &'static [(&'static str, &'static LazyLock<Regex>)] {
    // Highly specific patterns are listed first so they win before any
    // broad generic patterns that share textual prefixes.
    static PAIRS: &[(&str, &LazyLock<Regex>)] = &[
        ("PEM private key", &RE_PEM_PRIVATE),
        ("PEM certificate", &RE_PEM_CERT),
        ("GitHub app token", &RE_GH_APP),
        ("GitHub personal access token", &RE_GH_PAT),
        ("GitHub OAuth token", &RE_GH_OAUTH),
        ("GitLab personal access token", &RE_GL_PAT),
        ("Stripe live secret key", &RE_STRIPE_LIVE),
        ("Stripe test secret key", &RE_STRIPE_TEST),
        ("Slack bot token", &RE_SLACK_BOT),
        ("Slack user token", &RE_SLACK_USER),
        ("Anthropic API key", &RE_ANTHROPIC),
        ("OpenAI API key", &RE_OPENAI),
        ("AWS access key ID", &RE_AWS_KEY_ID),
        ("AWS secret access key", &RE_AWS_SECRET),
        ("GCP service account key", &RE_GCP_SA),
        ("Azure SAS token", &RE_AZURE_SAS),
        ("Azure connection string", &RE_AZURE_CONN),
        // Generic — must come after all specific patterns
        ("password assignment", &RE_PASSWORD),
        ("secret assignment", &RE_SECRET),
        ("token assignment", &RE_TOKEN),
        ("email address", &RE_EMAIL),
        ("long hex token", &RE_HEX),
        ("long base64 token", &RE_B64),
    ];
    PAIRS
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Scan `content` for credential and PII patterns.
///
/// Returns every match found, with 1-based line numbers and a safely truncated
/// copy of the matched text.  The returned list may be empty, meaning no known
/// credentials were detected.
///
/// This function is pure and has no side-effects.
#[must_use]
pub fn check_for_credentials(content: &str) -> Vec<CredentialMatch> {
    let mut matches = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_number = line_idx + 1; // convert to 1-based

        for (label, re) in patterns() {
            if let Some(m) = re.find(line) {
                let raw = m.as_str();
                // Truncate the pattern to avoid storing live credentials in
                // the match list (keep only the first 12 chars).
                let pattern = if raw.len() > 12 {
                    format!("{}…", &raw[..12])
                } else {
                    raw.to_owned()
                };

                matches.push(CredentialMatch {
                    credential_type: (*label).to_owned(),
                    pattern,
                    line: line_number,
                });
                // One match per pattern per line is enough.
                break;
            }
        }
    }

    matches
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn match_types(content: &str) -> Vec<String> {
        check_for_credentials(content)
            .into_iter()
            .map(|m| m.credential_type)
            .collect()
    }

    #[test]
    fn detects_openai_key() {
        let content = "Set OPENAI_API_KEY=sk-abcdefghij1234567890XYZ in your shell";
        let types = match_types(content);
        assert!(
            types.iter().any(|t| t == "OpenAI API key"),
            "expected OpenAI API key, got: {types:?}"
        );
    }

    #[test]
    fn detects_pem_private_key() {
        let content =
            "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
        let types = match_types(content);
        assert!(
            types.iter().any(|t| t == "PEM private key"),
            "expected PEM private key, got: {types:?}"
        );
    }

    #[test]
    fn detects_aws_access_key() {
        let content = "export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        let types = match_types(content);
        assert!(
            types.iter().any(|t| t == "AWS access key ID"),
            "expected AWS access key ID, got: {types:?}"
        );
    }

    #[test]
    fn detects_email_address() {
        let content = "Contact us at support@example.com for help.";
        let types = match_types(content);
        assert!(
            types.iter().any(|t| t == "email address"),
            "expected email address, got: {types:?}"
        );
    }

    #[test]
    fn detects_password_assignment() {
        let content = r#"DB_PASSWORD="supersecret123""#;
        let types = match_types(content);
        assert!(
            types.iter().any(|t| t == "password assignment"),
            "expected password assignment, got: {types:?}"
        );
    }

    #[test]
    fn detects_github_pat() {
        // exactly 36 alphanumeric chars after ghp_ (26 uppercase + 10 lowercase)
        let content = "token = ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij";
        let types = match_types(content);
        assert!(
            types.iter().any(|t| t == "GitHub personal access token"),
            "expected GitHub personal access token, got: {types:?}"
        );
    }

    #[test]
    fn detects_stripe_live_key() {
        // Build test key at runtime to avoid triggering GitHub push protection
        let content = format!("STRIPE_SECRET=sk_live_{}", "TESTKEY000000000000000000");
        let types = match_types(&content);
        assert!(
            types.iter().any(|t| t == "Stripe live secret key"),
            "expected Stripe live secret key, got: {types:?}"
        );
    }

    #[test]
    fn clean_content_returns_empty() {
        let content = "Always use const instead of let when the value never changes.";
        let result = check_for_credentials(content);
        assert!(result.is_empty(), "expected no matches, got: {result:?}");
    }

    #[test]
    fn match_carries_correct_line_number() {
        let content = "line one\nline two\nsk-verylongopenaiapikey12345\nline four";
        let matches = check_for_credentials(content);
        let key_match = matches
            .iter()
            .find(|m| m.credential_type == "OpenAI API key");
        assert!(key_match.is_some(), "expected OpenAI API key match");
        assert_eq!(key_match.unwrap().line, 3);
    }

    #[test]
    fn pattern_is_truncated_at_12_chars() {
        // Use a long key pattern to confirm truncation
        let content = "export KEY=sk-abcdefghijklmnopqrstuvwxyz";
        let matches = check_for_credentials(content);
        let openai = matches
            .iter()
            .find(|m| m.credential_type == "OpenAI API key");
        if let Some(m) = openai {
            // Pattern must not exceed 13 visible chars (12 + ellipsis)
            assert!(
                m.pattern.chars().count() <= 13,
                "pattern should be truncated: {}",
                m.pattern
            );
        }
    }
}
