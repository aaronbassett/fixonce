//! Write pipeline stages.
//!
//! Each module is an independent, testable stage:
//!
//! | Module            | Stage                                    |
//! |-------------------|------------------------------------------|
//! | `credential_check` | Regex-based credential / PII detection  |
//! | `quality_gate`    | Claude-powered signal-quality assessment |
//! | `dedup`           | Embedding + Claude deduplication         |
//! | `enrichment`      | Heuristic metadata enrichment            |

pub mod credential_check;
pub mod dedup;
pub mod enrichment;
pub mod quality_gate;
