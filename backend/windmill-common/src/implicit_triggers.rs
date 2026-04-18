//! Implicit asset triggers: projected from `#trigger: asset` annotations
//! in script source onto rows in `asset_trigger` with `is_implicit = true`.
//!
//! Every script save re-parses annotations and calls
//! [`sync_implicit_asset_triggers_for_script`], which computes the
//! subscription set from the script's inferred read assets and upserts one
//! row per asset annotation (v1 allows at most one). Implicit rows are never
//! edited via API or CLI — the trigger stays in perfect sync with the script.

use serde::{Deserialize, Serialize};
use sqlx::{types::Json, Postgres, Transaction};

use windmill_types::assets::{AssetKind, AssetUsageAccessType, AssetWithAltAccessType};

use crate::{
    error::Error,
    scripts::ScriptLang,
    trigger_annotations::{
        parse_trigger_annotations, AssetTriggerAnnotation, TriggerAnnotation,
        TriggerAnnotationError,
    },
};

/// Deterministic path used by implicit asset triggers. Keeps the original
/// script path prefix so row-level-security policies on `asset_trigger`
/// work unchanged (`SPLIT_PART(path, '/', 1)` — 'u' / 'f' / 'g').
pub const IMPLICIT_TRIGGER_PATH_SUFFIX: &str = "/__asset_trigger__";

pub fn implicit_trigger_path(script_path: &str) -> String {
    format!("{script_path}{IMPLICIT_TRIGGER_PATH_SUFFIX}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionEntry {
    pub kind: AssetKind,
    pub path: String,
}

/// JSONB shape stored in `asset_trigger.subscription_set`. Uses an object
/// wrapper with a `paths` array so stage 8 (partitioning) can add sibling
/// fields without a migration. Indexed with GIN — containment queries like
/// `subscription_set -> 'paths' @> ...` remain indexable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionSet {
    pub paths: Vec<SubscriptionEntry>,
}

