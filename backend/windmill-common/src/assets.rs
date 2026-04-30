use sqlx::PgExecutor;

use crate::{error, scripts::ScriptHash};

pub use windmill_parser::asset_parser::{parse_pipeline_annotations, TriggerSpec};
pub use windmill_types::assets::*;

#[derive(sqlx::Type, Debug, Clone, Copy, PartialEq)]
#[sqlx(type_name = "SCRIPT_TRIGGER_KIND", rename_all = "lowercase")]
pub enum ScriptTriggerKind {
    Asset,
    Schedule,
    Webhook,
    Email,
    Kafka,
    Mqtt,
    Nats,
    Postgres,
    Sqs,
    Gcp,
}

pub async fn insert_static_asset_usage<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    asset: &AssetWithAltAccessType,
    usage_path: &str,
    usage_kind: AssetUsageKind,
) -> error::Result<()> {
    // Convert columns BTreeMap to JSONB format
    let columns_json = asset
        .columns
        .as_ref()
        .map(|cols| serde_json::to_value(cols).unwrap_or(serde_json::Value::Null));

    sqlx::query!(
        r#"INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind, columns)
                VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING"#,
        workspace_id,
        asset.path,
        asset.kind as AssetKind,
        (asset.access_type.or(asset.alt_access_type)) as Option<AssetUsageAccessType>,
        usage_path,
        usage_kind as AssetUsageKind,
        columns_json as Option<serde_json::Value>
    )
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn clear_static_asset_usage<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    usage_path: &str,
    usage_kind: AssetUsageKind,
) -> error::Result<()> {
    sqlx::query!(
        r#"DELETE FROM asset WHERE workspace_id = $1 AND usage_path = $2 AND usage_kind = $3"#,
        workspace_id,
        usage_path,
        usage_kind as AssetUsageKind
    )
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn clear_static_asset_usage_by_script_hash<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    script_hash: ScriptHash,
) -> error::Result<()> {
    sqlx::query!(
        "DELETE FROM asset WHERE workspace_id = $1 AND usage_kind = 'script' AND usage_path = (SELECT path FROM script WHERE hash = $2 AND workspace_id = $1)",
        workspace_id,
        script_hash.0
    )
    .execute(executor)
    .await?;
    Ok(())
}

// Wipe all pipeline trigger declarations held by the given runnable. Used at
// deploy time: redeploying a script wipes its prior `// on` annotations so
// removing them implicitly un-declares those edges.
pub async fn clear_script_triggers<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    runnable_path: &str,
    runnable_kind: AssetUsageKind,
) -> error::Result<()> {
    sqlx::query!(
        r#"DELETE FROM script_trigger
           WHERE workspace_id = $1 AND runnable_kind = $2 AND runnable_path = $3"#,
        workspace_id,
        runnable_kind as AssetUsageKind,
        runnable_path,
    )
    .execute(executor)
    .await?;
    Ok(())
}

// Reconcile a managed schedule for a pipeline script based on its parsed
// `// schedule "<cron>"` annotation. Idempotent: each call brings the
// `schedule` row in line with the annotation as of *this* deploy.
//
// The schedule lives at the same path as the runnable. `managed_by_runnable_path`
// disambiguates auto-created rows from user-managed ones — only managed
// rows are updated or removed by reconciliation; manually-created schedules
// at the same path are left alone (the annotation is silently ignored).
//
// Pass `cron = None` when the annotation has been removed → drops the
// managed row. Pass `cron = Some(...)` to upsert.
pub async fn reconcile_pipeline_schedule<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    runnable_path: &str,
    is_flow: bool,
    email: &str,
    edited_by: &str,
    permissioned_as: &str,
    cron: Option<&str>,
) -> error::Result<()> {
    match cron {
        Some(cron) => {
            // Upsert. ON CONFLICT only updates rows that are already managed
            // (the WHERE clause guards against trampling user-created
            // schedules that happen to live at the runnable's path).
            sqlx::query!(
                r#"
                INSERT INTO schedule (
                    workspace_id, path, schedule, timezone, edited_by, script_path,
                    is_flow, enabled, email, permissioned_as,
                    ws_error_handler_muted, no_flow_overlap, cron_version,
                    managed_by_runnable_path
                )
                VALUES ($1, $2, $3, 'UTC', $4, $2, $5, true, $6, $7, false, false, 'v2', $2)
                ON CONFLICT (workspace_id, path) DO UPDATE
                SET schedule = EXCLUDED.schedule,
                    edited_at = now(),
                    edited_by = EXCLUDED.edited_by,
                    managed_by_runnable_path = EXCLUDED.managed_by_runnable_path
                WHERE schedule.managed_by_runnable_path = EXCLUDED.managed_by_runnable_path
                   OR schedule.managed_by_runnable_path IS NULL AND schedule.script_path = EXCLUDED.script_path
                "#,
                workspace_id,
                runnable_path,
                cron,
                edited_by,
                is_flow,
                email,
                permissioned_as,
            )
            .execute(executor)
            .await?;
        }
        None => {
            // Drop any prior managed schedule for this runnable. Manual
            // schedules at the same path keep `managed_by_runnable_path =
            // NULL` and are unaffected.
            sqlx::query!(
                r#"DELETE FROM schedule
                   WHERE workspace_id = $1
                     AND managed_by_runnable_path = $2"#,
                workspace_id,
                runnable_path,
            )
            .execute(executor)
            .await?;
        }
    }
    Ok(())
}

