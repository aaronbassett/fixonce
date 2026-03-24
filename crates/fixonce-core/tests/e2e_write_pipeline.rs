//! T303 — Write pipeline end-to-end integration tests.
//!
//! Tests the complete write pipeline logic: credential check → quality gate
//! prompt building → dedup prompt building → enrichment flow.
//!
//! No live API calls or Claude CLI invocations are made — only the pure
//! functions and data-structure logic are exercised.

use fixonce_core::{
    memory::types::{CreateMemoryRequest, MemoryType, SourceType},
    pipeline::write::{
        credential_check::check_for_credentials, dedup::build_dedup_prompt,
        enrichment::enrich_metadata, quality_gate::build_quality_prompt,
    },
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_request(title: &str, content: &str, language: Option<&str>) -> CreateMemoryRequest {
    CreateMemoryRequest {
        title: title.to_owned(),
        content: content.to_owned(),
        summary: "Test summary.".to_owned(),
        memory_type: MemoryType::Discovery,
        source_type: SourceType::Manual,
        language: language.map(str::to_owned),
        embedding: None,
        compact_pragma: None,
        compact_compiler: None,
        midnight_js: None,
        indexer_version: None,
        node_version: None,
        source_url: None,
        repo_url: None,
        task_summary: None,
        session_id: None,
    }
}

// ---------------------------------------------------------------------------
// T303-a: Credential detection — verify credentials are caught
// ---------------------------------------------------------------------------

#[test]
fn openai_api_key_in_content_is_detected() {
    let content = "Set OPENAI_API_KEY=sk-abcdefghijklmnopqrstuvwxyz1234 in your environment.";
    let matches = check_for_credentials(content);
    assert!(
        matches
            .iter()
            .any(|m| m.credential_type == "OpenAI API key"),
        "OpenAI key must be detected, got: {matches:?}"
    );
}

#[test]
fn anthropic_api_key_detected_before_generic_openai() {
    // Anthropic keys start with sk-ant- and should win over the generic sk- pattern
    let content = "API_KEY=sk-ant-api03-abcdefghijklmnopqrstuvwxyz12345678";
    let matches = check_for_credentials(content);
    assert!(
        matches
            .iter()
            .any(|m| m.credential_type == "Anthropic API key"),
        "Anthropic key must be detected, got: {matches:?}"
    );
    // Generic OpenAI pattern should NOT fire for an Anthropic key
    let has_openai = matches
        .iter()
        .any(|m| m.credential_type == "OpenAI API key");
    assert!(
        !has_openai,
        "Anthropic key must not also match OpenAI pattern"
    );
}

#[test]
fn pem_private_key_is_detected() {
    let content =
        "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
    let matches = check_for_credentials(content);
    assert!(
        matches
            .iter()
            .any(|m| m.credential_type == "PEM private key"),
        "PEM private key must be detected"
    );
}

#[test]
fn github_pat_is_detected() {
    let content = "GITHUB_TOKEN=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij";
    let matches = check_for_credentials(content);
    assert!(
        matches
            .iter()
            .any(|m| m.credential_type == "GitHub personal access token"),
        "GitHub PAT must be detected"
    );
}

#[test]
fn aws_access_key_is_detected() {
    let content = "export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
    let matches = check_for_credentials(content);
    assert!(
        matches
            .iter()
            .any(|m| m.credential_type == "AWS access key ID"),
        "AWS access key ID must be detected"
    );
}

#[test]
fn stripe_live_key_is_detected() {
    // Build test key at runtime to avoid triggering GitHub push protection
    let content = format!("STRIPE_SECRET=sk_live_{}", "TESTKEY000000000000000000");
    let matches = check_for_credentials(&content);
    assert!(
        matches
            .iter()
            .any(|m| m.credential_type == "Stripe live secret key"),
        "Stripe live secret key must be detected"
    );
}

#[test]
fn clean_content_returns_no_matches() {
    let content = "Always use const instead of let when the value never changes. This is a best practice for Rust.";
    let matches = check_for_credentials(content);
    assert!(
        matches.is_empty(),
        "clean content must not produce any credential matches, got: {matches:?}"
    );
}

#[test]
fn credential_match_contains_correct_line_number() {
    let content = "line one\nline two with no secrets\nsk-verylongopenaiapikey12345678\nline four";
    let matches = check_for_credentials(content);
    let key = matches
        .iter()
        .find(|m| m.credential_type == "OpenAI API key");
    assert!(key.is_some(), "OpenAI key must be detected");
    assert_eq!(key.unwrap().line, 3, "credential must be found on line 3");
}

#[test]
fn matched_pattern_is_truncated_for_safety() {
    let content = "SECRET=sk-abcdefghijklmnopqrstuvwxyz0123456789";
    let matches = check_for_credentials(content);
    if let Some(m) = matches
        .iter()
        .find(|m| m.credential_type == "OpenAI API key")
    {
        assert!(
            m.pattern.chars().count() <= 13,
            "pattern must be truncated, got: {}",
            m.pattern
        );
    }
}

#[test]
fn password_assignment_is_detected_in_config_snippet() {
    let content = r#"[database]
host = "localhost"
DB_PASSWORD="supersecretpassword123"
port = 5432"#;
    let matches = check_for_credentials(content);
    assert!(
        matches
            .iter()
            .any(|m| m.credential_type == "password assignment"),
        "password assignment must be detected in config snippet"
    );
}

// ---------------------------------------------------------------------------
// T303-b: Quality gate prompt building
// ---------------------------------------------------------------------------

#[test]
fn quality_gate_prompt_embeds_all_three_fields() {
    let title = "Use parameterized queries";
    let content = "Always use parameterized queries to prevent SQL injection.";
    let summary = "SQL injection prevention via parameterized queries.";

    let prompt = build_quality_prompt(title, content, summary);

    assert!(prompt.contains(title), "prompt must contain title");
    assert!(prompt.contains(content), "prompt must contain content");
    assert!(prompt.contains(summary), "prompt must contain summary");
}

#[test]
fn quality_gate_prompt_specifies_json_output_format() {
    let prompt = build_quality_prompt("t", "c", "s");

    assert!(
        prompt.contains("valid JSON"),
        "prompt must require JSON output"
    );
    assert!(
        prompt.contains("actionability"),
        "prompt must mention actionability"
    );
    assert!(
        prompt.contains("specificity"),
        "prompt must mention specificity"
    );
    assert!(
        prompt.contains("signal_to_noise"),
        "prompt must mention signal_to_noise"
    );
}

#[test]
fn quality_gate_prompt_includes_decision_rule() {
    let prompt = build_quality_prompt("t", "c", "s");
    assert!(
        prompt.contains("0.5"),
        "prompt must mention the 0.5 acceptance threshold"
    );
    assert!(
        prompt.contains("0.3"),
        "prompt must mention the 0.3 minimum score"
    );
}

// ---------------------------------------------------------------------------
// T303-c: Dedup prompt building
// ---------------------------------------------------------------------------

#[test]
fn dedup_prompt_with_no_candidates_still_lists_all_outcomes() {
    let req = make_request(
        "Avoid using unwrap()",
        "Don't call unwrap() in library code.",
        None,
    );
    let prompt = build_dedup_prompt(&req, &[]);

    for outcome in &["new", "discard", "replace", "update", "merge"] {
        assert!(
            prompt.contains(outcome),
            "dedup prompt must list outcome '{outcome}'"
        );
    }
}

#[test]
fn dedup_prompt_includes_incoming_title_and_content() {
    let req = make_request(
        "Unique Integration Test Title XYZ",
        "Unique content body for T303 dedup prompt test.",
        None,
    );
    let prompt = build_dedup_prompt(&req, &[]);

    assert!(
        prompt.contains("Unique Integration Test Title XYZ"),
        "prompt must embed the incoming memory title"
    );
    assert!(
        prompt.contains("Unique content body for T303 dedup prompt test."),
        "prompt must embed the incoming memory content"
    );
}

#[test]
fn dedup_prompt_requires_json_output() {
    let req = make_request("t", "c", None);
    let prompt = build_dedup_prompt(&req, &[]);
    assert!(
        prompt.contains("valid JSON"),
        "dedup prompt must require JSON output"
    );
}

// ---------------------------------------------------------------------------
// T303-d: Enrichment — verify correct language/type suggestions
// ---------------------------------------------------------------------------

#[test]
fn enrichment_detects_rust_from_fn_main() {
    let req = make_request("Memory title", "fn main() { println!(\"hello\"); }", None);
    let result = enrich_metadata(&req.content.clone(), &req);
    assert_eq!(
        result.suggested_language.as_deref(),
        Some("rust"),
        "must detect Rust from fn main()"
    );
}

#[test]
fn enrichment_detects_python_from_def_keyword() {
    let req = make_request(
        "Memory title",
        "def calculate(x, y):\n    return x + y",
        None,
    );
    let result = enrich_metadata(&req.content.clone(), &req);
    assert_eq!(
        result.suggested_language.as_deref(),
        Some("python"),
        "must detect Python from def keyword"
    );
}

#[test]
fn enrichment_detects_typescript_from_import_react() {
    let req = make_request("Memory title", "import React from 'react';", None);
    let result = enrich_metadata(&req.content.clone(), &req);
    assert_eq!(
        result.suggested_language.as_deref(),
        Some("typescript"),
        "must detect TypeScript from import React"
    );
}

#[test]
fn enrichment_detects_go_from_package_main() {
    let req = make_request("Memory title", "package main\n\nfunc main() {}", None);
    let result = enrich_metadata(&req.content.clone(), &req);
    assert_eq!(
        result.suggested_language.as_deref(),
        Some("go"),
        "must detect Go from package main"
    );
}

#[test]
fn enrichment_does_not_suggest_language_when_already_set() {
    let req = make_request("Memory title", "fn main() {}", Some("rust"));
    let result = enrich_metadata(&req.content.clone(), &req);
    assert!(
        result.suggested_language.is_none(),
        "must not suggest language when already set on request"
    );
}

#[test]
fn enrichment_suggests_anti_pattern_for_avoid_keyword() {
    let req = make_request(
        "Bad practice",
        "You should avoid calling unwrap() in library code.",
        None,
    );
    let result = enrich_metadata(&req.content.clone(), &req);
    assert_eq!(
        result.suggested_memory_type,
        Some(MemoryType::AntiPattern),
        "must suggest AntiPattern for 'avoid' keyword"
    );
}

#[test]
fn enrichment_suggests_gotcha_for_gotcha_keyword() {
    let req = make_request(
        "React gotcha",
        "Gotcha: useEffect cleanup runs before the next effect.",
        None,
    );
    let result = enrich_metadata(&req.content.clone(), &req);
    assert_eq!(
        result.suggested_memory_type,
        Some(MemoryType::Gotcha),
        "must suggest Gotcha for 'gotcha' keyword"
    );
}

#[test]
fn enrichment_suggests_best_practice_for_always_keyword() {
    let req = make_request(
        "SQL safety",
        "Always use parameterized queries to prevent SQL injection.",
        None,
    );
    let result = enrich_metadata(&req.content.clone(), &req);
    assert_eq!(
        result.suggested_memory_type,
        Some(MemoryType::BestPractice),
        "must suggest BestPractice for 'always' keyword"
    );
}

#[test]
fn enrichment_warns_about_missing_language() {
    let req = make_request("Memory", "some generic content without code", None);
    let result = enrich_metadata(&req.content.clone(), &req);
    assert!(
        result
            .missing_metadata_warnings
            .iter()
            .any(|w| w.contains("language")),
        "must warn about missing language"
    );
}

#[test]
fn enrichment_warns_about_missing_source_url() {
    let req = make_request("Memory", "some content", None);
    let result = enrich_metadata(&req.content.clone(), &req);
    assert!(
        result
            .missing_metadata_warnings
            .iter()
            .any(|w| w.contains("source_url") || w.contains("repo_url")),
        "must warn about missing source/repo URL"
    );
}

#[test]
fn enrichment_warns_about_solidity_missing_pragma_metadata() {
    let req = make_request(
        "Solidity contract",
        "pragma solidity ^0.8.0;\ncontract MyToken { }",
        None,
    );
    let result = enrich_metadata(&req.content.clone(), &req);
    assert!(
        result
            .missing_metadata_warnings
            .iter()
            .any(|w| w.contains("compact_pragma")),
        "must warn about missing compact_pragma for Solidity content"
    );
}

#[test]
fn full_write_pipeline_integration_credential_then_enrich() {
    // Simulate the write pipeline: first check for credentials, then enrich.
    let content = r#"fn main() {
    // Always use environment variables for secrets, never hardcode them.
    let db_host = std::env::var("DB_HOST").expect("DB_HOST must be set");
    println!("Connecting to {}", db_host);
}"#;
    let req = make_request("Rust env var best practice", content, None);

    // Step 1: credential check
    let creds = check_for_credentials(content);
    assert!(
        creds.is_empty(),
        "no credentials should be detected in clean Rust code, got: {creds:?}"
    );

    // Step 2: enrich
    let enrichment = enrich_metadata(content, &req);
    assert_eq!(
        enrichment.suggested_language.as_deref(),
        Some("rust"),
        "must detect Rust"
    );
    assert_eq!(
        enrichment.suggested_memory_type,
        Some(MemoryType::BestPractice),
        "must suggest BestPractice due to 'always' keyword"
    );
}
