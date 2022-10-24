use axum::Router;
mod run;
use run::*;
mod completed_jobs;
use completed_jobs::*;
mod job_update;
use job_update::*;
use windmill_common::scripts::ScriptHash;

pub fn workspaced_service() -> Router {
    Router::new()
        .route(
            "/run/f/*script_path",
            post(windmill_queue::run_flow_by_path),
        )
        .route("/run/p/*script_path", post(windmill_queue::run_job_by_path))
        .route(
            "/run_wait_result/p/*script_path",
            post(run_wait_result_job_by_path),
        )
        .route(
            "/run_wait_result/h/:hash",
            post(windmill_queue::run_wait_result_job_by_hash),
        )
        .route("/run/h/:hash", post(windmill_queue::run_job_by_hash))
        .route("/run/preview", post(windmill_queue::run_preview_job))
        .route(
            "/run/preview_flow",
            post(windmill_queue::run_preview_flow_job),
        )
        .route("/list", get(windmill_queue::list_jobs))
        .route("/queue/list", get(windmill_queue::list_queue_jobs))
        .route("/queue/cancel/:id", post(cancel_job))
        .route("/completed/list", get(windmill_queue::list_completed_jobs))
        .route("/completed/get/:id", get(windmill_queue::get_completed_job))
        .route(
            "/completed/get_result/:id",
            get(windmill_queue::get_completed_job_result),
        )
        .route(
            "/completed/delete/:id",
            post(windmill_queue::delete_completed_job),
        )
        .route("/get/:id", get(windmill_queue::get_job))
        .route("/getupdate/:id", get(windmill_queue::get_job_update))
        .route(
            "/job_signature/:job_id/:resume_id",
            get(windmill_queue::create_job_signature),
        )
        .route("/result_by_id/:job_id/:node_id", get(get_result_by_id))
        .route("/hash/p/*script_path", get_latest_hash_for_path_api)
}

pub fn global_service() -> Router {
    Router::new()
        .route(
            "/resume/:job_id/:resume_id/:secret",
            get(windmill_queue::resume_suspended_job),
        )
        .route(
            "/resume/:job_id/:resume_id/:secret",
            post(windmill_queue::resume_suspended_job),
        )
        .route(
            "/cancel/:job_id/:resume_id/:secret",
            get(windmill_queue::cancel_suspended_job),
        )
        .route(
            "/cancel/:job_id/:resume_id/:secret",
            post(windmill_queue::cancel_suspended_job),
        )
}

async get_result_by_id(
    Extension(db): Extension<DB>,
    Query(ResultByIdQuery { mut skip_direct }): Query<ResultByIdQuery>,
    Path((w_id, flow_id, node_id)): Path<(String, String, String)>
) -> windmill_common::error::JsonResult<serde_json::Value> {
    let res = windmill_queue::get_result_by_id(db, skip_direct, w_id, flow_id, node_id).await?;
    Ok(Json(res))
}

async fn cancel_job(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Json(CancelJob { reason }): Json<CancelJob>,
) -> error::Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    let res = windmill_queue::cancel_job(tx, authed, w_id, id, reason).await?;
    tx.commit().await?;
    res
}

async fn get_latest_hash_for_path_api(
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
) -> error::JsonResult<ScriptHash> {
    Ok(Json(get_latest_hash_for_path(user_db.begin().await?, w_id, script_path).await?))
}

pub async fn get_latest_hash_for_path<'c>(
    db: &mut Transaction<'c, Postgres>,
    w_id: &str,
    script_path: &str,
) -> error::Result<ScriptHash> {
    let script_hash_o = sqlx::query_scalar!(
        "select hash from script where path = $1 AND (workspace_id = $2 OR workspace_id = \
         'starter') AND
    created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND (workspace_id = $2 OR \
         workspace_id = 'starter')) AND
    deleted = false",
        script_path,
        w_id
    )
    .fetch_optional(db)
    .await?;

    let script_hash = crate::utils::not_found_if_none(script_hash_o, "ScriptHash", script_path)?;

    Ok(ScriptHash(script_hash))
}

