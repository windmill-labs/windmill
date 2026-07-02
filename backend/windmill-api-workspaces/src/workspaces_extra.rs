use std::collections::HashMap;

use windmill_api_auth::{require_super_admin, ApiAuthed};
use windmill_common::DB;

use crate::workspaces::{
    archive_workspace_impl, check_w_id_conflict, CREATE_WORKSPACE_REQUIRE_SUPERADMIN,
};

use axum::extract::Query;
use axum::{
    extract::{Extension, Path},
    Json,
};

use sqlx::{Postgres, Transaction};
use tracing::info;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;

use windmill_common::worker::CLOUD_HOSTED;

use windmill_common::{
    auth::is_super_admin_email,
    db::UserDB,
    error::{Error, Result},
    utils::require_admin,
    workspaces::{DataTable, DEV_WORKSPACE_LOCK_RULE_NAME, WM_FORK_PREFIX},
};
use windmill_queue::schedule::{get_schedule_opt, push_scheduled_job};

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct ChangeWorkspaceId {
    new_id: String,
    new_name: String,
}

pub(crate) async fn change_workspace_id(
    authed: ApiAuthed,
    Path(old_id): Path<String>,
    Extension(db): Extension<DB>,
    Json(rw): Json<ChangeWorkspaceId>,
) -> Result<String> {
    if *CLOUD_HOSTED && !is_super_admin_email(&db, &authed.email).await? {
        return Err(Error::BadRequest(
            "This feature is not available on the cloud".to_string(),
        ));
    }

    if *CREATE_WORKSPACE_REQUIRE_SUPERADMIN {
        require_super_admin(&db, &authed.email).await?;
    } else {
        require_admin(authed.is_admin, &authed.username)?;
    }

    let mut tx = db.begin().await?;

    check_w_id_conflict(&mut tx, &rw.new_id).await?;

    info!(
        "Changing workspace id from {} to {} (move and archive approach)",
        old_id, rw.new_id
    );

    // Create new workspace with new id and name. Fork lineage AND the dev designation are preserved
    // from the source row, not inferred from the new id's prefix: a prefix-less fork (a dev or
    // detached-dev workspace) would otherwise be silently promoted to a root workspace, and a dev
    // would lose its flag — leaving its prod locked with no canonical dev. Promoting out of a fork is
    // a separate, explicit action — a rename never does it implicitly.
    info!("Creating new workspace row");
    let old = sqlx::query!(
        r#"SELECT (parent_workspace_id IS NOT NULL) AS "has_parent!", is_dev_workspace
           FROM workspace WHERE id = $1"#,
        &old_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let new_is_fork = old.as_ref().map(|o| o.has_parent).unwrap_or(false);
    let new_is_dev = new_is_fork && old.as_ref().map(|o| o.is_dev_workspace).unwrap_or(false);
    // Neutralize the old row's dev flag BEFORE inserting the new one: the move-and-archive archives
    // the old row only later, so without this the new dev row and the not-yet-archived old dev row
    // would momentarily both be active under the same parent and trip the one-dev-per-parent index.
    if new_is_dev {
        sqlx::query!(
            "UPDATE workspace SET is_dev_workspace = false WHERE id = $1",
            &old_id
        )
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query!(
        "INSERT INTO workspace (id, name, owner, deleted, premium, parent_workspace_id, is_dev_workspace)
         SELECT $1, $2, owner, false, premium,
                CASE WHEN $4 THEN parent_workspace_id ELSE NULL END, $5
         FROM workspace WHERE id = $3",
        &rw.new_id,
        &rw.new_name,
        &old_id,
        new_is_fork,
        new_is_dev
    )
    .execute(&mut *tx)
    .await?;

    // Duplicate workspace settings (keep copy in old workspace for reference)
    info!("Duplicating workspace_settings table");
    sqlx::query!(
        "INSERT INTO workspace_settings SELECT $1, slack_team_id, slack_name, slack_command_script, slack_email, customer_id, plan, webhook, deploy_to, ai_config, large_file_storage, git_sync, default_app, default_scripts, deploy_ui, mute_critical_alerts, color, operator_settings, teams_command_script, teams_team_id, teams_team_name, git_app_installations, ducklake, slack_oauth_client_id, slack_oauth_client_secret, datatable, teams_team_guid, auto_invite, error_handler, success_handler FROM workspace_settings WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Duplicating workspace_key table");
    sqlx::query!(
        "INSERT INTO workspace_key SELECT $1, kind, key FROM workspace_key WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating workspace_env table");
    sqlx::query!(
        "UPDATE workspace_env SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating workspace_invite table");
    sqlx::query!(
        "UPDATE workspace_invite SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating account table");
    sqlx::query!(
        "UPDATE account SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating app table");
    sqlx::query!(
        "UPDATE app SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating capture table");
    sqlx::query!(
        "UPDATE capture SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating capture_config table");
    sqlx::query!(
        "UPDATE capture_config SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating http_trigger table");
    sqlx::query!(
        "UPDATE http_trigger SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating websocket_trigger table");
    sqlx::query!(
        "UPDATE websocket_trigger SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating kafka_trigger table");
    sqlx::query!(
        "UPDATE kafka_trigger SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating nats_trigger table");
    sqlx::query!(
        "UPDATE nats_trigger SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating postgres_trigger table");
    sqlx::query!(
        "UPDATE postgres_trigger SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating mqtt_trigger table");
    sqlx::query!(
        "UPDATE mqtt_trigger SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating gcp_trigger table");
    sqlx::query!(
        "UPDATE gcp_trigger SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating sqs_trigger table");
    sqlx::query!(
        "UPDATE sqs_trigger SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating email_trigger table");
    sqlx::query!(
        "UPDATE email_trigger SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating native_trigger table");
    sqlx::query!(
        "UPDATE native_trigger SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating dependency_map table");
    sqlx::query!(
        "UPDATE dependency_map SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating deployment_metadata table");
    sqlx::query!(
        "UPDATE deployment_metadata SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating draft table");
    sqlx::query!(
        "UPDATE draft SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating favorite table");
    sqlx::query!(
        "UPDATE favorite SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    // Duplicate flow table rows (FK constraint requires insert then delete)
    info!("Duplicating flow table rows");
    sqlx::query!(
        "INSERT INTO flow
            (workspace_id, path, summary, description, archived, extra_perms, dependency_job, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at, lock_error_logs)
        SELECT $1, path, summary, description, archived, extra_perms, dependency_job, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at, lock_error_logs
            FROM flow WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating flow_version table");
    sqlx::query!(
        "UPDATE flow_version SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating workspace_runnable_dependencies table");
    sqlx::query!(
        "UPDATE workspace_runnable_dependencies SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating workspace_dependencies table");
    sqlx::query!(
        "UPDATE workspace_dependencies SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating workspace_diff table");
    sqlx::query!(
        "UPDATE workspace_diff SET source_workspace_id = $1 WHERE source_workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE workspace_diff SET fork_workspace_id = $1 WHERE fork_workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating workspace_fork_deployment_request table");
    sqlx::query!(
        "UPDATE workspace_fork_deployment_request SET source_workspace_id = $1 WHERE source_workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE workspace_fork_deployment_request SET fork_workspace_id = $1 WHERE fork_workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    // Re-parent child forks: any fork whose parent_workspace_id was the old id
    // must follow the renamed parent to the new id, otherwise it is left
    // pointing at the soft-deleted old shell (whose data has moved here).
    info!("Re-parenting child forks to the new workspace id");
    let reparented_children: Vec<String> = sqlx::query_scalar!(
        "UPDATE workspace SET parent_workspace_id = $1 WHERE parent_workspace_id = $2 RETURNING id",
        &rw.new_id,
        &old_id
    )
    .fetch_all(&mut *tx)
    .await?;

    // A dev/fork's `deploy_to` points at the prod root, so it must follow the rename too — otherwise
    // the child re-parents to the new id but still deploys to the soft-deleted old shell.
    sqlx::query!(
        "UPDATE workspace_settings SET deploy_to = $1 WHERE deploy_to = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating workspace_protection_rule table");
    sqlx::query!(
        "UPDATE workspace_protection_rule SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating workspace_integrations table");
    sqlx::query!(
        "UPDATE workspace_integrations SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating ai_agent_memory table");
    sqlx::query!(
        "UPDATE ai_agent_memory SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating flow_conversation table");
    sqlx::query!(
        "UPDATE flow_conversation SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating mcp_oauth_refresh_token table");
    sqlx::query!(
        "UPDATE mcp_oauth_refresh_token SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating mcp_oauth_server_code table");
    sqlx::query!(
        "UPDATE mcp_oauth_server_code SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Duplicating skip_workspace_diff_tally table");
    sqlx::query!(
        "INSERT INTO skip_workspace_diff_tally SELECT $1, added_at FROM skip_workspace_diff_tally WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating asset table");
    sqlx::query!(
        "UPDATE asset SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating flow_node table");
    sqlx::query!(
        "UPDATE flow_node SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Deleting old flow rows");
    sqlx::query!("DELETE FROM flow WHERE workspace_id = $1", &old_id)
        .execute(&mut *tx)
        .await?;

    // Duplicate group_ with new workspace id (FK constraint)
    info!("Duplicating group_ table rows");
    sqlx::query!(
        "INSERT INTO group_ SELECT $1, name, summary, extra_perms FROM group_ WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating usr_to_group table");
    sqlx::query!(
        "UPDATE usr_to_group SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating group_permission_history table");
    sqlx::query!(
        "UPDATE group_permission_history SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Deleting old group_ rows");
    sqlx::query!("DELETE FROM group_ WHERE workspace_id = $1", &old_id)
        .execute(&mut *tx)
        .await?;

    // Duplicate folders with new workspace id (FK constraint)
    info!("Duplicating folder table rows");
    sqlx::query!(
        "INSERT INTO folder (name, workspace_id, display_name, owners, extra_perms, summary, edited_at, created_by, default_permissioned_as, labels) \
         SELECT name, $1, display_name, owners, extra_perms, summary, edited_at, created_by, default_permissioned_as, labels \
         FROM folder WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating folder_permission_history table");
    sqlx::query!(
        "UPDATE folder_permission_history SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Deleting old folder rows");
    sqlx::query!("DELETE FROM folder WHERE workspace_id = $1", &old_id)
        .execute(&mut *tx)
        .await?;

    info!("Updating input table");
    sqlx::query!(
        "UPDATE input SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    // Get enabled schedules and clear their queued jobs BEFORE moving jobs
    // This way we don't need to filter out scheduled jobs - they're already removed
    let enabled_schedule_paths: Vec<String> = sqlx::query_scalar!(
        "SELECT path FROM schedule WHERE workspace_id = $1 AND enabled = true",
        &old_id
    )
    .fetch_all(&mut *tx)
    .await?;

    info!(
        "Found {} enabled schedules, clearing their queued jobs",
        enabled_schedule_paths.len()
    );

    for schedule_path in &enabled_schedule_paths {
        windmill_queue::schedule::clear_schedule(&mut tx, schedule_path, &old_id).await?;
    }

    // Move queued jobs (not running) to new workspace using skip lock
    // Scheduled jobs were already cleared above, so no need to filter them
    info!("Moving v2_job_queue entries to new workspace");
    sqlx::query!(
        "UPDATE v2_job_queue SET workspace_id = $1
         WHERE id IN (
             SELECT id FROM v2_job_queue
             WHERE workspace_id = $2
             AND running = false
             AND id IN (SELECT id FROM v2_job WHERE workspace_id = $2 AND parent_job IS NULL)
             FOR UPDATE SKIP LOCKED
         )",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating v2_job table for moved queue entries");
    sqlx::query!(
        "UPDATE v2_job SET workspace_id = $1
         WHERE workspace_id = $2
         AND id IN (SELECT id FROM v2_job_queue WHERE workspace_id = $1)",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating job_perms table for migrated jobs");
    sqlx::query!(
        "UPDATE job_perms SET workspace_id = $1
         WHERE job_id IN (SELECT id FROM v2_job WHERE workspace_id = $1)",
        &rw.new_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating raw_app table");
    sqlx::query!(
        "UPDATE raw_app SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Deleting raw_script_temp table");
    sqlx::query!(
        "DELETE FROM raw_script_temp WHERE workspace_id = $1",
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating resource table");
    sqlx::query!(
        "UPDATE resource SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating resource_type table");
    sqlx::query!(
        "UPDATE resource_type SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating schedule table");
    sqlx::query!(
        "UPDATE schedule SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating script table");
    sqlx::query!(
        "UPDATE script SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating token table");
    sqlx::query!(
        "UPDATE token SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating usage table");
    sqlx::query!(
        "UPDATE usage SET id = $1 WHERE id = $2 AND is_workspace = true",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Duplicating usr table");
    sqlx::query!(
        "INSERT INTO usr SELECT $1, username, email, is_admin, created_at, operator, disabled, role FROM usr WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating variable table");
    sqlx::query!(
        "UPDATE variable SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    // Re-push jobs for enabled schedules in the new workspace (same transaction)
    info!(
        "Re-pushing jobs for {} enabled schedules",
        enabled_schedule_paths.len()
    );
    for schedule_path in &enabled_schedule_paths {
        if let Some(schedule) = get_schedule_opt(&mut *tx, &rw.new_id, schedule_path).await? {
            tx = push_scheduled_job(&db, tx, &schedule, None, None).await?;
        }
    }

    // Audit log in the same transaction as the workspace changes
    audit_log(
        &mut *tx,
        &authed,
        "workspace.change_workspace_id",
        ActionKind::Update,
        &rw.new_id,
        Some(&authed.email),
        Some(
            [("old_workspace_id", old_id.as_str())]
                .into_iter()
                .collect::<HashMap<&str, &str>>(),
        ),
    )
    .await?;

    tx.commit().await?;

    // The children's parent_workspace_id changed (old root -> new root); invalidate their fork-parent
    // routing cache and their billing-workspace mapping so jobs route + meter under the renamed root
    // rather than the old (archived) one, instead of waiting for the caches' TTLs. Deeper descendants
    // (fork-of-fork) self-heal via the 60s billing-cache TTL.
    for child in &reparented_children {
        windmill_queue::tags::invalidate_fork_parent_cache(child);
        #[cfg(feature = "cloud")]
        windmill_common::workspaces::invalidate_billing_workspace_cache(child);
    }

    // Archive old workspace: disable schedules, cancel remaining jobs, set deleted=true
    // Note: schedules were already moved to new workspace, so this will find 0 schedules
    info!("Archiving old workspace");
    let (_schedules_count, canceled_count, _deleted_tokens_count) =
        archive_workspace_impl(&db, &old_id, &authed.username, None).await?;

    info!(
        "Workspace id change completed: moved {} to {}, archived old workspace",
        old_id, rw.new_id
    );

    Ok(format!(
        "Moved workspace from {} to {}, archived old workspace (canceled {} remaining jobs)",
        &old_id, &rw.new_id, canceled_count
    ))
}

#[derive(Deserialize)]
pub(crate) struct DeleteWorkspaceQuery {
    pub(crate) only_delete_forks: Option<bool>,
}

pub(crate) async fn delete_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
    Query(dwq): Query<DeleteWorkspaceQuery>,
) -> Result<String> {
    let w_id = match w_id.as_str() {
        "starter" => Err(Error::BadRequest(
            "starter workspace cannot be deleted".to_string(),
        )),
        "admins" => Err(Error::BadRequest(
            "admins workspace cannot be deleted".to_string(),
        )),
        _ => Ok(w_id),
    }?;

    let is_fork = workspace_is_fork(&db, &w_id).await?;
    if dwq.only_delete_forks.unwrap_or(false) && !is_fork {
        return Err(Error::BadRequest(
            "Cannot delete this workspace because it is not a workspace fork.".to_string(),
        ));
    }

    let mut tx = db.begin().await?;
    if !(is_fork && is_workspace_owner(&authed, &w_id, &mut tx).await?)
        && !is_super_admin_email(&db, &authed.email).await?
    {
        return Err(Error::PermissionDenied(
            "Deleting this workspace requires being the fork's owner or a superadmin".to_string(),
        ));
    }

    // Don't hard-delete a workspace that still has a dev workspace paired to it: the FK is
    // ON DELETE SET NULL, which would orphan the (prefix-less) dev into a parentless, non-fork row
    // its owner could no longer self-delete. Require detaching/deleting the dev first. Ordinary
    // forks have no such guard — they keep their prefix and stay owner-deletable when orphaned.
    // Archived devs (deleted = true) are included: they keep is_dev_workspace = true, so SET NULL on
    // their parent would violate the `is_dev ⇒ has parent` CHECK and fail the whole delete with a 500.
    if let Some(dev_id) = sqlx::query_scalar!(
        "SELECT id FROM workspace WHERE parent_workspace_id = $1 AND is_dev_workspace",
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    {
        return Err(Error::BadRequest(format!(
            "Cannot delete workspace '{}' because it has a dev workspace ('{}'). Detach or delete the dev workspace first.",
            w_id, dev_id
        )));
    }

    // Deleting an attached dev workspace removes the parent prod's dev_workspace_lock (below), so it
    // must be a prod-admin action, not just the dev's own owner (dev ownership can diverge from
    // prod's) — mirrors detach_dev_workspace, which is prod-admin gated.
    if let Some(prod) = sqlx::query_scalar!(
        "SELECT parent_workspace_id FROM workspace WHERE id = $1 AND is_dev_workspace",
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten()
    {
        let is_prod_admin = sqlx::query_scalar!(
            "SELECT is_admin FROM usr WHERE workspace_id = $1 AND email = $2",
            &prod,
            &authed.email
        )
        .fetch_optional(&mut *tx)
        .await?
        .unwrap_or(false);
        if !is_prod_admin && !is_super_admin_email(&db, &authed.email).await? {
            return Err(Error::PermissionDenied(format!(
                "Deleting dev workspace '{w_id}' requires being an admin of its parent prod workspace '{prod}' (or a superadmin)"
            )));
        }
    }

    sqlx::query!("DELETE FROM ai_agent_memory WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!(
        "DELETE FROM flow_conversation WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM workspace_env WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM dependency_map WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM v2_job_queue WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    // dispatch_event / flow_conversation_message / zombie_job_counter no longer cascade from
    // v2_job (see migration drop_v2_job_side_table_cascades); delete them before v2_job so the
    // workspace's jobs leave no orphan side rows. One round-trip, scanning v2_job once.
    sqlx::query!(
        "WITH ids AS (SELECT id FROM v2_job WHERE workspace_id = $1),
              _de AS (DELETE FROM dispatch_event WHERE workspace_id = $1),
              _fc AS (DELETE FROM flow_conversation_message WHERE job_id IN (SELECT id FROM ids))
         DELETE FROM zombie_job_counter WHERE job_id IN (SELECT id FROM ids)",
        &w_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!("DELETE FROM v2_job WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM capture WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    // capture_config has on delete cascade

    sqlx::query!("DELETE FROM draft WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM script WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM flow WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM app WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM raw_app WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM input WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM variable WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM resource WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM schedule WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!(
        "DELETE FROM v2_job_completed WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM job_stats WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!(
        "DELETE FROM deployment_metadata WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM usr WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM resource_type WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!(
        "DELETE FROM workspace_invite WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM usr_to_group WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM group_ WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM folder WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM account WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM workspace_key WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!(
        "DELETE FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM token WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!("DELETE FROM http_trigger WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!(
        "DELETE FROM websocket_trigger WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM kafka_trigger WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    // NATS triggers have on delete cascade

    sqlx::query!("DELETE FROM raw_script_temp WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    // workspace_diff and skip_workspace_diff_tally are keyed by workspace id with no
    // FK cascade. A fork id is reused when a fork is deleted and recreated under the
    // same name, so leaving these rows behind leaks the previous fork's cached diff
    // verdicts onto the new fork — causing a spurious "changes not visible" warning
    // that hides the deploy button.
    sqlx::query!(
        "DELETE FROM workspace_diff WHERE source_workspace_id = $1 OR fork_workspace_id = $1",
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM skip_workspace_diff_tally WHERE workspace_id = $1",
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    // If this workspace is itself a dev workspace, deleting it dissolves the pairing, so also drop
    // the parent prod's reserved dev_workspace_lock (mirrors detach_dev_workspace) — otherwise prod
    // stays locked against direct deploy/forking with no dev workspace left to make changes in.
    let dev_lock_parent: Option<String> = sqlx::query_scalar!(
        "SELECT parent_workspace_id FROM workspace WHERE id = $1 AND is_dev_workspace",
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();

    // Capture direct child forks before the delete: the FK is ON DELETE SET NULL, so they're about to
    // be orphaned (their billing root changes from this workspace's root to themselves). We drop their
    // cached mappings after commit alongside the deleted id itself.
    let orphaned_children: Vec<String> = sqlx::query_scalar!(
        "SELECT id FROM workspace WHERE parent_workspace_id = $1",
        &w_id
    )
    .fetch_all(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM workspace WHERE id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    if let Some(ref parent) = dev_lock_parent {
        sqlx::query!(
            "DELETE FROM workspace_protection_rule WHERE workspace_id = $1 AND name = $2",
            parent,
            DEV_WORKSPACE_LOCK_RULE_NAME
        )
        .execute(&mut *tx)
        .await?;
    }

    // Record under the instance-level "admins" workspace. The per-workspace audit
    // rows are deleted along with the workspace, so this instance-level entry is the
    // only durable, superadmin-discoverable record of who deleted the workspace.
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.delete",
        ActionKind::Delete,
        "admins",
        Some(&w_id),
        None,
    )
    .await?;
    tx.commit().await?;

    if let Some(parent) = dev_lock_parent {
        windmill_common::workspaces::invalidate_protection_rules_cache(&parent);
    }

    // Fork ids are reusable after permanent deletion, so drop the cached fork->parent (tag routing)
    // and fork->root (billing) mappings for the deleted id and any just-orphaned children — otherwise a
    // recreated id under a different root, or an orphaned child, could resolve to the gone parent/root
    // within the caches' TTLs. Deeper (grandchild) descendants self-heal via the 60s billing TTL.
    for id in std::iter::once(&w_id).chain(orphaned_children.iter()) {
        windmill_queue::tags::invalidate_fork_parent_cache(id);
        #[cfg(feature = "cloud")]
        windmill_common::workspaces::invalidate_billing_workspace_cache(id);
    }

    Ok(format!("Deleted workspace {}", &w_id))
}

#[derive(Deserialize)]
pub struct DropForkedDatatableDatabasesRequest {
    datatable_names: Vec<String>,
}

/// Drop forked datatable databases. Returns errors per datatable that failed.
/// Same permission as delete_workspace: fork owner or super admin.
pub async fn drop_forked_datatable_databases(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<DropForkedDatatableDatabasesRequest>,
) -> Result<Json<Vec<String>>> {
    // Same permission check as delete_workspace: fork owner or super admin
    let is_fork = workspace_is_fork(&db, &w_id).await?;
    let mut tx = db.begin().await?;
    if !(is_fork && is_workspace_owner(&authed, &w_id, &mut tx).await?)
        && !is_super_admin_email(&db, &authed.email).await?
    {
        return Err(Error::PermissionDenied(
            "Dropping forked datatable databases requires being the fork's owner or a superadmin"
                .to_string(),
        ));
    }
    tx.commit().await?;

    let parent_w_id = sqlx::query_scalar!(
        "SELECT parent_workspace_id FROM workspace WHERE id = $1",
        &w_id
    )
    .fetch_optional(&db)
    .await?
    .flatten()
    .ok_or_else(|| Error::BadRequest("No parent workspace found".to_string()))?;

    let datatable_config = sqlx::query_scalar!(
        "SELECT datatable->'datatables' FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_optional(&db)
    .await?
    .flatten()
    .unwrap_or(serde_json::json!({}));

    let datatables: HashMap<String, DataTable> =
        serde_json::from_value(datatable_config).unwrap_or_default();

    let mut errors: Vec<String> = Vec::new();

    for dt_name in &req.datatable_names {
        let dt = match datatables.get(dt_name) {
            Some(dt) if dt.forked_from.is_some() => dt,
            _ => continue,
        };

        if dt.database.resource_type
            == windmill_common::workspaces::DataTableCatalogResourceType::Instance
        {
            let db_to_drop = &dt.database.resource_path;
            if !db_to_drop.starts_with("wm_fork_") {
                errors.push(format!(
                    "Refusing to drop instance database '{}' for datatable://{}:  name does not start with 'wm_fork_'",
                    db_to_drop, dt_name
                ));
                continue;
            }
            if let Err(e) = windmill_common::drop_custom_instance_database(&db, db_to_drop).await {
                errors.push(format!(
                    "Could not drop instance database '{}' for datatable://{}: {}",
                    db_to_drop, dt_name, e
                ));
            }
        } else {
            let fork_pg = match crate::workspaces::resolve_pg_source_checked(
                &db,
                &user_db,
                &authed,
                &w_id,
                &format!("datatable://{}", dt_name),
            )
            .await
            {
                Ok(pg) => pg,
                Err(e) => {
                    errors.push(format!(
                        "Could not resolve fork resource for datatable://{}: {}",
                        dt_name, e
                    ));
                    continue;
                }
            };
            // We cannot drop the current database, so we connect to the parent's version to run DROP DATABASE on
            // the forked version
            let parent_pg = match crate::workspaces::resolve_pg_source_checked(
                &db,
                &user_db,
                &authed,
                &parent_w_id,
                &format!("datatable://{}", dt_name),
            )
            .await
            {
                Ok(pg) => pg,
                Err(e) => {
                    errors.push(format!(
                        "Could not resolve parent resource for datatable://{}: {}",
                        dt_name, e
                    ));
                    continue;
                }
            };

            let db_to_drop = &fork_pg.dbname;
            if let Err(e) = windmill_common::validate_dbname(db_to_drop) {
                errors.push(format!(
                    "Invalid database name '{}' for datatable://{}: {}",
                    db_to_drop, dt_name, e
                ));
                continue;
            }
            if !db_to_drop.starts_with("wm_fork_") {
                errors.push(format!(
                    "Refusing to drop resource database '{}' for datatable://{}: name does not start with 'wm_fork_'",
                    db_to_drop, dt_name
                ));
                continue;
            }

            match parent_pg.connect(Some(&db)).await {
                Ok((client, connection)) => {
                    let join_handle = tokio::spawn(async move { connection.await });
                    if let Err(e) = client
                        .execute(&format!("DROP DATABASE \"{}\"", db_to_drop), &[])
                        .await
                    {
                        errors.push(format!(
                            "Could not drop database '{}' for datatable://{}: {}",
                            db_to_drop, dt_name, e
                        ));
                    }
                    drop(client);
                    let _ = join_handle.await;
                }
                Err(e) => {
                    errors.push(format!(
                        "Could not connect to drop database for datatable://{}: {}",
                        dt_name, e
                    ));
                }
            }
        }
    }

    Ok(Json(errors))
}

async fn is_workspace_owner(
    authed: &ApiAuthed,
    w_id: &str,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<bool> {
    let owner = sqlx::query_scalar!("SELECT owner FROM workspace WHERE id = $1", w_id)
        .fetch_optional(&mut **tx)
        .await?;
    Ok(owner.map(|o| o == authed.email).unwrap_or(false))
}

/// Whether a workspace is a fork or dev workspace. Both forks and dev workspaces set
/// `parent_workspace_id`, but a `wm-fork-` workspace can outlive its parent (the FK is
/// `ON DELETE SET NULL`), so also treat the prefix as fork-ness — otherwise an orphaned fork would
/// lose owner-self-delete. Used to gate owner-self-delete, which is permitted for forks/dev
/// workspaces but requires superadmin otherwise.
async fn workspace_is_fork(db: &DB, w_id: &str) -> Result<bool> {
    if w_id.starts_with(WM_FORK_PREFIX) {
        return Ok(true);
    }
    Ok(sqlx::query_scalar!(
        r#"SELECT (parent_workspace_id IS NOT NULL) AS "has_parent!" FROM workspace WHERE id = $1"#,
        w_id
    )
    .fetch_optional(db)
    .await?
    .unwrap_or(false))
}
