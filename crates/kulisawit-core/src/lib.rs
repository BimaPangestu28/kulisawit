//! Kulisawit domain types, adapter trait, orchestrator
//!
//! See the workspace root `README.md` and `docs/PRD.md` for the product brief.

pub mod error;
pub mod ids;
pub mod status;

pub use error::{CoreError, CoreResult};
pub use ids::{BuahId, ColumnId, KebunId, LahanId};
pub use status::{BuahStatus, RunStatus, SortirStatus, UnknownBuahStatus};
