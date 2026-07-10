//! Shared types for the per-user draft surface.
//!
//! Mirrors the `OtherDraftUser` type in `windmill-common::user_drafts` —
//! kept here so `windmill-types` row structs (ListableScript / ListableFlow /
//! ListableApp) can expose a typed `draft_users` field without taking a
//! dependency on `windmill-common`. The two structs serialize identically,
//! so the frontend doesn't notice.

use serde::{Deserialize, Serialize};

/// One workspace user (or the legacy NULL-email row) with a per-user draft
/// at a given path. Used by the home-page list endpoints to feed the
/// avatar-circles inside the Draft badge.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DraftUserRef {
    /// `None` represents a legacy workspace-level draft (no owner).
    pub username: Option<String>,
}
