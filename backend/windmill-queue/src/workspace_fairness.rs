//! Per-workspace fairness for the shared worker pool (Enterprise feature).
//!
//! The real algorithm — overloaded-set aggregation, coordinated refresh on
//! `background_task_state`, audit emission, stochastic admission decision —
//! lives in [`crate::workspace_fairness_ee`] and only compiles when the
//! `private` feature is on. This module is the public surface used by the
//! pull dispatch in `jobs.rs` and the integration tests; when EE is on it
//! transparently re-exports the EE implementation, when EE is off it
//! provides no-op stubs so the OSS build stays bit-identical to the
//! pre-fairness pull path.
//!
//! See [`crate::workspace_fairness_ee`] for design notes and SQL details.

#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::workspace_fairness_ee::*;

#[cfg(not(feature = "private"))]
mod oss_stubs {
    use sqlx::{Pool, Postgres};

    /// No-op on OSS — workspace fairness is an Enterprise feature.
    #[inline]
    pub fn maybe_refresh_overloaded(_db: &Pool<Postgres>) {}

    /// No-op on OSS — always returns `true` so the dispatch never reaches
    /// the fairness pull query (which is empty anyway, since
    /// `store_pull_query` only materialises it when fairness is enabled).
    #[inline]
    pub fn should_admit_capped() -> bool {
        true
    }
}

#[cfg(not(feature = "private"))]
pub use oss_stubs::*;