pub async fn get_path_for_hash<'c>(
    db: &mut Transaction<'c, Postgres>,
    w_id: &str,
    hash: i64,
) -> error::Result<String> {
    let path = sqlx::query_scalar!(
        "select path from script where hash = $1 AND (workspace_id = $2 OR workspace_id = \
         'starter')",
        hash,
        w_id
    )
    .fetch_one(db)
    .await
    .map_err(|e| {
        Error::InternalErr(format!(
            "querying getting path for hash {hash} in {w_id}: {e}"
        ))
    })?;
    Ok(path)
}

async fn get_job(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::JsonResult<Job> {
    let tx = db.begin().await?;
    let (job_o, tx) = get_job_by_id(tx, &w_id, id).await?;
    let job = crate::utils::not_found_if_none(job_o, "Job", id.to_string())?;
    tx.commit().await?;
    Ok(Json(job))
}

#[derive(Deserialize)]
pub struct ResultByIdQuery {
    pub skip_direct: bool,
}

pub async fn get_job_by_id<'c>(
    mut tx: Transaction<'c, Postgres>,
    w_id: &str,
    id: Uuid,
) -> error::Result<(Option<Job>, Transaction<'c, Postgres>)> {
    let cjob_option = sqlx::query_as::<_, CompletedJob>(
        "SELECT * FROM completed_job WHERE id = $1 AND workspace_id = $2",
    )
    .bind(id)
    .bind(w_id)
    .fetch_optional(&mut tx)
    .await?;
    let job_option = match cjob_option {
        Some(job) => Some(Job::CompletedJob(job)),
        None => get_queued_job(id, w_id, &mut tx).await?.map(Job::QueuedJob),
    };
    if job_option.is_some() {
        Ok((job_option, tx))
    } else {
        // check if a job had been moved in-between queries
        let cjob_option = sqlx::query_as::<_, CompletedJob>(
            "SELECT * FROM completed_job WHERE id = $1 AND workspace_id = $2",
        )
        .bind(id)
        .bind(w_id)
        .fetch_optional(&mut tx)
        .await?;
        Ok((cjob_option.map(Job::CompletedJob), tx))
    }
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct CompletedJob {
    workspace_id: String,
    id: Uuid,
    parent_job: Option<Uuid>,
    created_by: String,
    created_at: chrono::DateTime<chrono::Utc>,
    started_at: chrono::DateTime<chrono::Utc>,
    duration_ms: i32,
    success: bool,
    script_hash: Option<ScriptHash>,
    script_path: Option<String>,
    args: Option<serde_json::Value>,
    result: Option<serde_json::Value>,
    logs: Option<String>,
    deleted: bool,
    raw_code: Option<String>,
    canceled: bool,
    canceled_by: Option<String>,
    canceled_reason: Option<String>,
    job_kind: JobKind,
    schedule_path: Option<String>,
    permissioned_as: String,
    flow_status: Option<serde_json::Value>,
    raw_flow: Option<serde_json::Value>,
    is_flow_step: bool,
    language: Option<ScriptLang>,
    is_skipped: bool,
}

#[derive(Deserialize, Clone, Copy)]
pub struct RunJobQuery {
    scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    scheduled_in_secs: Option<i64>,
    parent_job: Option<Uuid>,
}

impl RunJobQuery {
    async fn get_scheduled_for<'c>(
        self,
        db: &mut Transaction<'c, Postgres>,
    ) -> error::Result<Option<chrono::DateTime<chrono::Utc>>> {
        if let Some(scheduled_for) = self.scheduled_for {
            Ok(Some(scheduled_for))
        } else if let Some(scheduled_in_secs) = self.scheduled_in_secs {
            let now = now_from_db(db).await?;
            Ok(Some(now + chrono::Duration::seconds(scheduled_in_secs)))
        } else {
            Ok(None)
        }
    }
}

