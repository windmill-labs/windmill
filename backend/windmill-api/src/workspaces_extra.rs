use crate::db::ApiAuthed;

use crate::workspaces::{check_w_id_conflict, CREATE_WORKSPACE_REQUIRE_SUPERADMIN};
use crate::{db::DB, utils::require_super_admin};

use axum::{
    extract::{Extension, Path},
    Json,
};

use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;

use windmill_common::worker::CLOUD_HOSTED;

use windmill_common::{
    auth::is_super_admin_email,
    error::{Error, Result},
    utils::require_admin,
};

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

    // duplicate workspace with new id name
    sqlx::query!(
        "INSERT INTO workspace SELECT $1, $2, owner, deleted, premium FROM workspace WHERE id = $3",
        &rw.new_id,
        &rw.new_name,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE account SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE app SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE audit SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE capture SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE capture_config SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE http_trigger SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE websocket_trigger SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE kafka_trigger SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE nats_trigger SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE v2_job_completed SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE dependency_map SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE deployment_metadata SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE draft SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE favorite SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO flow 
            (workspace_id, path, summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at) 
        SELECT $1, path, summary, description, archived, extra_perms, dependency_job, draft_only, tag, ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only, on_behalf_of_email, concurrency_key, versions, value, schema, edited_by, edited_at
            FROM flow WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE flow_version SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE workspace_runnable_dependencies SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE flow_node SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM flow WHERE workspace_id = $1", &old_id)
        .execute(&mut *tx)
        .await?;

    // have to duplicate group_ with new workspace id because of foreign key constraint
    sqlx::query!(
        "INSERT INTO group_ SELECT $1, name, summary, extra_perms FROM group_ WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE usr_to_group SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    // then delete old group_
    sqlx::query!("DELETE FROM group_ WHERE workspace_id = $1", &old_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query!(
        "UPDATE folder SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE input SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE job_logs SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE job_stats SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE v2_job_queue SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE v2_job SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE raw_app SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE resource SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE resource_type SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE schedule SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE script SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE token SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE usage SET id = $1 WHERE id = $2 AND is_workspace = true",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE usr SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE variable SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE workspace_env SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE workspace_invite SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE workspace_key SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE workspace_settings SET workspace_id = $1 WHERE workspace_id = $2",
        &rw.new_id,
        &old_id
    )
    .execute(&mut *tx)
    .await?;

    // delete old workspace
    sqlx::query!("DELETE FROM workspace WHERE id = $1", &old_id)
        .execute(&mut *tx)
        .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspace.change_workspace_id",
        ActionKind::Update,
        &rw.new_id,
        Some(&authed.email),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!(
        "updated workspace from {} to {}",
        &old_id, &rw.new_id
    ))
}

pub(crate) async fn delete_workspace(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    authed: ApiAuthed,
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
    let mut tx = db.begin().await?;
    require_super_admin(&db, &authed.email).await?;

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