// Drop the managed schedule (if any) for a runnable that's been deleted.
// Equivalent to `reconcile_pipeline_schedule(..., None)` but exposed
// separately so script-delete sites can call it without parsing annotations.
pub async fn delete_managed_pipeline_schedule<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    runnable_path: &str,
) -> error::Result<()> {
    sqlx::query!(
        r#"DELETE FROM schedule
           WHERE workspace_id = $1
             AND managed_by_runnable_path = $2"#,
        workspace_id,
        runnable_path,
    )
    .execute(executor)
    .await?;
    Ok(())
}

// Insert a single trigger declaration. Caller is expected to wipe first.
pub async fn insert_script_trigger<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    runnable_kind: AssetUsageKind,
    runnable_path: &str,
    trigger_kind: ScriptTriggerKind,
    trigger_ref: &str,
) -> error::Result<()> {
    sqlx::query!(
        r#"INSERT INTO script_trigger
             (workspace_id, runnable_kind, runnable_path, trigger_kind, trigger_ref)
           VALUES ($1, $2, $3, $4, $5)"#,
        workspace_id,
        runnable_kind as AssetUsageKind,
        runnable_path,
        trigger_kind as ScriptTriggerKind,
        trigger_ref,
    )
    .execute(executor)
    .await?;
    Ok(())
}

// Inverse of trigger_spec_to_row for the Asset variant: parses a stored
// trigger_ref (e.g. `s3://foo`, `$res:bar`) back into the (kind, path) pair
// used as a graph node id. Returns None for refs that don't match any known
// asset prefix — callers should skip those edges.
pub fn parse_asset_trigger_ref(s: &str) -> Option<(AssetKind, String)> {
    let (kind, path) = windmill_parser::asset_parser::parse_asset_syntax(s, false)?;
    Some((asset_kind_from_parser(kind), path.to_string()))
}

// Convert a parser TriggerSpec into the `(kind, ref)` pair stored in
// script_trigger. Asset refs get their canonical prefix back so the
// trigger_ref matches what downstream lookups expect.
pub fn trigger_spec_to_row(spec: &TriggerSpec) -> (ScriptTriggerKind, String) {
    match spec {
        TriggerSpec::Asset { asset_kind, path } => {
            let prefix = match asset_kind {
                windmill_parser::asset_parser::AssetKind::S3Object => "s3://",
                windmill_parser::asset_parser::AssetKind::Resource => "$res:",
                windmill_parser::asset_parser::AssetKind::Ducklake => "ducklake://",
                windmill_parser::asset_parser::AssetKind::DataTable => "datatable://",
                windmill_parser::asset_parser::AssetKind::Volume => "volume://",
            };
            (ScriptTriggerKind::Asset, format!("{}{}", prefix, path))
        }
        TriggerSpec::Schedule { cron } => (ScriptTriggerKind::Schedule, cron.clone()),
        TriggerSpec::Webhook { path } => (ScriptTriggerKind::Webhook, path.clone()),
        TriggerSpec::Email { path } => (ScriptTriggerKind::Email, path.clone()),
        TriggerSpec::Kafka { path } => (ScriptTriggerKind::Kafka, path.clone()),
        TriggerSpec::Mqtt { path } => (ScriptTriggerKind::Mqtt, path.clone()),
        TriggerSpec::Nats { path } => (ScriptTriggerKind::Nats, path.clone()),
        TriggerSpec::Postgres { path } => (ScriptTriggerKind::Postgres, path.clone()),
        TriggerSpec::Sqs { path } => (ScriptTriggerKind::Sqs, path.clone()),
        TriggerSpec::Gcp { path } => (ScriptTriggerKind::Gcp, path.clone()),
    }
}

pub fn asset_kind_from_parser(parser_kind: windmill_parser::asset_parser::AssetKind) -> AssetKind {
    match parser_kind {
        windmill_parser::asset_parser::AssetKind::S3Object => AssetKind::S3Object,
        windmill_parser::asset_parser::AssetKind::Resource => AssetKind::Resource,
        windmill_parser::asset_parser::AssetKind::Ducklake => AssetKind::Ducklake,
        windmill_parser::asset_parser::AssetKind::DataTable => AssetKind::DataTable,
        windmill_parser::asset_parser::AssetKind::Volume => AssetKind::Volume,
    }
}

pub fn asset_access_type_from_parser(
    parser_kind: windmill_parser::asset_parser::AssetUsageAccessType,
) -> AssetUsageAccessType {
    match parser_kind {
        windmill_parser::asset_parser::AssetUsageAccessType::R => AssetUsageAccessType::R,
        windmill_parser::asset_parser::AssetUsageAccessType::W => AssetUsageAccessType::W,
        windmill_parser::asset_parser::AssetUsageAccessType::RW => AssetUsageAccessType::RW,
    }
}