#[derive(Deserialize)]
pub struct ListQueueQuery {
    pub script_path_start: Option<String>,
    pub script_path_exact: Option<String>,
    pub script_hash: Option<String>,
    pub created_by: Option<String>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub running: Option<bool>,
    pub parent_job: Option<String>,
    pub order_desc: Option<bool>,
    pub job_kinds: Option<String>,
}

fn list_queue_jobs_query(w_id: &str, lq: &ListQueueQuery, fields: &[&str]) -> SqlBuilder {
    let mut sqlb = SqlBuilder::select_from("queue")
        .fields(fields)
        .order_by("created_at", lq.order_desc.unwrap_or(true))
        .limit(1000)
        .and_where_eq("workspace_id", "?".bind(&w_id))
        .clone();

    if let Some(ps) = &lq.script_path_start {
        sqlb.and_where_like_left("script_path", "?".bind(ps));
    }
    if let Some(p) = &lq.script_path_exact {
        sqlb.and_where_eq("script_path", "?".bind(p));
    }
    if let Some(h) = &lq.script_hash {
        sqlb.and_where_eq("script_hash", "?".bind(h));
    }
    if let Some(cb) = &lq.created_by {
        sqlb.and_where_eq("created_by", "?".bind(cb));
    }
    if let Some(r) = &lq.running {
        sqlb.and_where_eq("running", &r);
    }
    if let Some(pj) = &lq.parent_job {
        sqlb.and_where_eq("parent_job", "?".bind(pj));
    }
    if let Some(dt) = &lq.created_before {
        sqlb.and_where_lt("created_at", format!("to_timestamp({})", dt.timestamp()));
    }
    if let Some(dt) = &lq.created_after {
        sqlb.and_where_gt("created_at", format!("to_timestamp({})", dt.timestamp()));
    }
    if let Some(jk) = &lq.job_kinds {
        sqlb.and_where_in(
            "job_kind",
            &jk.split(',').into_iter().map(quote).collect::<Vec<_>>(),
        );
    }

    sqlb
}

async fn list_queue_jobs(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(lq): Query<ListQueueQuery>,
) -> error::JsonResult<Vec<QueuedJob>> {
    let sql = list_queue_jobs_query(&w_id, &lq, &["*"]).sql()?;
    let jobs = sqlx::query_as::<_, QueuedJob>(&sql).fetch_all(&db).await?;
    Ok(Json(jobs))
}

async fn list_jobs(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListCompletedQuery>,
) -> error::JsonResult<Vec<Job>> {
    todo!("rewrite this to just run list_queue_jobs and list_completed_jobs separately and return as one");
    let (per_page, offset) = crate::utils::paginate(pagination);
    let lqc = lq.clone();
    let sqlq = list_queue_jobs_query(
        &w_id,
        &ListQueueQuery {
            script_path_start: lq.script_path_start,
            script_path_exact: lq.script_path_exact,
            script_hash: lq.script_hash,
            created_by: lq.created_by,
            created_before: lq.created_before,
            created_after: lq.created_after,
            running: None,
            parent_job: lq.parent_job,
            order_desc: Some(true),
            job_kinds: lq.job_kinds,
        },
        &[
            "'QueuedJob' as typ",
            "id",
            "workspace_id",
            "parent_job",
            "created_by",
            "created_at",
            "started_at",
            "scheduled_for",
            "running",
            "script_hash",
            "script_path",
            "args",
            "null as duration_ms",
            "null as success",
            "false as deleted",
            "canceled",
            "canceled_by",
            "job_kind",
            "schedule_path",
            "permissioned_as",
            "flow_status",
            "is_flow_step",
            "language",
            "false as is_skipped",
        ],
    );
    let sqlc = list_completed_jobs_query(
        &w_id,
        per_page + offset,
        0,
        &ListCompletedQuery { order_desc: Some(true), ..lqc },
        &[
            "'CompletedJob' as typ",
            "id",
            "workspace_id",
            "parent_job",
            "created_by",
            "created_at",
            "started_at",
            "null as scheduled_for",
            "null as running",
            "script_hash",
            "script_path",
            "args",
            "duration_ms",
            "success",
            "deleted",
            "canceled",
            "canceled_by",
            "job_kind",
            "schedule_path",
            "permissioned_as",
            "flow_status",
            "is_flow_step",
            "language",
            "is_skipped",
        ],
    );
    let sql = format!(
        "{} UNION ALL {} ORDER BY created_at DESC LIMIT {} OFFSET {};",
        &sqlq.subquery()?,
        &sqlc.subquery()?,
        per_page,
        offset
    );
    let mut tx = user_db.begin(&authed).await?;
    let jobs: Vec<UnifiedJob> = sqlx::query_as(&sql).fetch_all(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(jobs.into_iter().map(From::from).collect()))
}

