//! Heuristic metadata enrichment.
//!
//! Pure, synchronous, no external calls — just pattern matching on the
//! memory content to suggest or validate metadata that the caller may not
//! have supplied.

use crate::memory::types::{CreateMemoryRequest, MemoryType};

// ---------------------------------------------------------------------------
// Result type
// ---------------------------------------------------------------------------

/// Suggestions and warnings produced by the enrichment stage.
#[derive(Debug, Clone, Default)]
pub struct EnrichmentResult {
    /// Detected programming language, if the heuristics are confident.
    pub suggested_language: Option<String>,
    /// Suggested [`MemoryType`] when the content clearly implies one.
    pub suggested_memory_type: Option<MemoryType>,
    /// Warnings about missing or suspicious metadata.
    pub missing_metadata_warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Language detection heuristics
// ---------------------------------------------------------------------------

/// (pattern substring, language tag)
static LANGUAGE_HINTS: &[(&str, &str)] = &[
    // Shebang lines
    ("#!/usr/bin/env python", "python"),
    ("#!/usr/bin/env node", "javascript"),
    ("#!/usr/bin/env ruby", "ruby"),
    ("#!/usr/bin/env bash", "bash"),
    ("#!/usr/bin/env sh", "bash"),
    // Rust
    ("fn main()", "rust"),
    ("use std::", "rust"),
    ("impl ", "rust"),
    ("pub struct ", "rust"),
    ("pub enum ", "rust"),
    ("let mut ", "rust"),
    ("cargo.toml", "rust"),
    // TypeScript / JavaScript
    ("import React", "typescript"),
    ("export default function", "typescript"),
    ("const { ", "javascript"),
    ("require('", "javascript"),
    ("module.exports", "javascript"),
    ("async function", "javascript"),
    (".tsx", "typescript"),
    (".ts", "typescript"),
    // Python
    ("def ", "python"),
    ("import numpy", "python"),
    ("import pandas", "python"),
    ("from django", "python"),
    ("from flask", "python"),
    ("if __name__ == '__main__'", "python"),
    // Go
    ("func main()", "go"),
    ("package main", "go"),
    ("import (\n", "go"),
    (":= ", "go"),
    // Java / Kotlin
    ("public class ", "java"),
    ("System.out.println", "java"),
    ("fun main(", "kotlin"),
    ("val ", "kotlin"),
    // Ruby
    ("def initialize", "ruby"),
    ("attr_accessor", "ruby"),
    ("require 'rails'", "ruby"),
    // Shell
    ("#!/bin/bash", "bash"),
    ("#!/bin/sh", "bash"),
    // SQL
    ("SELECT ", "sql"),
    ("CREATE TABLE", "sql"),
    ("INSERT INTO", "sql"),
    // Solidity / blockchain (domain-specific to FixOnce)
    ("pragma solidity", "solidity"),
    ("contract ", "solidity"),
    // CSS / SCSS
    (".css", "css"),
    (".scss", "scss"),
    // HTML
    ("<html", "html"),
    ("<!DOCTYPE html", "html"),
    // YAML / TOML / JSON (config)
    ("apiVersion:", "yaml"),
    ("kind: Deployment", "yaml"),
];

/// Detect the most likely programming language from `content`.
///
/// Returns the language tag for the first hint that matches (case-insensitive
/// for the lowercase hints, exact for `PascalCase` ones).  Returns `None` when
/// no hint fires.
#[must_use]
pub fn detect_language(content: &str) -> Option<String> {
    let lower = content.to_lowercase();
    for (hint, lang) in LANGUAGE_HINTS {
        // Most hints are lowercase; the to_lowercase lets us match case-
        // insensitively without re-writing all hints.
        if lower.contains(&hint.to_lowercase()) {
            return Some((*lang).to_owned());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Memory-type heuristics
// ---------------------------------------------------------------------------

/// Suggest a [`MemoryType`] from obvious keywords in the title or content.
///
/// Returns `None` when the heuristics are not confident enough to make a
/// suggestion.
#[must_use]
fn suggest_memory_type(title: &str, content: &str) -> Option<MemoryType> {
    let haystack = format!("{title} {content}").to_lowercase();

    // Anti-patterns are typically phrased as "don't", "avoid", "never".
    if haystack.contains("don't ")
        || haystack.contains("dont ")
        || haystack.contains("avoid ")
        || haystack.contains("anti-pattern")
        || haystack.contains("antipattern")
        || haystack.contains("never use")
        || haystack.contains("bad practice")
    {
        return Some(MemoryType::AntiPattern);
    }

    // Corrections tend to describe a fix or resolution.
    if haystack.contains("fix:")
        || haystack.contains("fixed by")
        || haystack.contains("the fix is")
        || haystack.contains("resolved by")
        || haystack.contains("corrected by")
        || haystack.contains("the bug was")
    {
        return Some(MemoryType::Correction);
    }

    // Gotchas are unexpected surprises.
    if haystack.contains("gotcha")
        || haystack.contains("watch out")
        || haystack.contains("beware")
        || haystack.contains("surprising")
        || haystack.contains("unexpected")
        || haystack.contains("pitfall")
        || haystack.contains("footgun")
    {
        return Some(MemoryType::Gotcha);
    }

    // Best practices are positive recommendations.
    if haystack.contains("always ")
        || haystack.contains("best practice")
        || haystack.contains("recommended")
        || haystack.contains("prefer ")
        || haystack.contains("use instead")
        || haystack.contains("the right way")
    {
        return Some(MemoryType::BestPractice);
    }

    None
}

// ---------------------------------------------------------------------------
// Metadata warnings
// ---------------------------------------------------------------------------

fn collect_warnings(req: &CreateMemoryRequest) -> Vec<String> {
    let mut warnings = Vec::new();

    if req.language.is_none() {
        warnings.push(
            "language is not set; consider tagging the memory with a language for better retrieval"
                .to_owned(),
        );
    }

    if req.source_url.is_none() && req.repo_url.is_none() {
        warnings.push(
            "no source_url or repo_url provided; linking to the original source aids verification"
                .to_owned(),
        );
    }

    if req.summary.len() > 2_000 {
        warnings.push(format!(
            "summary is {} characters — the recommended maximum is 2 000",
            req.summary.len()
        ));
    }

    if req.title.len() > 200 {
        warnings.push(format!(
            "title is {} characters — prefer a concise title under 200 characters",
            req.title.len()
        ));
    }

    // Blockchain-specific checks: if the content looks like Solidity, flag
    // missing compiler / pragma metadata.
    let lower = req.content.to_lowercase();
    if lower.contains("pragma solidity") || lower.contains("contract ") {
        if req.compact_pragma.is_none() {
            warnings.push("Solidity content detected but compact_pragma is not set".to_owned());
        }
        if req.compact_compiler.is_none() {
            warnings.push("Solidity content detected but compact_compiler is not set".to_owned());
        }
    }

    warnings
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run heuristic metadata enrichment on `existing` and return suggestions.
///
/// This function never mutates `existing`; the caller decides whether to apply
/// the suggestions.
#[must_use]
pub fn enrich_metadata(content: &str, existing: &CreateMemoryRequest) -> EnrichmentResult {
    // Language: use existing if already set; otherwise run heuristics.
    let suggested_language = if existing.language.is_some() {
        None // already set — no suggestion needed
    } else {
        detect_language(content)
    };

    let suggested_memory_type = suggest_memory_type(&existing.title, content);
    let missing_metadata_warnings = collect_warnings(existing);

    EnrichmentResult {
        suggested_language,
        suggested_memory_type,
        missing_metadata_warnings,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::SourceType;

    fn base_req(content: &str) -> CreateMemoryRequest {
        CreateMemoryRequest {
            title: "Test memory".to_owned(),
            content: content.to_owned(),
            summary: "A short summary".to_owned(),
            memory_type: MemoryType::Discovery,
            source_type: SourceType::Manual,
            language: None,
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

    // --- Language detection ---

    #[test]
    fn detects_rust_from_fn_main() {
        assert_eq!(
            detect_language("fn main() { println!(\"hi\"); }"),
            Some("rust".to_owned())
        );
    }

    #[test]
    fn detects_python_from_def() {
        assert_eq!(
            detect_language("def my_func(x):\n    return x * 2"),
            Some("python".to_owned())
        );
    }

    #[test]
    fn detects_typescript_from_import_react() {
        assert_eq!(
            detect_language("import React from 'react';"),
            Some("typescript".to_owned())
        );
    }

    #[test]
    fn detects_go_from_package_main() {
        assert_eq!(
            detect_language("package main\n\nfunc main() {}"),
            Some("go".to_owned())
        );
    }

    #[test]
    fn detects_solidity_pragma() {
        assert_eq!(
            detect_language("pragma solidity ^0.8.0; contract Foo {}"),
            Some("solidity".to_owned())
        );
    }

    #[test]
    fn returns_none_for_plain_english() {
        assert_eq!(
            detect_language("Always prefer immutable data structures for shared state."),
            None
        );
    }

    #[test]
    fn no_suggestion_when_language_already_set() {
        let mut req = base_req("fn main() {}");
        req.language = Some("rust".to_owned());
        let result = enrich_metadata("fn main() {}", &req);
        assert!(
            result.suggested_language.is_none(),
            "should not suggest language when already set"
        );
    }

    // --- Memory type suggestions ---

    #[test]
    fn suggests_anti_pattern_for_avoid_keyword() {
        let req = base_req("You should avoid using unwrap() in library code.");
        let result = enrich_metadata(&req.content.clone(), &req);
        assert_eq!(result.suggested_memory_type, Some(MemoryType::AntiPattern));
    }

    #[test]
    fn suggests_gotcha_for_gotcha_keyword() {
        let req = base_req("Gotcha: tokio::spawn captures must be 'static.");
        let result = enrich_metadata(&req.content.clone(), &req);
        assert_eq!(result.suggested_memory_type, Some(MemoryType::Gotcha));
    }

    #[test]
    fn suggests_correction_for_fix_keyword() {
        let req = base_req("The fix is to add a semicolon after the closing brace.");
        let result = enrich_metadata(&req.content.clone(), &req);
        assert_eq!(result.suggested_memory_type, Some(MemoryType::Correction));
    }

    #[test]
    fn suggests_best_practice_for_always_keyword() {
        let req = base_req("Always use parameterised queries to prevent SQL injection.");
        let result = enrich_metadata(&req.content.clone(), &req);
        assert_eq!(result.suggested_memory_type, Some(MemoryType::BestPractice));
    }

    // --- Metadata warnings ---

    #[test]
    fn warns_about_missing_language() {
        let req = base_req("some content");
        let result = enrich_metadata(&req.content.clone(), &req);
        assert!(
            result
                .missing_metadata_warnings
                .iter()
                .any(|w| w.contains("language")),
            "expected language warning"
        );
    }

    #[test]
    fn no_language_warning_when_set() {
        let mut req = base_req("some content");
        req.language = Some("rust".to_owned());
        let result = enrich_metadata(&req.content.clone(), &req);
        assert!(
            !result
                .missing_metadata_warnings
                .iter()
                .any(|w| w.contains("language")),
            "should not warn about language when set"
        );
    }

    #[test]
    fn warns_about_solidity_missing_pragma() {
        let req = base_req("pragma solidity ^0.8.0; contract Foo { }");
        let result = enrich_metadata(&req.content.clone(), &req);
        assert!(
            result
                .missing_metadata_warnings
                .iter()
                .any(|w| w.contains("compact_pragma")),
            "expected solidity pragma warning"
        );
    }

    #[test]
    fn warns_about_overlong_summary() {
        let mut req = base_req("content");
        req.summary = "x".repeat(2_001);
        let result = enrich_metadata(&req.content.clone(), &req);
        assert!(
            result
                .missing_metadata_warnings
                .iter()
                .any(|w| w.contains("summary")),
            "expected overlong summary warning"
        );
    }
}
