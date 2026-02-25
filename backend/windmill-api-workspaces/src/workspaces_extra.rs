use std::collections::HashMap;

use windmill_api_auth::{require_super_admin, ApiAuthed};
use windmill_common::DB;

use crate::workspaces::{
    archive_workspace_impl, check_w_id_conflict, CREATE_WORKSPACE_REQUIRE_SUPERADMIN,
    WM_FORK_PREFIX,
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
    error::{Error, Result},
    utils::require_admin,
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

    // Create new workspace with new id and name
    info!("Creating new workspace row");
    sqlx::query!(
        "INSERT INTO workspace SELECT $1, $2, owner, false, premium FROM workspace WHERE id = $3",
        &rw.new_id,
        &rw.new_name,
        &old_id
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
            (workspace_id, path, summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at, lock_error_logs)
        SELECT $1, path, summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at, lock_error_logs
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
        "INSERT INTO folder SELECT name, $1, display_name, owners, extra_perms, summary, edited_at, created_by FROM folder WHERE workspace_id = $2",
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

    // Archive old workspace: disable schedules, cancel remaining jobs, set deleted=true
    // Note: schedules were already moved to new workspace, so this will find 0 schedules
    info!("Archiving old workspace");
    let (_schedules_count, canceled_count, _deleted_tokens_count) =
        archive_workspace_impl(&db, &old_id, &authed.username).await?;

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

    if dwq.only_delete_forks.unwrap_or(false) && !w_id.starts_with(WM_FORK_PREFIX) {
        return Err(Error::BadRequest(
            "Cannot delete this workspace because it is not a workspace fork.".to_string(),
        ));
    }

    let mut tx = db.begin().await?;
    if !(w_id.starts_with(WM_FORK_PREFIX) && is_workspace_owner(&authed, &w_id, &mut tx).await?) {
        require_super_admin(&db, &authed.email).await?;
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

    sqlx::query!("DELETE FROM workspace WHERE id = $1", &w_id)
        .execute(&mut *tx)
        .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.delete",
        ActionKind::Delete,
        &w_id,
        Some(&authed.email),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Deleted workspace {}", &w_id))
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