pub async fn resume_suspended_job(
    /* unauthed */
    Extension(db): Extension<DB>,
    Path((w_id, job, resume_id, secret)): Path<(String, Uuid, u32, String)>,
    QueryOrBody(value): QueryOrBody<serde_json::Value>,
) -> error::Result<StatusCode> {
    let value = value.unwrap_or(serde_json::Value::Null);
    insert_resume_job(&db, &w_id, job, resume_id, secret, false, value).await?;
    Ok(StatusCode::CREATED)
}

pub async fn cancel_suspended_job(
    /* unauthed */
    Extension(db): Extension<DB>,
    Path((w_id, job, resume_id, secret)): Path<(String, Uuid, u32, String)>,
    QueryOrBody(value): QueryOrBody<serde_json::Value>,
) -> error::Result<StatusCode> {
    let value = value.unwrap_or(serde_json::Value::Null);
    insert_resume_job(&db, &w_id, job, resume_id, secret, true, value).await?;
    Ok(StatusCode::CREATED)
}

pub async fn create_job_signature(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, job_id, resume_id)): Path<(String, Uuid, u32)>,
) -> error::Result<String> {
    let key = get_workspace_key(&w_id, &mut user_db.begin(&authed).await?).await?;
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).map_err(to_anyhow)?;
    mac.update(job_id.as_bytes());
    mac.update(resume_id.to_be_bytes().as_ref());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum Job {
    QueuedJob(QueuedJob),
    CompletedJob(CompletedJob),
}

#[derive(sqlx::FromRow)]
struct UnifiedJob {
    workspace_id: String,
    typ: String,
    id: Uuid,
    parent_job: Option<Uuid>,
    created_by: String,
    created_at: chrono::DateTime<chrono::Utc>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    running: Option<bool>,
    script_hash: Option<ScriptHash>,
    script_path: Option<String>,
    args: Option<serde_json::Value>,
    duration_ms: Option<i32>,
    success: Option<bool>,
    deleted: bool,
    canceled: bool,
    canceled_by: Option<String>,
    job_kind: JobKind,
    schedule_path: Option<String>,
    permissioned_as: String,
    flow_status: Option<serde_json::Value>,
    is_flow_step: bool,
    language: Option<ScriptLang>,
    is_skipped: bool,
}

