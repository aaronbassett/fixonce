//! Query technique stages for the read pipeline.
//!
//! Each technique is a zero-sized struct that implements
//! [`PipelineStage`][super::pipeline_runner::PipelineStage].  They can be
//! composed freely in a [`PipelineRunner`][super::pipeline_runner::PipelineRunner].
//!
//! | Technique | What it does |
//! |-----------|-------------|
//! | [`QueryRewriting`] | Claude rewrites the query for better retrieval |
//! | [`MultiQuery`] | Claude generates 3-5 variant queries |
//! | [`StepBack`] | Claude abstracts the query to a higher-level concept |
//! | [`HyDE`] | Hypothetical Document Embedding — Claude drafts an ideal answer |
//! | [`Decomposition`] | Break complex queries into sub-queries |
//! | [`RetrieveReadRetrieve`] | First search, read results, then refine and re-search |
//! | [`QueryRefinement`] | Use initial search results to refine the query |
//! | [`ContradictionDetection`] | Identify conflicting memories in current results |

use crate::pipeline::{claude::ClaudeClient, PipelineError};

use super::pipeline_runner::{PipelineContext, PipelineStage};

// ---------------------------------------------------------------------------
// Helpers shared across techniques
// ---------------------------------------------------------------------------

/// Remove an optional ```json … ``` or ``` … ``` fence.
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
// 1. QueryRewriting
// ---------------------------------------------------------------------------

/// Rewrites the user query to make it more amenable to vector search.
///
/// Prompt asks Claude to emit a single improved query string.
pub struct QueryRewriting;

/// Build the query rewriting prompt (exposed for testing).
#[must_use]
pub fn build_query_rewriting_prompt(query: &str) -> String {
    format!(
        r"You are a search query optimiser for a developer knowledge base called FixOnce.

Rewrite the query below so it is more specific and retrieval-friendly, without
changing its intent.  Use technical keywords where appropriate.

Query: {query}

Reply with **only** the rewritten query text — no JSON, no explanation."
    )
}

