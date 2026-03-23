//! Read pipeline for querying stored memories.
//!
//! The pipeline applies composable stages to a [`PipelineContext`]:
//!
//! | Module | Role |
//! |--------|------|
//! | `pipeline_runner` | [`PipelineRunner`] + [`PipelineStage`] trait + [`PipelineContext`] |
//! | `query_techniques` | 8 query transformation techniques |
//! | `result_refinement` | 7 result refinement techniques |
//! | `search_modes` | 6 search execution modes |

pub mod pipeline_runner;
pub mod query_techniques;
pub mod result_refinement;
pub mod search_modes;

pub use pipeline_runner::{PipelineContext, PipelineRunner, PipelineStage};