#[derive(thiserror::Error, Debug)]
pub enum SyncError {
    #[error("trigger annotation: {0}")]
    Annotation(#[from] TriggerAnnotationError),
    #[error("sql: {0}")]
    Sql(#[from] sqlx::Error),
}

impl From<SyncError> for Error {
    fn from(e: SyncError) -> Self {
        Error::BadRequest(e.to_string())
    }
}

/// Re-sync implicit asset triggers for a script. Called inside the script-
/// save transaction after the script row has been inserted.
///
/// `inferred_read_assets` is the static asset-usage list captured at save
/// time (`ns.assets` on the save handler). Only entries with `r` or `rw`
/// access contribute to the subscription set — reactive triggers subscribe
/// to *upstreams*, not the script's own writes.
pub async fn sync_implicit_asset_triggers_for_script(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    script_path: &str,
    script_hash: i64,
    language: ScriptLang,
    content: &str,
    inferred_read_assets: &[AssetWithAltAccessType],
    author_email: &str,
    author_username: &str,
) -> std::result::Result<(), SyncError> {
    let anns = parse_trigger_annotations(content, language)?;
    let asset_annotations: Vec<&AssetTriggerAnnotation> = anns
        .iter()
        .map(|a| match a {
            TriggerAnnotation::Asset(inner) => inner,
        })
        .collect();

    // Delete all existing implicit rows for this owner — we'll re-insert the
    // current ones below. Safer than a diff since v1 is single-annotation.
    sqlx::query!(
        "DELETE FROM asset_trigger \
         WHERE workspace_id = $1 AND owner_script_path = $2 AND is_implicit = true",
        workspace_id,
        script_path,
    )
    .execute(&mut **tx)
    .await?;

    for ann in asset_annotations {
        let paths = resolve_subscription_set(ann, inferred_read_assets);
        let subscription_set = SubscriptionSet { paths };
        let trigger_path = implicit_trigger_path(script_path);
        let on_event = match ann.on {
            crate::trigger_annotations::AssetTriggerEvent::Change => "change",
        };
        let fires = match ann.fires {
            crate::trigger_annotations::FiresMode::All => "all",
            crate::trigger_annotations::FiresMode::Any => "any",
        };
        let backlog = match ann.backlog {
            crate::trigger_annotations::BacklogMode::Coalesce => "coalesce",
            crate::trigger_annotations::BacklogMode::Replay => "replay",
            crate::trigger_annotations::BacklogMode::Skip => "skip",
        };
        sqlx::query!(
            "INSERT INTO asset_trigger ( \
                workspace_id, path, script_path, is_flow, \
                owner_script_path, owner_script_hash, is_implicit, \
                on_event, subscription_set, fires, debounce_s, partition_map, \
                cancel_on_new, backlog, \
                edited_by, email \
             ) VALUES ( \
                $1, $2, $3, false, \
                $3, $4, true, \
                $5, $6, $7, $8, $9, \
                $10, $11, \
                $12, $13 \
             )",
            workspace_id,
            trigger_path,
            script_path,
            script_hash,
            on_event,
            Json(&subscription_set) as _,
            fires,
            ann.debounce_s,
            Json(&ann.partition_map) as _,
            ann.cancel_on_new,
            backlog,
            author_username,
            author_email,
        )
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

/// Resolve the reactive subscription set from the annotation and the
/// script's inferred reads. `only` overrides the inferred reads entirely;
/// otherwise we take inferred reads minus `exclude` plus `extra`, filtered
/// by `kinds` if specified. Writes are ignored.
fn resolve_subscription_set(
    ann: &AssetTriggerAnnotation,
    inferred_reads: &[AssetWithAltAccessType],
) -> Vec<SubscriptionEntry> {
    let read_paths: Vec<SubscriptionEntry> = if let Some(only) = &ann.only {
        // `only` specifies bare paths without kind — default to Resource.
        // Kinds filter (below) still applies, so `kinds=[s3object]` with
        // `only=[f/foo]` yields an s3object subscription.
        let kind = ann
            .kinds
            .as_ref()
            .and_then(|ks| ks.first().copied())
            .unwrap_or(AssetKind::Resource);
        only.iter()
            .map(|p| SubscriptionEntry { kind, path: p.clone() })
            .collect()
    } else {
        inferred_reads
            .iter()
            .filter(|a| {
                matches!(
                    a.access_type.or(a.alt_access_type),
                    Some(AssetUsageAccessType::R) | Some(AssetUsageAccessType::RW)
                )
            })
            .filter(|a| !ann.exclude.iter().any(|e| e == &a.path))
            .map(|a| SubscriptionEntry { kind: a.kind, path: a.path.clone() })
            .collect()
    };

    let extras = ann.extra.iter().map(|p| SubscriptionEntry {
        kind: ann
            .kinds
            .as_ref()
            .and_then(|ks| ks.first().copied())
            .unwrap_or(AssetKind::Resource),
        path: p.clone(),
    });

    let mut all: Vec<SubscriptionEntry> = read_paths.into_iter().chain(extras).collect();

    if let Some(kinds) = &ann.kinds {
        all.retain(|e| kinds.contains(&e.kind));
    }

    // Dedup (preserve first-seen order).
    let mut seen = std::collections::HashSet::new();
    all.retain(|e| seen.insert((e.kind, e.path.clone())));
    all
}

/// Delete all implicit asset triggers owned by a script. Called when a
/// script is archived or deleted.
pub async fn delete_implicit_asset_triggers_for_script(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    script_path: &str,
) -> std::result::Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM asset_trigger \
         WHERE workspace_id = $1 AND owner_script_path = $2 AND is_implicit = true",
        workspace_id,
        script_path,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Rename implicit triggers when the owning script is renamed. Keeps the
/// trigger path aligned with the new script path so the deterministic
/// addressing remains stable.
pub async fn rename_implicit_asset_triggers_for_script(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    old_script_path: &str,
    new_script_path: &str,
) -> std::result::Result<(), sqlx::Error> {
    let new_trigger_path = implicit_trigger_path(new_script_path);
    sqlx::query!(
        "UPDATE asset_trigger \
         SET owner_script_path = $1, script_path = $1, path = $2, server_id = NULL \
         WHERE workspace_id = $3 AND owner_script_path = $4 AND is_implicit = true",
        new_script_path,
        new_trigger_path,
        workspace_id,
        old_script_path,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
