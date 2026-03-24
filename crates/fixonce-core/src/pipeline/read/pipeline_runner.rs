//! Composable read pipeline runner.
//!
//! The runner chains [`PipelineStage`] implementations sequentially.  Two
//! preset pipelines are provided:
//!
//! | Preset | Stages |
//! |--------|--------|
//! | Default | query rewriting → hybrid search → relevance reranking |
//! | Deep (`--deep`) | multi-query → `HyDE` → hybrid → retrieve-read-retrieve → confidence → reranking → coverage |
//!
//! Each stage receives a mutable [`PipelineContext`] and may mutate it.  If
//! any stage returns an error the pipeline halts immediately.

use tracing::info_span;

use crate::memory::types::SearchHit;

use super::{
    query_techniques::{HyDE, MultiQuery, QueryRewriting, RetrieveReadRetrieve},
    result_refinement::{Coverage, RelevanceReranking, ScoredHit},
    search_modes::HybridSearch,
};
use crate::pipeline::PipelineError;

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

/// Mutable context passed through every stage.
///
/// Stages accumulate their work here; later stages can depend on the output of
/// earlier ones.
#[derive(Debug, Default)]
pub struct PipelineContext {
    /// The user's original, unmodified query.
    pub original_query: String,
    /// Rewritten or variant queries produced by query techniques.
    pub rewritten_queries: Vec<String>,
    /// Embeddings for all active queries (original + variants).
    pub embeddings: Vec<Vec<f64>>,
    /// Current set of search results, updated by search and refinement stages.
    pub results: Vec<SearchHit>,
    /// Scored / annotated results produced by refinement stages.
    pub scored_results: Vec<ScoredHit>,
    /// Freeform stage metadata (for debugging / logging).
    pub metadata: serde_json::Value,
    /// Whether Claude was unavailable during the pipeline run.
    pub degraded: bool,
}

impl PipelineContext {
    /// Create a new context for the given query.
    #[must_use]
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            original_query: query.into(),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Stage trait
// ---------------------------------------------------------------------------

/// A single, independently testable stage in the read pipeline.
///
/// Stages must be `Send + Sync` so the pipeline runner can be used in an
/// async context.
pub trait PipelineStage: Send + Sync {
    /// Human-readable stage identifier (used in logs / metadata).
    fn name(&self) -> &str;

    /// Execute the stage, mutating `ctx` in place.
    ///
    /// # Errors
    ///
    /// Returns [`PipelineError`] if the stage fails in a way that should halt
    /// the pipeline.
    fn execute(
        &self,
        ctx: &mut PipelineContext,
    ) -> impl std::future::Future<Output = Result<(), PipelineError>> + Send;
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

/// Runs a sequence of [`PipelineStage`]s against a [`PipelineContext`].
pub struct PipelineRunner {
    stages: Vec<Box<dyn PipelineStageObject>>,
}

/// Object-safe wrapper trait for [`PipelineStage`].
///
/// This is necessary because `PipelineStage` uses an associated future in its
/// `execute` method (RPITIT), which is not object-safe.  We bridge it here
/// with a boxed future.
pub trait PipelineStageObject: Send + Sync {
    fn name(&self) -> &str;
    fn execute<'a>(
        &'a self,
        ctx: &'a mut PipelineContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), PipelineError>> + Send + 'a>>;
}

/// Blanket implementation: any [`PipelineStage`] can be wrapped as a
/// [`PipelineStageObject`].
impl<S: PipelineStage> PipelineStageObject for S {
    fn name(&self) -> &str {
        PipelineStage::name(self)
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a mut PipelineContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), PipelineError>> + Send + 'a>>
    {
        Box::pin(PipelineStage::execute(self, ctx))
    }
}

impl PipelineRunner {
    /// Create a runner from an explicit list of stages.
    #[must_use]
    pub fn new(stages: Vec<Box<dyn PipelineStageObject>>) -> Self {
        Self { stages }
    }

    /// Default three-stage pipeline.
    ///
    /// query rewriting → hybrid search → relevance reranking
    #[must_use]
    pub fn default_pipeline() -> Self {
        Self::new(vec![
            Box::new(QueryRewriting),
            Box::new(HybridSearch),
            Box::new(RelevanceReranking),
        ])
    }

    /// Deep seven-stage pipeline (use with `--deep`).
    ///
    /// multi-query → `HyDE` → hybrid → retrieve-read-retrieve →
    /// confidence → reranking → coverage
    #[must_use]
    pub fn deep_pipeline() -> Self {
        use super::result_refinement::Confidence;

        Self::new(vec![
            Box::new(MultiQuery),
            Box::new(HyDE),
            Box::new(HybridSearch),
            Box::new(RetrieveReadRetrieve),
            Box::new(Confidence),
            Box::new(RelevanceReranking),
            Box::new(Coverage),
        ])
    }

