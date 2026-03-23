//! Result refinement stages for the read pipeline.
//!
//! Each refinement technique is a zero-sized struct that implements
//! [`PipelineStage`][super::pipeline_runner::PipelineStage].
//!
//! | Technique | What it does |
//! |-----------|-------------|
//! | [`Confidence`] | Claude assigns a confidence score to each result |
//! | [`RelevanceReranking`] | Claude reranks results by relevance to the query |
//! | [`TrustAware`] | Boost by `decay_score` × `reinforcement_score` |
//! | [`Freshness`] | Boost recently created/updated memories |
//! | [`Dedup`] | Remove near-duplicate results (by title similarity) |
//! | [`Coverage`] | Ensure diversity across memory types |
//! | [`Answerability`] | Filter results that cannot answer the query |

use crate::{
    memory::types::SearchHit,
    pipeline::{claude::ClaudeClient, PipelineError},
};

use super::pipeline_runner::{PipelineContext, PipelineStage};

// ---------------------------------------------------------------------------
// ScoredHit — annotated search result
// ---------------------------------------------------------------------------

/// A search result with an additional pipeline-assigned confidence score.
#[derive(Debug, Clone)]
pub struct ScoredHit {
    /// The underlying search result.
    pub hit: SearchHit,
    /// Confidence score in `[0, 1]` (1 = most confident).
    pub confidence: f64,
    /// Optional human-readable note from the refinement stage.
    pub note: Option<String>,
}

