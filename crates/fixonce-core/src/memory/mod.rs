//! Memory model types, metadata, provenance, and dynamics.

pub mod contradictions;
pub mod dynamics;
pub mod lineage;
pub mod metadata;
pub mod provenance;
pub mod signatures;
pub mod types;

pub use metadata::VersionMetadata;
pub use provenance::Provenance;
pub use types::{
    AntiMemory, CreateMemoryRequest, CreateMemoryResponse, DeleteMemoryResponse, EmbeddingStatus,
    Feedback, FeedbackRating, Memory, MemoryType, PipelineStatus, SearchHit, SearchMemoryRequest,
    SearchMemoryResponse, SourceType, UpdateMemoryResponse,
};