    /// Execute every stage in order against `ctx`.
    ///
    /// Execution halts on the first error.  Degraded-mode errors from Claude
    /// are recorded in `ctx.degraded` rather than propagated.
    ///
    /// # Errors
    ///
    /// Returns [`PipelineError`] on non-recoverable stage failures.
    pub async fn run(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        for stage in &self.stages {
            let span = info_span!("pipeline.stage", stage.name = stage.name());
            let _guard = span.enter();

            match stage.execute(ctx).await {
                Ok(()) => {}
                Err(
                    PipelineError::ClaudeNotFound
                    | PipelineError::ClaudeTimeout { .. }
                    | PipelineError::ClaudeExitFailure { .. },
                ) => {
                    // Degrade gracefully: mark context and continue with raw results.
                    ctx.degraded = true;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Return the names of all stages in this runner.
    #[must_use]
    pub fn stage_names(&self) -> Vec<&str> {
        self.stages.iter().map(|s| s.name()).collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Mock stage for composition testing ---

    struct RecordingStage {
        id: u32,
    }

    impl PipelineStage for RecordingStage {
        fn name(&self) -> &str {
            "recording"
        }

        async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
            // Record that this stage ran by pushing to rewritten_queries.
            ctx.rewritten_queries.push(format!("stage-{}", self.id));
            Ok(())
        }
    }

    struct FailingStage;

    impl PipelineStage for FailingStage {
        fn name(&self) -> &str {
            "failing"
        }

        async fn execute(&self, _ctx: &mut PipelineContext) -> Result<(), PipelineError> {
            Err(PipelineError::Api("intentional test failure".to_owned()))
        }
    }

    struct ClaudeUnavailableStage;

    impl PipelineStage for ClaudeUnavailableStage {
        fn name(&self) -> &str {
            "claude-unavailable"
        }

        async fn execute(&self, _ctx: &mut PipelineContext) -> Result<(), PipelineError> {
            Err(PipelineError::ClaudeNotFound)
        }
    }

    // --- Tests ---

    #[tokio::test]
    async fn stages_execute_in_order() {
        let runner = PipelineRunner::new(vec![
            Box::new(RecordingStage { id: 1 }),
            Box::new(RecordingStage { id: 2 }),
            Box::new(RecordingStage { id: 3 }),
        ]);

        let mut ctx = PipelineContext::new("test query");
        runner.run(&mut ctx).await.expect("pipeline must succeed");

        assert_eq!(
            ctx.rewritten_queries,
            vec!["stage-1", "stage-2", "stage-3"],
            "stages must execute in insertion order"
        );
    }

    #[tokio::test]
    async fn pipeline_halts_on_api_error() {
        let runner = PipelineRunner::new(vec![
            Box::new(RecordingStage { id: 1 }),
            Box::new(FailingStage),
            Box::new(RecordingStage { id: 3 }),
        ]);

        let mut ctx = PipelineContext::new("test query");
        let err = runner.run(&mut ctx).await.expect_err("must fail");
        assert!(matches!(err, PipelineError::Api(_)));

        // Stage 3 must not have run.
        assert_eq!(ctx.rewritten_queries, vec!["stage-1"]);
    }

    #[tokio::test]
    async fn claude_unavailable_sets_degraded_flag() {
        let runner = PipelineRunner::new(vec![
            Box::new(RecordingStage { id: 1 }),
            Box::new(ClaudeUnavailableStage),
            Box::new(RecordingStage { id: 3 }),
        ]);

        let mut ctx = PipelineContext::new("test query");
        runner
            .run(&mut ctx)
            .await
            .expect("must not fail on Claude unavailability");

        assert!(ctx.degraded, "context must be marked degraded");
        // Stage 3 must have run (pipeline continues).
        assert_eq!(ctx.rewritten_queries, vec!["stage-1", "stage-3"]);
    }

    #[tokio::test]
    async fn context_preserves_original_query() {
        let ctx = PipelineContext::new("my query");
        assert_eq!(ctx.original_query, "my query");
    }

    #[test]
    fn default_pipeline_has_three_stages() {
        let p = PipelineRunner::default_pipeline();
        assert_eq!(p.stage_names().len(), 3);
    }

    #[test]
    fn deep_pipeline_has_seven_stages() {
        let p = PipelineRunner::deep_pipeline();
        assert_eq!(p.stage_names().len(), 7);
    }

    #[test]
    fn stage_names_are_accessible() {
        let p = PipelineRunner::default_pipeline();
        let names = p.stage_names();
        // All names are non-empty.
        for name in &names {
            assert!(!name.is_empty(), "stage name must not be empty");
        }
    }
}