impl From<UnifiedJob> for Job {
    fn from(uj: UnifiedJob) -> Self {
        match uj.typ.as_ref() {
            "CompletedJob" => Job::CompletedJob(CompletedJob {
                workspace_id: uj.workspace_id,
                id: uj.id,
                parent_job: uj.parent_job,
                created_by: uj.created_by,
                created_at: uj.created_at,
                started_at: uj.started_at.unwrap_or(uj.created_at),
                duration_ms: uj.duration_ms.unwrap(),
                success: uj.success.unwrap(),
                script_hash: uj.script_hash,
                script_path: uj.script_path,
                args: uj.args,
                result: None,
                logs: None,
                deleted: uj.deleted,
                canceled: uj.canceled,
                canceled_by: uj.canceled_by,
                raw_code: None,
                canceled_reason: None,
                job_kind: uj.job_kind,
                schedule_path: uj.schedule_path,
                permissioned_as: uj.permissioned_as,
                flow_status: uj.flow_status,
                raw_flow: None,
                is_flow_step: uj.is_flow_step,
                language: uj.language,
                is_skipped: uj.is_skipped,
            }),
            "QueuedJob" => Job::QueuedJob(QueuedJob {
                workspace_id: uj.workspace_id,
                id: uj.id,
                parent_job: uj.parent_job,
                created_by: uj.created_by,
                created_at: uj.created_at,
                started_at: uj.started_at,
                script_hash: uj.script_hash,
                script_path: uj.script_path,
                args: uj.args,
                running: uj.running.unwrap(),
                scheduled_for: uj.scheduled_for.unwrap(),
                logs: None,
                raw_code: None,
                canceled: uj.canceled,
                canceled_by: uj.canceled_by,
                canceled_reason: None,
                last_ping: None,
                job_kind: uj.job_kind,
                schedule_path: uj.schedule_path,
                permissioned_as: uj.permissioned_as,
                flow_status: uj.flow_status,
                raw_flow: None,
                is_flow_step: uj.is_flow_step,
                language: uj.language,
                same_worker: false,
            }),
            t => panic!("job type {} not valid", t),
        }
    }
}
#[derive(Deserialize)]
struct CancelJob {
    reason: Option<String>,
}

#[derive(Deserialize)]
struct Preview {
    content: String,
    path: Option<String>,
    args: Option<Map<String, Value>>,
    language: ScriptLang,
}

#[derive(Deserialize)]
struct PreviewFlow {
    value: FlowValue,
    path: Option<String>,
    args: Option<Map<String, Value>>,
}

#[instrument(level = "trace", skip_all)]
pub async fn add_completed_job_error<E: ToString + std::fmt::Debug>(
    db: &DB,
    queued_job: &QueuedJob,
    logs: String,
    e: E,
    metrics: Option<worker::Metrics>, // remove. worker metrics have no place here
) -> Result<(Uuid, Map<String, Value>), Error> {
    metrics.map(|m| m.worker_execution_failed.inc());
    let mut output_map = serde_json::Map::new();
    output_map.insert(
        "error".to_string(),
        serde_json::Value::String(e.to_string()),
    );
    let a = add_completed_job(
        db,
        &queued_job,
        false,
        false,
        serde_json::Value::Object(output_map.clone()),
        logs,
    )
    .await?;
    Ok((a, output_map))
}