impl PipelineStage for QueryRewriting {
    fn name(&self) -> &'static str {
        "query_rewriting"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        let client = ClaudeClient::new();
        let prompt = build_query_rewriting_prompt(&ctx.original_query);
        let rewritten = client.prompt(&prompt).await?;
        let rewritten = rewritten.trim().to_owned();
        if !rewritten.is_empty() {
            ctx.rewritten_queries.push(rewritten);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 2. MultiQuery
// ---------------------------------------------------------------------------

/// Generates 3-5 variant phrasings of the query to improve recall.
///
/// Claude returns a JSON array of strings.
pub struct MultiQuery;

/// Build the multi-query prompt (exposed for testing).
#[must_use]
pub fn build_multi_query_prompt(query: &str) -> String {
    format!(
        r#"You are a search assistant for a developer knowledge base called FixOnce.

Generate 3 to 5 different phrasings of the query below that cover the same
intent but use different wording.  Variety helps retrieve more relevant results.

Query: {query}

Reply with **only** a JSON array of strings, e.g.:
["variant 1", "variant 2", "variant 3"]"#
    )
}

/// Parse a JSON array of strings from Claude's multi-query response.
///
/// Returns an empty vec if parsing fails, so the pipeline can degrade
/// gracefully.
#[must_use]
pub fn parse_multi_query_response(raw: &str) -> Vec<String> {
    let json = strip_code_fence(raw);
    serde_json::from_str::<Vec<String>>(json).unwrap_or_default()
}

impl PipelineStage for MultiQuery {
    fn name(&self) -> &'static str {
        "multi_query"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        let client = ClaudeClient::new();
        let prompt = build_multi_query_prompt(&ctx.original_query);
        let raw = client.prompt(&prompt).await?;
        let variants = parse_multi_query_response(&raw);
        ctx.rewritten_queries.extend(variants);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 3. StepBack
// ---------------------------------------------------------------------------

/// Abstracts the query to a higher-level concept to improve coverage.
///
/// E.g. "how do I fix ENOMEM in rustls?" → "memory management errors in TLS".
pub struct StepBack;

/// Build the step-back prompt (exposed for testing).
#[must_use]
pub fn build_step_back_prompt(query: &str) -> String {
    format!(
        r"You are a search assistant for a developer knowledge base called FixOnce.

Given the specific query below, produce a higher-level, more abstract version
that captures the general topic.  This broader query can help surface related
knowledge.

Query: {query}

Reply with **only** the abstracted query text — no JSON, no explanation."
    )
}

impl PipelineStage for StepBack {
    fn name(&self) -> &'static str {
        "step_back"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        let client = ClaudeClient::new();
        let prompt = build_step_back_prompt(&ctx.original_query);
        let abstracted = client.prompt(&prompt).await?;
        let abstracted = abstracted.trim().to_owned();
        if !abstracted.is_empty() {
            ctx.rewritten_queries.push(abstracted);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 4. HyDE — Hypothetical Document Embedding
// ---------------------------------------------------------------------------

/// Generates a hypothetical ideal answer, which is then used as a search
/// query (its embedding tends to be closer to real answers than the question).
pub struct HyDE;

/// Build the `HyDE` prompt (exposed for testing).
#[must_use]
pub fn build_hyde_prompt(query: &str) -> String {
    format!(
        r"You are an expert developer knowledge assistant.

Write a concise, authoritative answer to the question below as if it appeared
in a developer knowledge base.  Include concrete details: version numbers,
error messages, exact commands, or code snippets where relevant.

Question: {query}

Reply with **only** the answer text — no preamble, no JSON."
    )
}

impl PipelineStage for HyDE {
    fn name(&self) -> &'static str {
        "hyde"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        let client = ClaudeClient::new();
        let prompt = build_hyde_prompt(&ctx.original_query);
        let hypothetical_doc = client.prompt(&prompt).await?;
        let hypothetical_doc = hypothetical_doc.trim().to_owned();
        if !hypothetical_doc.is_empty() {
            // Use the hypothetical document as an additional search query;
            // the search stage will embed each rewritten_query in turn.
            ctx.rewritten_queries.push(hypothetical_doc);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 5. Decomposition
// ---------------------------------------------------------------------------

/// Breaks a complex query into simpler sub-queries that can be searched
/// independently.
pub struct Decomposition;

/// Build the decomposition prompt (exposed for testing).
#[must_use]
pub fn build_decomposition_prompt(query: &str) -> String {
    format!(
        r#"You are a search assistant for a developer knowledge base called FixOnce.

The query below may be complex.  Break it down into 2-4 simpler sub-questions
that together cover its full meaning.

Query: {query}

Reply with **only** a JSON array of strings, e.g.:
["sub-question 1", "sub-question 2"]"#
    )
}

impl PipelineStage for Decomposition {
    fn name(&self) -> &'static str {
        "decomposition"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        let client = ClaudeClient::new();
        let prompt = build_decomposition_prompt(&ctx.original_query);
        let raw = client.prompt(&prompt).await?;
        // Reuse the same JSON-array parser as MultiQuery.
        let sub_queries = parse_multi_query_response(&raw);
        ctx.rewritten_queries.extend(sub_queries);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 6. RetrieveReadRetrieve
// ---------------------------------------------------------------------------

/// Two-pass retrieval: first search → read top results → refine query → search again.
///
/// The second search's results are appended to the existing result set.
/// Requires that the search stage has already populated `ctx.results`.
pub struct RetrieveReadRetrieve;

/// Build the retrieve-read-retrieve prompt (exposed for testing).
#[must_use]
pub fn build_rrr_prompt(query: &str, result_summaries: &str) -> String {
    format!(
        r"You are a search refinement assistant for a developer knowledge base called FixOnce.

The user asked: {query}

The initial search returned these summaries:
{result_summaries}

Based on what you have seen, produce a refined search query that is more
targeted and will surface additional relevant results the first search may have
missed.

Reply with **only** the refined query text — no JSON, no explanation."
    )
}

impl PipelineStage for RetrieveReadRetrieve {
    fn name(&self) -> &'static str {
        "retrieve_read_retrieve"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        if ctx.results.is_empty() {
            // Nothing to refine yet — skip.
            return Ok(());
        }

        let summaries: String = ctx
            .results
            .iter()
            .take(3)
            .enumerate()
            .map(|(i, hit)| {
                format!(
                    "{}. [{}] {}: {}",
                    i + 1,
                    hit.memory.memory_type,
                    hit.memory.title,
                    hit.memory.summary,
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let client = ClaudeClient::new();
        let prompt = build_rrr_prompt(&ctx.original_query, &summaries);
        let refined = client.prompt(&prompt).await?;
        let refined = refined.trim().to_owned();
        if !refined.is_empty() {
            ctx.rewritten_queries.push(refined);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 7. QueryRefinement
// ---------------------------------------------------------------------------

/// Uses the initial result set to produce a refined query (simpler than RRR —
/// one additional Claude call, no extra search pass).
pub struct QueryRefinement;

/// Build the query refinement prompt (exposed for testing).
#[must_use]
pub fn build_query_refinement_prompt(query: &str, result_titles: &str) -> String {
    format!(
        r"You are a search assistant for a developer knowledge base called FixOnce.

Original query: {query}

The search returned results with these titles:
{result_titles}

Refine the query to improve precision — remove any terms that led to
irrelevant results and add terms that would surface better matches.

Reply with **only** the refined query text — no JSON, no explanation."
    )
}

impl PipelineStage for QueryRefinement {
    fn name(&self) -> &'static str {
        "query_refinement"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        if ctx.results.is_empty() {
            return Ok(());
        }

        let titles: String = ctx
            .results
            .iter()
            .take(5)
            .enumerate()
            .map(|(i, hit)| format!("{}. {}", i + 1, hit.memory.title))
            .collect::<Vec<_>>()
            .join("\n");

        let client = ClaudeClient::new();
        let prompt = build_query_refinement_prompt(&ctx.original_query, &titles);
        let refined = client.prompt(&prompt).await?;
        let refined = refined.trim().to_owned();
        if !refined.is_empty() {
            ctx.rewritten_queries.push(refined);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 8. ContradictionDetection
// ---------------------------------------------------------------------------

/// Scans the current result set for conflicting memories and annotates
/// `ctx.metadata` with any contradictions found.
pub struct ContradictionDetection;

/// Response shape from Claude's contradiction check.
#[derive(Debug, serde::Deserialize)]
pub struct ContradictionResponse {
    /// Pairs of memory IDs that appear to contradict each other.
    pub contradictions: Vec<[String; 2]>,
    /// Human-readable explanation.
    pub summary: String,
}

/// Build the contradiction detection prompt (exposed for testing).
#[must_use]
pub fn build_contradiction_prompt(results_text: &str) -> String {
    format!(
        r#"You are a fact-checker for a developer knowledge base called FixOnce.

Review the memories below and identify any that give conflicting or contradictory
information on the same topic.

## Memories
{results_text}

Reply with **only** valid JSON:
```json
{{
  "contradictions": [["id-a", "id-b"], ...],
  "summary": "<one sentence>"
}}
```

If there are no contradictions reply with an empty array."#
    )
}

impl PipelineStage for ContradictionDetection {
    fn name(&self) -> &'static str {
        "contradiction_detection"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        if ctx.results.len() < 2 {
            // Need at least two results to detect contradictions.
            return Ok(());
        }

        let results_text: String = ctx
            .results
            .iter()
            .enumerate()
            .map(|(i, hit)| {
                format!(
                    "### Memory {} (id: {})\n{}\n",
                    i + 1,
                    hit.memory.id,
                    hit.memory.summary,
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let client = ClaudeClient::new();
        let prompt = build_contradiction_prompt(&results_text);
        let raw = client.prompt(&prompt).await?;
        let json = strip_code_fence(&raw);

        if let Ok(parsed) = serde_json::from_str::<ContradictionResponse>(json) {
            ctx.metadata["contradictions"] =
                serde_json::to_value(&parsed.contradictions).unwrap_or(serde_json::Value::Null);
            ctx.metadata["contradiction_summary"] = serde_json::Value::String(parsed.summary);
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- QueryRewriting ---

    #[test]
    fn query_rewriting_prompt_contains_query() {
        let p = build_query_rewriting_prompt("how do I fix ENOMEM?");
        assert!(p.contains("how do I fix ENOMEM?"));
        assert!(p.contains("rewritten query text"));
    }

    // --- MultiQuery ---

    #[test]
    fn multi_query_prompt_contains_query() {
        let p = build_multi_query_prompt("async rust errors");
        assert!(p.contains("async rust errors"));
        assert!(p.contains("JSON array"));
    }

    #[test]
    fn parse_multi_query_valid_array() {
        let raw = r#"["variant 1", "variant 2", "variant 3"]"#;
        let v = parse_multi_query_response(raw);
        assert_eq!(v, vec!["variant 1", "variant 2", "variant 3"]);
    }

    #[test]
    fn parse_multi_query_with_code_fence() {
        let raw = "```json\n[\"a\", \"b\"]\n```";
        let v = parse_multi_query_response(raw);
        assert_eq!(v, vec!["a", "b"]);
    }

    #[test]
    fn parse_multi_query_malformed_returns_empty() {
        let raw = "not valid json at all";
        let v = parse_multi_query_response(raw);
        assert!(v.is_empty());
    }

    // --- StepBack ---

    #[test]
    fn step_back_prompt_contains_query() {
        let p = build_step_back_prompt("ENOMEM in rustls v0.21");
        assert!(p.contains("ENOMEM in rustls v0.21"));
        assert!(p.contains("abstract"));
    }

    // --- HyDE ---

    #[test]
    fn hyde_prompt_contains_query() {
        let p = build_hyde_prompt("what causes TCP connection resets?");
        assert!(p.contains("what causes TCP connection resets?"));
        assert!(p.contains("answer text"));
    }

    // --- Decomposition ---

    #[test]
    fn decomposition_prompt_contains_query() {
        let p = build_decomposition_prompt("async rust error handling with tokio");
        assert!(p.contains("async rust error handling with tokio"));
        assert!(p.contains("JSON array"));
    }

    // --- RetrieveReadRetrieve ---

    #[test]
    fn rrr_prompt_contains_query_and_summaries() {
        let p = build_rrr_prompt("my query", "1. Some result\n2. Another result");
        assert!(p.contains("my query"));
        assert!(p.contains("1. Some result"));
    }

    // --- QueryRefinement ---

    #[test]
    fn query_refinement_prompt_contains_query_and_titles() {
        let p = build_query_refinement_prompt("my query", "1. Title A\n2. Title B");
        assert!(p.contains("my query"));
        assert!(p.contains("Title A"));
    }

    // --- ContradictionDetection ---

    #[test]
    fn contradiction_prompt_contains_memories() {
        let p = build_contradiction_prompt("### Memory 1 (id: abc)\nsome text");
        assert!(p.contains("Memory 1"));
        assert!(p.contains("contradictions"));
    }

    #[test]
    fn contradiction_response_parses_empty_array() {
        let raw = r#"{"contradictions": [], "summary": "No contradictions found."}"#;
        let parsed: ContradictionResponse = serde_json::from_str(raw).expect("must parse");
        assert!(parsed.contradictions.is_empty());
        assert_eq!(parsed.summary, "No contradictions found.");
    }

    #[test]
    fn contradiction_response_parses_pairs() {
        let raw = r#"{"contradictions": [["id-1", "id-2"]], "summary": "Two memories conflict."}"#;
        let parsed: ContradictionResponse = serde_json::from_str(raw).expect("must parse");
        assert_eq!(parsed.contradictions.len(), 1);
        assert_eq!(
            parsed.contradictions[0],
            ["id-1".to_owned(), "id-2".to_owned()]
        );
    }

    // --- Stage names ---

    #[test]
    fn all_stage_names_are_non_empty() {
        let stages: Vec<Box<dyn std::any::Any>> = vec![];
        let names = [
            PipelineStage::name(&QueryRewriting),
            PipelineStage::name(&MultiQuery),
            PipelineStage::name(&StepBack),
            PipelineStage::name(&HyDE),
            PipelineStage::name(&Decomposition),
            PipelineStage::name(&RetrieveReadRetrieve),
            PipelineStage::name(&QueryRefinement),
            PipelineStage::name(&ContradictionDetection),
        ];
        let _ = stages;
        for name in &names {
            assert!(!name.is_empty(), "stage name '{name}' must not be empty");
        }
    }

    #[test]
    fn all_stage_names_are_unique() {
        let names = [
            PipelineStage::name(&QueryRewriting),
            PipelineStage::name(&MultiQuery),
            PipelineStage::name(&StepBack),
            PipelineStage::name(&HyDE),
            PipelineStage::name(&Decomposition),
            PipelineStage::name(&RetrieveReadRetrieve),
            PipelineStage::name(&QueryRefinement),
            PipelineStage::name(&ContradictionDetection),
        ];
        let mut seen = std::collections::HashSet::new();
        for name in &names {
            assert!(seen.insert(name), "duplicate stage name: {name}");
        }
    }
}
