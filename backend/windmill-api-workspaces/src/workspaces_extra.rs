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

    info!("Updating macro_definition table");
    sqlx::query!(
        "UPDATE macro_definition SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    info!("Updating macro_usage table");
    sqlx::query!(
        "UPDATE macro_usage SET workspace_id = $1 WHERE workspace_id = $2",
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
        windmill_common::workspaces::invalidate_fork_ancestor_chain_cache(child);
        // Grandchildren's cached chains contain the old (renamed-away) ancestor id; a stale
        // chain drops all defer ancestors in the ducklake resolver, so sweep the subtree
        // rather than letting it wait out the TTL.
        for id in windmill_common::workspaces::list_fork_descendants(&db, child)
            .await
            .unwrap_or_default()
        {
            windmill_common::workspaces::invalidate_fork_ancestor_chain_cache(&id);
        }
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

    // Snapshot the fork's ducklake namespaces + RESOLVE their connection material NOW — the
    // registry rows and the fork's `$res:` resources both CASCADE with the workspace row —
    // but the destructive cleanup itself runs only after the commit below: a delete that
    // fails mid-way must never leave a live workspace with its fork data destroyed and no
    // registry row to retry from. Read-only: nothing is dropped here.
    let fork_ducklake_cleanups = prepare_fork_ducklake_cleanups(&db, &w_id)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("deleting workspace {w_id}: preparing ducklake cleanup: {e:#}");
            vec![]
        });

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
    sqlx::query!("DELETE FROM macro_usage WHERE workspace_id = $1", &w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!(
        "DELETE FROM macro_definition WHERE workspace_id = $1",
        &w_id
    )
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

    // Physical ducklake-namespace cleanup, post-commit, from the pre-read snapshot: fork
    // namespaces are deterministic from (id, lake), so an orphan would silently REATTACH to a
    // recreated identical fork id. Runs inline so every delete path is covered (CLI,
    // force-delete dialog, direct API — not just the sidebar flow, which still calls the
    // endpoint first for per-lake error toasts; the rerun is an idempotent no-op). Best
    // effort: failures are logged — the workspace row is already gone, and broken storage
    // credentials must not have made it undeletable.
    for e in cleanup_fork_ducklake_namespaces(&db, &w_id, fork_ducklake_cleanups).await {
        tracing::warn!("deleted workspace {w_id}: ducklake namespace cleanup: {e}");
    }

    if let Some(parent) = dev_lock_parent {
        windmill_common::workspaces::invalidate_protection_rules_cache(&parent);
    }

    // Workspace ids are reusable after permanent deletion, so drop every cached mapping keyed by the
    // deleted id (and any just-orphaned children) — otherwise a recreated id could inherit the gone
    // workspace's state within the caches' lifetimes. This covers fork->parent (tag routing) and
    // fork->root (billing), plus the premium/team-plan status: TEAM_PLAN_CACHE has no TTL and is only
    // evicted by the premium-change NOTIFY, so without this a reused id would keep the old workspace's
    // premium indefinitely (free forks/usage). Deeper (grandchild) descendants self-heal via the 60s
    // billing TTL.
    for id in std::iter::once(&w_id).chain(orphaned_children.iter()) {
        windmill_queue::tags::invalidate_fork_parent_cache(id);
        windmill_common::workspaces::invalidate_fork_ancestor_chain_cache(id);
        #[cfg(feature = "cloud")]
        {
            windmill_common::workspaces::invalidate_billing_workspace_cache(id);
            windmill_common::workspaces::invalidate_team_plan_cache(id);
        }
    }
    // Deeper descendants' cached ancestor CHAINS still contain the deleted workspace; unlike
    // the sibling caches (which self-heal harmlessly via TTL), a stale chain makes the
    // ducklake resolver drop all defer ancestors (all-or-nothing on broken links) — a visible
    // defer/chips outage for up to the TTL. Anchor at the orphaned children: the deleted row
    // is gone, but their subtrees are intact.
    for child in orphaned_children.iter() {
        for id in windmill_common::workspaces::list_fork_descendants(&db, child)
            .await
            .unwrap_or_default()
        {
            windmill_common::workspaces::invalidate_fork_ancestor_chain_cache(&id);
        }
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

/// Drop this fork workspace's ducklake namespaces: the `wm_fork_*` metadata schema in each
/// lake's catalog database, plus (best effort) the fork's `__wm_forks/<wid>/…` data files in
/// the workspace storage. Driven by the `fork_ducklake_namespace` registry written at first
/// fork attach, so it works even after settings drift. Returns errors per lake that failed;
/// the registry row is only deleted once both cleanups succeeded, so a retry resumes.
/// Same permission as delete_workspace: fork owner or super admin. `delete_workspace` also
/// runs this inline (the UI calls this endpoint first for error visibility; the inline run
/// covers every other delete path — CLI, force-delete dialogs, direct API — whose row delete
/// would otherwise CASCADE the registry away while orphaning the physical namespaces, which a
/// recreated identical fork id would then silently reattach).
pub async fn drop_forked_ducklake_namespaces(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> Result<Json<Vec<String>>> {
    let is_fork = workspace_is_fork(&db, &w_id).await?;
    let mut tx = db.begin().await?;
    if !(is_fork && is_workspace_owner(&authed, &w_id, &mut tx).await?)
        && !is_super_admin_email(&db, &authed.email).await?
    {
        return Err(Error::PermissionDenied(
            "Dropping forked ducklake namespaces requires being the fork's owner or a superadmin"
                .to_string(),
        ));
    }
    tx.commit().await?;
    Ok(Json(drop_forked_ducklake_namespaces_impl(&db, &w_id).await?))
}

/// The cleanup itself, shared by the endpoint and the post-commit `delete_workspace` run. No
/// authorization of its own — callers gate.
pub(crate) async fn drop_forked_ducklake_namespaces_impl(
    db: &DB,
    w_id: &str,
) -> Result<Vec<String>> {
    let prepared = prepare_fork_ducklake_cleanups(db, w_id).await?;
    Ok(cleanup_fork_ducklake_namespaces(db, w_id, prepared).await)
}

struct ForkDucklakeNamespaceRow {
    ducklake_name: String,
    metadata_schema: String,
    catalog: String,
    storage: String,
    storage_ref: String,
    data_path: String,
}

/// A namespace row plus its RESOLVED connection material. `delete_workspace` prepares these
/// BEFORE its transaction commits — the registry rows AND the fork's `$res:` resources both
/// disappear with the workspace row — and runs the destructive cleanup only AFTER the commit:
/// a delete that fails mid-way must never leave a live workspace with its fork data
/// destroyed, and a committed delete must still be able to reach catalogs/storages whose
/// credentials lived in the (now gone) fork resources. Per-row resolution failures are
/// carried as strings so they surface with the other cleanup errors.
pub(crate) struct PreparedForkDucklakeCleanup {
    ns: ForkDucklakeNamespaceRow,
    catalog_pg: std::result::Result<windmill_common::PgDatabase, String>,
    store: std::result::Result<(windmill_types::s3::LargeFileStorage, serde_json::Value), String>,
}

/// One row per (lake, catalog, storage, data path) ever attached — settings drift adds rows,
/// so every location the fork wrote gets cleaned, not just the first. Read-only: resolves
/// credentials but destroys nothing.
pub(crate) async fn prepare_fork_ducklake_cleanups(
    db: &DB,
    w_id: &str,
) -> Result<Vec<PreparedForkDucklakeCleanup>> {
    let rows = sqlx::query_as!(
        ForkDucklakeNamespaceRow,
        r#"SELECT ducklake_name AS "ducklake_name!", metadata_schema AS "metadata_schema!",
                  catalog AS "catalog!", storage AS "storage!",
                  storage_ref AS "storage_ref!", data_path AS "data_path!"
             FROM fork_ducklake_namespace WHERE workspace_id = $1"#,
        w_id
    )
    .fetch_all(db)
    .await?;
    let mut prepared = Vec::with_capacity(rows.len());
    for ns in rows {
        let catalog_pg = resolve_fork_catalog_pg(db, w_id, &ns.ducklake_name, &ns.catalog)
            .await
            .map_err(|e| e.to_string());
        let storage = Some(ns.storage.as_str()).filter(|s| !s.is_empty());
        let store = resolve_fork_storage(db, w_id, storage, &ns.storage_ref)
            .await
            .map_err(|e| e.to_string());
        prepared.push(PreparedForkDucklakeCleanup { ns, catalog_pg, store });
    }
    Ok(prepared)
}

/// Drop the prepared namespaces' metadata schemas + data files, deleting each registry row
/// only after both succeed (a no-op when the row already cascaded away with the workspace).
/// Returns per-namespace error strings; never fails as a whole.
pub(crate) async fn cleanup_fork_ducklake_namespaces(
    db: &DB,
    w_id: &str,
    prepared: Vec<PreparedForkDucklakeCleanup>,
) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();
    for PreparedForkDucklakeCleanup { ns, catalog_pg, store } in prepared {
        // Hard guards mirroring the forked-datatable drop: never touch a schema outside the
        // fork prefix, never delete outside the fork data dir — even if a registry row was
        // somehow tampered with.
        if !ns
            .metadata_schema
            .starts_with(windmill_common::workspaces::FORK_DUCKLAKE_SCHEMA_PREFIX)
        {
            errors.push(format!(
                "Refusing to drop schema '{}' for ducklake://{}: name does not start with '{}'",
                ns.metadata_schema,
                ns.ducklake_name,
                windmill_common::workspaces::FORK_DUCKLAKE_SCHEMA_PREFIX
            ));
            continue;
        }
        // The fork's directory segment, NOT the raw workspace id: ids are only
        // git-branch-safe and may contain `/`, which raw would let one fork's prefix nest
        // inside a sibling's (`wm-fork-a/b` under `wm-fork-a`) and be swept by its cleanup.
        let expected_prefix = format!(
            "{}/{}/",
            windmill_common::workspaces::FORK_DUCKLAKE_DATA_DIR,
            windmill_common::workspaces::fork_data_dir_segment(w_id)
        );
        if !format!("{}/", ns.data_path.trim_end_matches('/')).starts_with(&expected_prefix) {
            errors.push(format!(
                "Refusing to delete data path '{}' for ducklake://{}: not under '{}'",
                ns.data_path, ns.ducklake_name, expected_prefix
            ));
            continue;
        }

        let drop_res = match catalog_pg {
            Ok(pg) => drop_fork_ducklake_metadata_schema(db, pg, &ns.metadata_schema).await,
            Err(e) => Err(Error::internal_err(e)),
        };
        if let Err(e) = drop_res {
            errors.push(format!(
                "Could not drop metadata schema '{}' for ducklake://{}: {e}",
                ns.metadata_schema, ns.ducklake_name
            ));
            continue;
        }
        let delete_res = match store {
            Ok((lfs, resource_value)) => {
                delete_fork_ducklake_data(lfs, resource_value, &ns.data_path).await
            }
            Err(e) => Err(Error::internal_err(e)),
        };
        if let Err(e) = delete_res {
            errors.push(format!(
                "Dropped metadata schema but could not delete data files under '{}' for ducklake://{}: {e}",
                ns.data_path, ns.ducklake_name
            ));
            continue;
        }
        sqlx::query!(
            "DELETE FROM fork_ducklake_namespace
             WHERE workspace_id = $1 AND ducklake_name = $2 AND catalog = $3
               AND storage = $4 AND storage_ref = $5 AND data_path = $6",
            w_id,
            &ns.ducklake_name,
            &ns.catalog,
            &ns.storage,
            &ns.storage_ref,
            &ns.data_path,
        )
        .execute(db)
        .await
        .map_err(|e| {
            errors.push(format!(
                "Cleaned ducklake://{} but could not delete its registry row: {e}",
                ns.ducklake_name
            ))
        })
        .ok();
    }
    errors
}

/// Drop the fork's metadata schema in the catalog database recorded by the registry row —
/// NOT whatever the fork's settings point at by now: a drifted catalog resource must not make
/// cleanup drop a schema in the wrong database while orphaning the real one. The pg schema
/// holds only DuckLake metadata tables (auto-created at first fork attach), so a plain
/// `DROP SCHEMA … CASCADE` on the catalog connection removes the whole fork namespace.
/// Resolve the catalog connection recorded in the registry row (read-only): instance
/// identities rebuild instance creds; resource identities resolve their `$res:` in the fork's
/// workspace — which is why this must run BEFORE the workspace (and its resources) are
/// deleted. Mysql never registers (rejected at resolution).
async fn resolve_fork_catalog_pg(
    db: &DB,
    w_id: &str,
    ducklake_name: &str,
    catalog: &str,
) -> Result<windmill_common::PgDatabase> {
    let (resource_type, resource_path) = catalog.split_once(':').ok_or_else(|| {
        Error::internal_err(format!(
            "ducklake://{ducklake_name}: malformed registry catalog identity `{catalog}`"
        ))
    })?;
    let catalog_resource = if resource_type == "instance" {
        let mut pg_creds = windmill_common::PgDatabase::parse_uri(
            &windmill_common::get_database_url().await?.as_str().await,
        )?;
        pg_creds.dbname = resource_path.to_string();
        pg_creds.user = Some("custom_instance_user".to_string());
        pg_creds.password =
            Some(windmill_common::utils::get_custom_pg_instance_password(db).await?);
        serde_json::to_value(&pg_creds)
            .map_err(|e| Error::internal_err(format!("serializing pg creds: {e}")))?
    } else {
        windmill_common::workspaces::transform_json_value_unchecked(
            &serde_json::Value::String(format!("$res:{resource_path}")),
            w_id,
            db,
        )
        .await?
    };
    serde_json::from_value(catalog_resource).map_err(|e| {
        Error::internal_err(format!(
            "ducklake://{ducklake_name}: catalog resource is not a postgres database: {e}"
        ))
    })
}

async fn drop_fork_ducklake_metadata_schema(
    db: &DB,
    pg: windmill_common::PgDatabase,
    metadata_schema: &str,
) -> Result<()> {
    let (client, connection) = pg.connect(Some(db)).await?;
    let join_handle = tokio::spawn(async move { connection.await });
    let res = client
        .execute(
            &format!(
                "DROP SCHEMA IF EXISTS \"{}\" CASCADE",
                metadata_schema.replace('"', "\"\"")
            ),
            &[],
        )
        .await;
    drop(client);
    let _ = join_handle.await;
    res.map_err(|e| Error::internal_err(format!("{e:#}")))?;
    Ok(())
}

/// Resolve the storage identified by the registry's `storage_ref` (read-only) — the storage
/// that was active when the fork's data was WRITTEN, not whatever the logical storage name
/// points at by deletion time (a repointed storage must not orphan the original fork data,
/// nor get a colliding prefix deleted). `storage_ref` = '' falls back to resolving the
/// logical name against current settings (registration couldn't identify the storage — best
/// effort). Resolves the `$res:` in the fork's workspace, which is why this must run BEFORE
/// the workspace is deleted.
async fn resolve_fork_storage(
    db: &DB,
    w_id: &str,
    storage: Option<&str>,
    storage_ref: &str,
) -> Result<(windmill_types::s3::LargeFileStorage, serde_json::Value)> {
    use windmill_types::s3::{
        AzureBlobStorage, FilesystemStorage, GoogleCloudStorage, LargeFileStorage, S3Storage,
    };

    let lfs: LargeFileStorage = if let Some((typ, path)) = storage_ref.split_once(':') {
        // Rebuild the LFS entry from the registered identity; only the variant (which
        // resource parser applies) and the path matter to `lfs_to_object_store_resource`.
        let s3 = |p: &str| S3Storage {
            s3_resource_path: p.to_string(),
            public_resource: None,
            advanced_permissions: None,
        };
        match typ {
            "S3Storage" => LargeFileStorage::S3Storage(s3(path)),
            "S3AwsOidc" => LargeFileStorage::S3AwsOidc(s3(path)),
            "AzureBlobStorage" => LargeFileStorage::AzureBlobStorage(AzureBlobStorage {
                azure_blob_resource_path: path.to_string(),
                public_resource: None,
                advanced_permissions: None,
            }),
            "AzureWorkloadIdentity" => LargeFileStorage::AzureWorkloadIdentity(AzureBlobStorage {
                azure_blob_resource_path: path.to_string(),
                public_resource: None,
                advanced_permissions: None,
            }),
            "GoogleCloudStorage" => LargeFileStorage::GoogleCloudStorage(GoogleCloudStorage {
                gcs_resource_path: path.to_string(),
                public_resource: None,
                advanced_permissions: None,
            }),
            "FilesystemStorage" => LargeFileStorage::FilesystemStorage(FilesystemStorage {
                root_path: path.to_string(),
                public_resource: None,
                advanced_permissions: None,
            }),
            other => {
                return Err(Error::internal_err(format!(
                    "unknown registered storage type `{other}`"
                )))
            }
        }
    } else {
        let lfs_json = sqlx::query_scalar!(
            "SELECT large_file_storage FROM workspace_settings WHERE workspace_id = $1",
            w_id
        )
        .fetch_optional(db)
        .await?
        .flatten()
        .ok_or_else(|| Error::BadRequest("workspace has no storage configured".to_string()))?;
        // Named storages live under `secondary_storage`; `None`/`_default_` is the primary.
        match storage.filter(|s| *s != "_default_") {
            None => serde_json::from_value(lfs_json.clone())
                .map_err(|e| Error::internal_err(format!("parsing large_file_storage: {e}")))?,
            Some(name) => serde_json::from_value(
                lfs_json
                    .get("secondary_storage")
                    .and_then(|s| s.get(name))
                    .cloned()
                    .ok_or_else(|| {
                        Error::BadRequest(format!("workspace has no storage named {name}"))
                    })?,
            )
            .map_err(|e| Error::internal_err(format!("parsing storage {name}: {e}")))?,
        }
    };
    // Filesystem storage stores a direct path (`lfs_to_object_store_resource` ignores the
    // resource value); everything else references a resource whose stored path may or may not
    // carry the `$res:` prefix — same normalization as `get_workspace_s3_resource_from_lfs`.
    let resource_value = if matches!(lfs, LargeFileStorage::FilesystemStorage(_)) {
        serde_json::Value::Null
    } else {
        let path = lfs.get_s3_resource_path();
        let path = path.strip_prefix("$res:").unwrap_or(path);
        windmill_common::workspaces::transform_json_value_unchecked(
            &serde_json::Value::String(format!("$res:{path}")),
            w_id,
            db,
        )
        .await?
    };
    Ok((lfs, resource_value))
}

/// Delete every object under the fork's data prefix in the pre-resolved storage. Requires the
/// `parquet` (object store) feature; without it the metadata schema is still dropped and the
/// unreachable data files are left for manual cleanup.
#[cfg(feature = "parquet")]
async fn delete_fork_ducklake_data(
    lfs: windmill_types::s3::LargeFileStorage,
    resource_value: serde_json::Value,
    data_path: &str,
) -> Result<()> {
    use futures::{StreamExt, TryStreamExt};

    let store = windmill_object_store::build_object_store_client(
        &windmill_object_store::lfs_to_object_store_resource(&lfs, resource_value)?,
    )
    .await?;

    let prefix = windmill_object_store::object_store_reexports::Path::from(
        data_path.trim_matches('/').to_string(),
    );
    let locations: Vec<_> = store
        .list(Some(&prefix))
        .map_ok(|m| m.location)
        .try_collect()
        .await
        .map_err(windmill_object_store::object_store_error_to_error)?;
    // 1000-object chunks: S3 DeleteObjects caps a batch at 1000 keys.
    for chunk in locations.chunks(1000) {
        store
            .delete_stream(futures::stream::iter(chunk.iter().cloned().map(Ok)).boxed())
            .try_collect::<Vec<_>>()
            .await
            .map_err(windmill_object_store::object_store_error_to_error)?;
    }
    Ok(())
}

#[cfg(not(feature = "parquet"))]
async fn delete_fork_ducklake_data(
    _lfs: windmill_types::s3::LargeFileStorage,
    _resource_value: serde_json::Value,
    _data_path: &str,
) -> Result<()> {
    Err(Error::internal_err(
        "object storage support (parquet feature) is not compiled in".to_string(),
    ))
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