#[instrument(level = "trace", skip_all)]
pub async fn add_completed_job(
    db: &DB,
    queued_job: &QueuedJob,
    success: bool,
    skipped: bool,
    result: serde_json::Value,
    logs: String,
) -> Result<Uuid, Error> {
    let mut tx = db.begin().await?;
    let job_id = queued_job.id.clone();
    sqlx::query!(
        "INSERT INTO completed_job AS cj
                   ( workspace_id
                   , id
                   , parent_job
                   , created_by
                   , created_at
                   , started_at
                   , duration_ms
                   , success
                   , script_hash
                   , script_path
                   , args
                   , result
                   , logs
                   , raw_code
                   , canceled
                   , canceled_by
                   , canceled_reason
                   , job_kind
                   , schedule_path
                   , permissioned_as
                   , flow_status
                   , raw_flow
                   , is_flow_step
                   , is_skipped
                   , language )
            VALUES ($1, $2, $3, $4, $5, $6, EXTRACT(milliseconds FROM (now() - $6)), $7, $8, $9,\
                    $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
         ON CONFLICT (id) DO UPDATE SET success = $7, result = $11, logs = concat(cj.logs, $12)",
        queued_job.workspace_id,
        queued_job.id,
        queued_job.parent_job,
        queued_job.created_by,
        queued_job.created_at,
        queued_job.started_at,
        success,
        queued_job.script_hash.map(|x| x.0),
        queued_job.script_path,
        queued_job.args,
        result,
        logs,
        queued_job.raw_code,
        queued_job.canceled,
        queued_job.canceled_by,
        queued_job.canceled_reason,
        queued_job.job_kind: JobKind,
        queued_job.schedule_path,
        queued_job.permissioned_as,
        queued_job.flow_status,
        queued_job.raw_flow,
        queued_job.is_flow_step,
        skipped,
        queued_job.language: ScriptLang,
    )
    .execute(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("Could not add completed job {job_id}: {e}")))?;
    let _ = delete_job(db, &queued_job.workspace_id, job_id).await?;
    if !queued_job.is_flow_step
        && queued_job.job_kind != JobKind::Flow
        && queued_job.job_kind != JobKind::FlowPreview
        && queued_job.schedule_path.is_some()
        && queued_job.script_path.is_some()
    {
        tx = schedule_again_if_scheduled(
            tx,
            queued_job.schedule_path.as_ref().unwrap(),
            queued_job.script_path.as_ref().unwrap(),
            &queued_job.workspace_id,
        )
        .await?;
    }
    tx.commit().await?;
    tracing::debug!("Added completed job {}", queued_job.id);
    Ok(queued_job.id)
}

#[instrument(level = "trace", skip_all)]
pub async fn schedule_again_if_scheduled<'c>(
    mut tx: Transaction<'c, Postgres>,
    schedule_path: &str,
    script_path: &str,
    w_id: &str,
) -> crate::error::Result<Transaction<'c, Postgres>> {
    let schedule = get_schedule_opt(&mut tx, &w_id, schedule_path)
        .await?
        .ok_or_else(|| {
            Error::InternalErr(format!(
                "Could not find schedule {:?} for workspace {}",
                schedule_path, w_id
            ))
        })?;
    if schedule.enabled && script_path == schedule.script_path {
        tx = crate::schedule::push_scheduled_job(tx, schedule).await?;
    }

    Ok(tx)
}

pub struct QueryOrBody<D>(pub Option<D>);

#[axum::async_trait]
impl<D, B> FromRequest<B> for QueryOrBody<D>
where
    D: DeserializeOwned,
    B: Send + axum::body::HttpBody,
    <B as axum::body::HttpBody>::Data: Send,
    <B as axum::body::HttpBody>::Error: Into<axum::BoxError>,
{
    type Rejection = Response;

    async fn from_request(
        req: &mut axum::extract::RequestParts<B>,
    ) -> Result<Self, Self::Rejection> {
        return if req.method() == axum::http::Method::GET {
            let Query(InPayload { payload }) = Query::from_request(req)
                .await
                .map_err(IntoResponse::into_response)?;
            payload
                .map(|p| {
                    decode_payload(p)
                        .map(QueryOrBody)
                        .map_err(|err| (StatusCode::BAD_REQUEST, format!("{err:#?}")))
                        .map_err(IntoResponse::into_response)
                })
                .unwrap_or(Ok(QueryOrBody(None)))
        } else {
            Json::from_request(req)
                .await
                .map(|Json(v)| QueryOrBody(Some(v)))
                .map_err(IntoResponse::into_response)
        };

        #[derive(Deserialize)]
        struct InPayload {
            payload: Option<String>,
        }

        fn decode_payload<D: DeserializeOwned, T: AsRef<[u8]>>(t: T) -> anyhow::Result<D> {
            let vec = base64::decode_config(&t, base64::URL_SAFE).context("invalid base64")?;
            serde_json::from_slice(vec.as_slice()).context("invalid json")
        }
    }
}