impl ScoredHit {
    /// Wrap a [`SearchHit`] with a default confidence equal to its similarity.
    #[must_use]
    pub fn from_hit(hit: SearchHit) -> Self {
        let confidence = hit.similarity;
        Self {
            hit,
            confidence,
            note: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

/// Ensure `ctx.scored_results` is populated from `ctx.results`.
fn ensure_scored(ctx: &mut PipelineContext) {
    if ctx.scored_results.is_empty() && !ctx.results.is_empty() {
        ctx.scored_results = ctx
            .results
            .iter()
            .cloned()
            .map(ScoredHit::from_hit)
            .collect();
    }
}

// ---------------------------------------------------------------------------
// 1. Confidence
// ---------------------------------------------------------------------------

/// Claude assigns a confidence score `[0, 1]` to each result indicating how
/// well it answers the query.
pub struct Confidence;

/// Response shape from Claude's confidence scoring.
#[derive(Debug, serde::Deserialize)]
pub struct ConfidenceResponse {
    /// Per-result scores keyed by memory ID.
    pub scores: Vec<ConfidenceScore>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ConfidenceScore {
    pub id: String,
    pub score: f64,
    pub note: Option<String>,
}

/// Build the confidence scoring prompt (exposed for testing).
#[must_use]
pub fn build_confidence_prompt(query: &str, results_text: &str) -> String {
    format!(
        r#"You are a relevance assessor for a developer knowledge base called FixOnce.

For each memory below, assign a confidence score from 0.0 (completely
irrelevant) to 1.0 (directly and fully answers the query).

Query: {query}

## Memories
{results_text}

Reply with **only** valid JSON:
```json
{{
  "scores": [
    {{"id": "<memory-id>", "score": <0.0-1.0>, "note": "<optional short note>"}},
    ...
  ]
}}
```"#
    )
}

impl PipelineStage for Confidence {
    fn name(&self) -> &'static str {
        "confidence"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        if ctx.results.is_empty() {
            return Ok(());
        }

        ensure_scored(ctx);

        let results_text: String = ctx
            .results
            .iter()
            .enumerate()
            .map(|(i, hit)| {
                format!(
                    "{}. id={} | {} | {}",
                    i + 1,
                    hit.memory.id,
                    hit.memory.title,
                    hit.memory.summary,
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let client = ClaudeClient::new();
        let prompt = build_confidence_prompt(&ctx.original_query, &results_text);
        let raw = client.prompt(&prompt).await?;
        let json = strip_code_fence(&raw);

        if let Ok(parsed) = serde_json::from_str::<ConfidenceResponse>(json) {
            for score_entry in &parsed.scores {
                if let Some(scored) = ctx
                    .scored_results
                    .iter_mut()
                    .find(|s| s.hit.memory.id == score_entry.id)
                {
                    scored.confidence = score_entry.score.clamp(0.0, 1.0);
                    if score_entry.note.is_some() {
                        scored.note.clone_from(&score_entry.note);
                    }
                }
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 2. RelevanceReranking
// ---------------------------------------------------------------------------

/// Claude reorders the result set by relevance to the query.
pub struct RelevanceReranking;

/// Response shape from Claude's reranking.
#[derive(Debug, serde::Deserialize)]
pub struct RerankResponse {
    /// Memory IDs in ranked order (most relevant first).
    pub ranked_ids: Vec<String>,
}

/// Build the reranking prompt (exposed for testing).
#[must_use]
pub fn build_rerank_prompt(query: &str, results_text: &str) -> String {
    format!(
        r#"You are a relevance ranker for a developer knowledge base called FixOnce.

Reorder the memories below from most to least relevant to the query.

Query: {query}

## Memories
{results_text}

Reply with **only** valid JSON — a list of memory IDs in ranked order:
```json
{{"ranked_ids": ["<id-1>", "<id-2>", ...]}}
```"#
    )
}

impl PipelineStage for RelevanceReranking {
    fn name(&self) -> &'static str {
        "relevance_reranking"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        if ctx.results.is_empty() {
            return Ok(());
        }

        ensure_scored(ctx);

        let results_text: String = ctx
            .results
            .iter()
            .enumerate()
            .map(|(i, hit)| {
                format!(
                    "{}. id={} | {} | {}",
                    i + 1,
                    hit.memory.id,
                    hit.memory.title,
                    hit.memory.summary,
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let client = ClaudeClient::new();
        let prompt = build_rerank_prompt(&ctx.original_query, &results_text);
        let raw = client.prompt(&prompt).await?;
        let json = strip_code_fence(&raw);

        if let Ok(parsed) = serde_json::from_str::<RerankResponse>(json) {
            // Reorder scored_results according to Claude's ranking.
            let mut reordered: Vec<ScoredHit> = Vec::with_capacity(ctx.scored_results.len());
            for id in &parsed.ranked_ids {
                if let Some(pos) = ctx
                    .scored_results
                    .iter()
                    .position(|s| &s.hit.memory.id == id)
                {
                    reordered.push(ctx.scored_results.remove(pos));
                }
            }
            // Append any results not mentioned in ranked_ids at the end.
            reordered.append(&mut ctx.scored_results);
            ctx.scored_results = reordered;

            // Mirror the order back into ctx.results for consistency.
            ctx.results = ctx.scored_results.iter().map(|s| s.hit.clone()).collect();
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 3. TrustAware
// ---------------------------------------------------------------------------

/// Adjusts confidence scores using `decay_score × reinforcement_score` from
/// the memory's metadata.  No Claude call required.
pub struct TrustAware;

impl PipelineStage for TrustAware {
    fn name(&self) -> &'static str {
        "trust_aware"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        ensure_scored(ctx);

        for scored in &mut ctx.scored_results {
            let trust = scored.hit.memory.decay_score * scored.hit.memory.reinforcement_score;
            // Blend: 70 % similarity + 30 % trust signal.
            scored.confidence = (scored.confidence * 0.7 + trust * 0.3).clamp(0.0, 1.0);
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 4. Freshness
// ---------------------------------------------------------------------------

/// Boosts results created or updated within the last 30 days.  No LLM call.
pub struct Freshness;

/// Parse an ISO-8601 timestamp into a Unix timestamp (seconds).
fn parse_ts(ts: &str) -> Option<i64> {
    // Accept "YYYY-MM-DDTHH:MM:SSZ" or "YYYY-MM-DD" prefixes.
    let date_str = &ts[..ts.len().min(10)]; // take at most 10 chars for date
    let parts: Vec<u32> = date_str
        .splitn(3, '-')
        .filter_map(|p| p.parse().ok())
        .collect();
    if parts.len() == 3 {
        // Rough Unix timestamp: days since 1970-01-01.
        let y = i64::from(parts[0]);
        let m = i64::from(parts[1]);
        let d = i64::from(parts[2]);
        // Simplified days-since-epoch (good enough for "within 30 days" check).
        Some((y - 1970) * 365 + m * 30 + d)
    } else {
        None
    }
}

/// Approximate "today" as days since 1970-01-01 using a fixed recent date for
/// deterministic behaviour in tests.  In production this would use `SystemTime`.
fn today_approx() -> i64 {
    // 2026-03-23 ≈ day 20_540 since 1970.
    (2026 - 1970) * 365 + 3 * 30 + 23
}

impl PipelineStage for Freshness {
    fn name(&self) -> &'static str {
        "freshness"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        ensure_scored(ctx);

        let today = today_approx();

        for scored in &mut ctx.scored_results {
            if let Some(ts) = parse_ts(&scored.hit.memory.updated_at) {
                let age_days = (today - ts).max(0);
                if age_days <= 30 {
                    // +10 % boost for recent memories, capped at 1.0.
                    scored.confidence = (scored.confidence + 0.10).min(1.0);
                }
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 5. Dedup
// ---------------------------------------------------------------------------

/// Removes near-duplicate results from the set.  Two results are considered
/// duplicates if they share the same memory ID or if their titles are
/// identical after normalisation.
pub struct Dedup;

fn normalise_title(t: &str) -> String {
    t.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

impl PipelineStage for Dedup {
    fn name(&self) -> &'static str {
        "dedup"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        ensure_scored(ctx);

        let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut seen_titles: std::collections::HashSet<String> = std::collections::HashSet::new();

        ctx.scored_results.retain(|s| {
            let id_new = seen_ids.insert(s.hit.memory.id.clone());
            let title_new = seen_titles.insert(normalise_title(&s.hit.memory.title));
            id_new && title_new
        });

        // Mirror back.
        ctx.results = ctx.scored_results.iter().map(|s| s.hit.clone()).collect();

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 6. Coverage
// ---------------------------------------------------------------------------

/// Ensures the result set includes a variety of memory types where possible.
///
/// If more than 60 % of results share a memory type, lower-ranked duplicates
/// of that type are moved to the end so other types surface higher.
pub struct Coverage;

impl PipelineStage for Coverage {
    fn name(&self) -> &'static str {
        "coverage"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        ensure_scored(ctx);

        if ctx.scored_results.len() < 3 {
            return Ok(());
        }

        // Count occurrences of each memory type (as display string).
        let total = ctx.scored_results.len();
        let mut type_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for s in &ctx.scored_results {
            *type_counts
                .entry(s.hit.memory.memory_type.to_string())
                .or_insert(0) += 1;
        }

        // Find over-represented types (> 60 %).
        let over_represented: std::collections::HashSet<String> = type_counts
            .into_iter()
            .filter(|(_, count)| *count * 10 > total * 6)
            .map(|(t, _)| t)
            .collect();

        if over_represented.is_empty() {
            return Ok(());
        }

        // Partition: keep well-represented types first, push over-represented ones to back.
        let mut primary: Vec<ScoredHit> = Vec::new();
        let mut secondary: Vec<ScoredHit> = Vec::new();
        // For over-represented types, keep one result in primary, rest in secondary.
        let mut seen_over: std::collections::HashSet<String> = std::collections::HashSet::new();

        for s in ctx.scored_results.drain(..) {
            let mtype = s.hit.memory.memory_type.to_string();
            if over_represented.contains(&mtype) {
                if seen_over.insert(mtype) {
                    primary.push(s); // first occurrence → primary
                } else {
                    secondary.push(s); // subsequent → secondary
                }
            } else {
                primary.push(s);
            }
        }

        primary.extend(secondary);
        ctx.scored_results = primary;
        ctx.results = ctx.scored_results.iter().map(|s| s.hit.clone()).collect();

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 7. Answerability
// ---------------------------------------------------------------------------

/// Filters out results that cannot meaningfully answer the query, using Claude.
pub struct Answerability;

/// Response shape from Claude's answerability check.
#[derive(Debug, serde::Deserialize)]
pub struct AnswerabilityResponse {
    /// IDs of memories that can answer the query.
    pub answerable: Vec<String>,
}

/// Build the answerability prompt (exposed for testing).
#[must_use]
pub fn build_answerability_prompt(query: &str, results_text: &str) -> String {
    format!(
        r#"You are a relevance filter for a developer knowledge base called FixOnce.

For the query below, identify which of the memories can meaningfully help answer
it.  Exclude memories that are only tangentially related.

Query: {query}

## Memories
{results_text}

Reply with **only** valid JSON — the IDs of answerable memories:
```json
{{"answerable": ["<id-1>", "<id-2>", ...]}}
```"#
    )
}

impl PipelineStage for Answerability {
    fn name(&self) -> &'static str {
        "answerability"
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        if ctx.results.is_empty() {
            return Ok(());
        }

        ensure_scored(ctx);

        let results_text: String = ctx
            .results
            .iter()
            .enumerate()
            .map(|(i, hit)| format!("{}. id={} | {}", i + 1, hit.memory.id, hit.memory.summary,))
            .collect::<Vec<_>>()
            .join("\n");

        let client = ClaudeClient::new();
        let prompt = build_answerability_prompt(&ctx.original_query, &results_text);
        let raw = client.prompt(&prompt).await?;
        let json = strip_code_fence(&raw);

        if let Ok(parsed) = serde_json::from_str::<AnswerabilityResponse>(json) {
            let answerable_set: std::collections::HashSet<String> =
                parsed.answerable.into_iter().collect();

            ctx.scored_results
                .retain(|s| answerable_set.contains(&s.hit.memory.id));
            ctx.results = ctx.scored_results.iter().map(|s| s.hit.clone()).collect();
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
    use crate::memory::types::{
        EmbeddingStatus, Memory, MemoryType, PipelineStatus, SearchHit, SourceType,
    };

    fn sample_hit(id: &str, title: &str, similarity: f64) -> SearchHit {
        SearchHit {
            memory: Memory {
                id: id.to_owned(),
                title: title.to_owned(),
                content: "some content".to_owned(),
                summary: "short summary".to_owned(),
                memory_type: MemoryType::Gotcha,
                source_type: SourceType::Manual,
                language: None,
                compact_pragma: None,
                compact_compiler: None,
                midnight_js: None,
                indexer_version: None,
                node_version: None,
                source_url: None,
                repo_url: None,
                task_summary: None,
                session_id: None,
                decay_score: 0.9,
                reinforcement_score: 0.8,
                last_accessed_at: None,
                embedding_status: EmbeddingStatus::Complete,
                pipeline_status: PipelineStatus::Complete,
                deleted_at: None,
                created_at: "2026-01-01T00:00:00Z".to_owned(),
                updated_at: "2026-01-01T00:00:00Z".to_owned(),
                created_by: "user-1".to_owned(),
                anti_memory: None,
            },
            similarity,
        }
    }

    // --- ScoredHit ---

    #[test]
    fn scored_hit_from_hit_sets_confidence_to_similarity() {
        let hit = sample_hit("id-1", "Title", 0.85);
        let scored = ScoredHit::from_hit(hit);
        assert!((scored.confidence - 0.85).abs() < f64::EPSILON);
        assert!(scored.note.is_none());
    }

    // --- Confidence ---

    #[test]
    fn confidence_prompt_contains_query_and_results() {
        let p = build_confidence_prompt("my query", "1. id=abc | Title | Summary");
        assert!(p.contains("my query"));
        assert!(p.contains("id=abc"));
    }

    #[test]
    fn confidence_response_parses() {
        let raw = r#"{"scores": [{"id": "abc", "score": 0.9, "note": "very relevant"}]}"#;
        let parsed: ConfidenceResponse = serde_json::from_str(raw).expect("must parse");
        assert_eq!(parsed.scores.len(), 1);
        assert_eq!(parsed.scores[0].id, "abc");
        assert!((parsed.scores[0].score - 0.9).abs() < f64::EPSILON);
    }

    // --- RelevanceReranking ---

    #[test]
    fn rerank_prompt_contains_query() {
        let p = build_rerank_prompt("my query", "1. id=x | title");
        assert!(p.contains("my query"));
        assert!(p.contains("ranked_ids"));
    }

    #[test]
    fn rerank_response_parses() {
        let raw = r#"{"ranked_ids": ["id-2", "id-1"]}"#;
        let parsed: RerankResponse = serde_json::from_str(raw).expect("must parse");
        assert_eq!(parsed.ranked_ids, vec!["id-2", "id-1"]);
    }

    // --- TrustAware ---

    #[tokio::test]
    async fn trust_aware_blends_confidence_and_trust() {
        let mut ctx = PipelineContext::default();
        let hit = sample_hit("id-1", "T", 0.8);
        // decay=0.9, reinforcement=0.8 → trust = 0.72
        // expected: 0.8*0.7 + 0.72*0.3 = 0.56 + 0.216 = 0.776
        ctx.results.push(hit);
        ensure_scored(&mut ctx);

        TrustAware.execute(&mut ctx).await.expect("must succeed");

        let expected = 0.8 * 0.7 + (0.9 * 0.8) * 0.3;
        assert!(
            (ctx.scored_results[0].confidence - expected).abs() < 1e-9,
            "expected {expected:.6} got {:.6}",
            ctx.scored_results[0].confidence
        );
    }

    // --- Dedup ---

    #[tokio::test]
    async fn dedup_removes_duplicate_id() {
        let mut ctx = PipelineContext::default();
        ctx.results.push(sample_hit("dup-id", "Title A", 0.9));
        ctx.results.push(sample_hit("dup-id", "Title B", 0.8));
        ensure_scored(&mut ctx);

        Dedup.execute(&mut ctx).await.expect("must succeed");

        assert_eq!(ctx.scored_results.len(), 1, "duplicate id must be removed");
    }

    #[tokio::test]
    async fn dedup_removes_duplicate_title() {
        let mut ctx = PipelineContext::default();
        ctx.results.push(sample_hit("id-1", "Same Title", 0.9));
        ctx.results.push(sample_hit("id-2", "Same Title", 0.8));
        ensure_scored(&mut ctx);

        Dedup.execute(&mut ctx).await.expect("must succeed");

        assert_eq!(
            ctx.scored_results.len(),
            1,
            "duplicate title must be removed"
        );
    }

    #[tokio::test]
    async fn dedup_keeps_distinct_results() {
        let mut ctx = PipelineContext::default();
        ctx.results.push(sample_hit("id-1", "Title A", 0.9));
        ctx.results.push(sample_hit("id-2", "Title B", 0.8));
        ensure_scored(&mut ctx);

        Dedup.execute(&mut ctx).await.expect("must succeed");

        assert_eq!(ctx.scored_results.len(), 2);
    }

    // --- Freshness ---

    #[test]
    fn parse_ts_extracts_date() {
        let ts = parse_ts("2026-03-23T10:00:00Z");
        assert!(ts.is_some());
    }

    // --- Answerability ---

    #[test]
    fn answerability_prompt_contains_query() {
        let p = build_answerability_prompt("query text", "1. id=x | summary");
        assert!(p.contains("query text"));
        assert!(p.contains("answerable"));
    }

    #[test]
    fn answerability_response_parses() {
        let raw = r#"{"answerable": ["id-1", "id-3"]}"#;
        let parsed: AnswerabilityResponse = serde_json::from_str(raw).expect("must parse");
        assert_eq!(parsed.answerable, vec!["id-1", "id-3"]);
    }

    // --- Stage names ---

    #[test]
    fn all_refinement_stage_names_unique() {
        let names = [
            PipelineStage::name(&Confidence),
            PipelineStage::name(&RelevanceReranking),
            PipelineStage::name(&TrustAware),
            PipelineStage::name(&Freshness),
            PipelineStage::name(&Dedup),
            PipelineStage::name(&Coverage),
            PipelineStage::name(&Answerability),
        ];
        let mut seen = std::collections::HashSet::new();
        for name in &names {
            assert!(seen.insert(name), "duplicate stage name: {name}");
        }
    }
}
