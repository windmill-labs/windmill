pub async fn run_flow_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    axum::Json(args): axum::Json<Option<Map<String, Value>>>,
    Query(run_query): Query<RunJobQuery>,
) -> error::Result<(StatusCode, String)> {
    let flow_path = flow_path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    let scheduled_for = run_query.get_scheduled_for(&mut tx).await?;
    let (uuid, tx) = push(
        tx,
        &w_id,
        JobPayload::Flow(flow_path.to_string()),
        args,
        &authed.username,
        owner_to_token_owner(&authed.username, false),
        scheduled_for,
        None,
        run_query.parent_job,
        false,
        false,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

pub async fn run_job_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    axum::Json(args): axum::Json<Option<Map<String, Value>>>,
    Query(run_query): Query<RunJobQuery>,
) -> error::Result<(StatusCode, String)> {
    let script_path = script_path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    let job_payload = script_path_to_payload(script_path, &mut tx, &w_id).await?;
    let scheduled_for = run_query.get_scheduled_for(&mut tx).await?;

    let (uuid, tx) = push(
        tx,
        &w_id,
        job_payload,
        args,
        &authed.username,
        owner_to_token_owner(&authed.username, false),
        scheduled_for,
        None,
        run_query.parent_job,
        false,
        false,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

async fn run_wait_result<T>(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    uuid: Uuid,
    Path((w_id, _)): Path<(String, T)>,
) -> error::JsonResult<serde_json::Value> {
    let mut result = None;
    for i in 0..48 {
        let mut tx = user_db.clone().begin(&authed).await?;

        result = sqlx::query_scalar!(
            "SELECT result FROM completed_job WHERE id = $1 AND workspace_id = $2",
            uuid,
            &w_id
        )
        .fetch_optional(&mut tx)
        .await?
        .flatten();

        if result.is_some() {
            break;
        }
        let delay = if i < 10 { 100 } else { 500 };
        tokio::time::sleep(core::time::Duration::from_millis(delay)).await;
    }
    if let Some(result) = result {
        Ok(Json(result))
    } else {
        Err(Error::ExecutionErr("timeout after 20s".to_string()))
    }
}

pub async fn run_wait_result_job_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    axum::Json(args): axum::Json<Option<Map<String, Value>>>,
    Query(run_query): Query<RunJobQuery>,
) -> error::JsonResult<serde_json::Value> {
    let script_path = script_path.to_path();
    let mut tx = user_db.clone().begin(&authed).await?;
    let job_payload = script_path_to_payload(script_path, &mut tx, &w_id).await?;
    let scheduled_for = run_query.get_scheduled_for(&mut tx).await?;

    let (uuid, tx) = push(
        tx,
        &w_id,
        job_payload,
        args,
        &authed.username,
        owner_to_token_owner(&authed.username, false),
        scheduled_for,
        None,
        run_query.parent_job,
        false,
        false,
    )
    .await?;
    tx.commit().await?;

    run_wait_result(authed, Extension(user_db), uuid, Path((w_id, script_path))).await
}

pub async fn run_wait_result_job_by_hash(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_hash)): Path<(String, ScriptHash)>,
    axum::Json(args): axum::Json<Option<Map<String, Value>>>,
    Query(run_query): Query<RunJobQuery>,
) -> error::JsonResult<serde_json::Value> {
    let hash = script_hash.0;
    let mut tx = user_db.clone().begin(&authed).await?;
    let path = get_path_for_hash(&mut tx, &w_id, hash).await?;
    let scheduled_for = run_query.get_scheduled_for(&mut tx).await?;

    let (uuid, tx) = push(
        tx,
        &w_id,
        JobPayload::ScriptHash { hash: ScriptHash(hash), path },
        args,
        &authed.username,
        owner_to_token_owner(&authed.username, false),
        scheduled_for,
        None,
        run_query.parent_job,
        false,
        false,
    )
    .await?;
    tx.commit().await?;

    run_wait_result(authed, Extension(user_db), uuid, Path((w_id, script_hash))).await
}

// a similar function exists on the worker
pub async fn script_path_to_payload<'c>(
    script_path: &str,
    db: &mut Transaction<'c, Postgres>,
    w_id: &String,
) -> Result<JobPayload, Error> {
    let job_payload = if script_path.starts_with("hub/") {
        JobPayload::ScriptHub { path: script_path.to_owned() }
    } else {
        let script_hash = get_latest_hash_for_path(db, w_id, script_path).await?;
        JobPayload::ScriptHash { hash: script_hash, path: script_path.to_owned() }
    };
    Ok(job_payload)
}

async fn run_preview_job(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(preview): Json<Preview>,
    Query(sch_query): Query<RunJobQuery>,
) -> error::Result<(StatusCode, String)> {
    let mut tx = user_db.begin(&authed).await?;
    let scheduled_for = sch_query.get_scheduled_for(&mut tx).await?;

    let (uuid, tx) = push(
        tx,
        &w_id,
        JobPayload::Code(RawCode {
            content: preview.content,
            path: preview.path,
            language: preview.language,
        }),
        preview.args,
        &authed.username,
        owner_to_token_owner(&authed.username, false),
        scheduled_for,
        None,
        None,
        false,
        false,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

async fn run_preview_flow_job(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(raw_flow): Json<PreviewFlow>,
    Query(sch_query): Query<RunJobQuery>,
) -> error::Result<(StatusCode, String)> {
    let mut tx = user_db.begin(&authed).await?;
    let scheduled_for = sch_query.get_scheduled_for(&mut tx).await?;
    let (uuid, tx) = push(
        tx,
        &w_id,
        JobPayload::RawFlow { value: raw_flow.value, path: raw_flow.path },
        raw_flow.args,
        &authed.username,
        owner_to_token_owner(&authed.username, false),
        scheduled_for,
        None,
        None,
        false,
        false,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

pub async fn run_job_by_hash(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_hash)): Path<(String, ScriptHash)>,
    axum::Json(args): axum::Json<Option<Map<String, Value>>>,
    Query(run_query): Query<RunJobQuery>,
) -> error::Result<(StatusCode, String)> {
    let hash = script_hash.0;
    let mut tx = user_db.begin(&authed).await?;
    let path = get_path_for_hash(&mut tx, &w_id, hash).await?;
    let scheduled_for = run_query.get_scheduled_for(&mut tx).await?;

    let (uuid, tx) = push(
        tx,
        &w_id,
        JobPayload::ScriptHash { hash: ScriptHash(hash), path },
        args,
        &authed.username,
        owner_to_token_owner(&authed.username, false),
        scheduled_for,
        None,
        run_query.parent_job,
        false,
        false,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}
