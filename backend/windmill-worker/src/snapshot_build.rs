use serde_json::value::RawValue;
use sqlx::Pool;
use sqlx::Postgres;
use windmill_common::worker::Connection;
use windmill_queue::{append_logs, empty_result, MiniPulledJob};

pub async fn handle_snapshot_build(
    job: &MiniPulledJob,
    db: &Pool<Postgres>,
    conn: &Connection,
) -> windmill_common::error::Result<Box<RawValue>> {
    let args = job
        .args
        .as_ref()
        .ok_or_else(|| windmill_common::error::Error::InternalErr("Missing args".to_string()))?;

    let snapshot_name = args
        .get("snapshot_name")
        .and_then(|v| serde_json::from_str::<String>(v.get()).ok())
        .ok_or_else(|| {
            windmill_common::error::Error::InternalErr("Missing snapshot_name arg".to_string())
        })?;

    let snapshot_tag = args
        .get("snapshot_tag")
        .and_then(|v| serde_json::from_str::<String>(v.get()).ok())
        .ok_or_else(|| {
            windmill_common::error::Error::InternalErr("Missing snapshot_tag arg".to_string())
        })?;

    let docker_image = args
        .get("docker_image")
        .and_then(|v| serde_json::from_str::<String>(v.get()).ok())
        .ok_or_else(|| {
            windmill_common::error::Error::InternalErr("Missing docker_image arg".to_string())
        })?;

    let setup_script: Option<String> = args
        .get("setup_script")
        .and_then(|v| serde_json::from_str(v.get()).ok());

    let include_wmill: bool = args
        .get("include_wmill")
        .and_then(|v| serde_json::from_str(v.get()).ok())
        .unwrap_or(false);

    let agent_binary: Option<String> = args
        .get("agent_binary")
        .and_then(|v| serde_json::from_str(v.get()).ok());

    append_logs(
        &job.id,
        job.workspace_id.clone(),
        format!(
            "Starting snapshot build {snapshot_name}:{snapshot_tag} from image {docker_image}\n"
        ),
        conn,
    )
    .await;

    let result = windmill_sandbox::build_snapshot(
        &job.workspace_id,
        &snapshot_name,
        &snapshot_tag,
        &docker_image,
        setup_script.as_deref(),
        include_wmill,
        agent_binary.as_deref(),
        db,
    )
    .await;

    match &result {
        Ok(()) => {
            append_logs(
                &job.id,
                job.workspace_id.clone(),
                format!("Snapshot {snapshot_name}:{snapshot_tag} built successfully\n"),
                conn,
            )
            .await;
        }
        Err(e) => {
            append_logs(
                &job.id,
                job.workspace_id.clone(),
                format!("Snapshot build failed: {e}\n"),
                conn,
            )
            .await;
        }
    }

    result?;
    Ok(empty_result())
}
