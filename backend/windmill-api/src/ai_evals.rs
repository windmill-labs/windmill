//! Eval datasets for reusable AI agents.
//!
//! A dataset is a curated set of cases: the inputs an agent is expected to handle, and what it was
//! expected to answer. Datasets, cases and experiments are rows (`eval_dataset`, `eval_case`,
//! `eval_experiment`, `eval_experiment_case`), so a dataset is permissioned, cascaded and queried
//! like any other workspace object.
//!
//! What a run *produced* is deliberately not stored here. The answer, the trajectory and every
//! scorer's return value belong to the job that produced them, and are read back out of
//! `v2_job_completed` when results are displayed, so there is one copy of them and one permission
//! model over them.
//!
//! Reads and dataset writes go through `user_db`, which makes row-level security the authority on
//! who may see or change a dataset. Cases and experiments carry a read policy derived from their
//! dataset and no write policy at all: they are written on the unrestricted pool, after
//! `require_dataset_writable` has asked the dataset row itself whether this caller may write it.

use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use uuid::Uuid;
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    utils::{paginate, Pagination},
};

use crate::db::{ApiAuthed, DB};
use windmill_api_auth::check_scopes;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/datasets/list", get(list_datasets))
        .route("/datasets/create", post(create_dataset))
        .route("/datasets/get/{*path}", get(get_dataset))
        .route("/datasets/update/{*path}", post(update_dataset))
        .route("/datasets/delete/{*path}", post(delete_dataset))
        .route("/cases/list/{*path}", get(list_cases))
        .route("/cases/add/{*path}", post(add_case))
        .route("/cases/update/{*path}", post(update_case))
        .route("/cases/delete/{*path}", post(delete_case))
        .route("/run", post(run_eval))
        .route("/experiments/run", post(run_experiment))
        .route("/experiments/list/{*path}", get(list_experiments))
        .route("/experiments/results/{*path}", get(experiment_results))
        .route("/case_draft/from_job/{job_id}", get(case_draft_from_job))
        .route(
            "/case_draft/from_conversation/{conversation_id}",
            get(case_draft_from_conversation),
        )
}

/// What a run is executed against. Kept as `(kind, path, version)` rather than a bare agent
/// path so flow-scoped evaluation is a later superset instead of a rewrite.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvalSubject {
    pub kind: EvalSubjectKind,
    pub path: String,
    /// `resource_version.id` the agent was at when the run was *enqueued*. Recorded, never
    /// used to pin execution: a linked agent step resolves its resource when it runs (see
    /// `docs/reusable-ai-agents.md`), so a run that waits in the queue across an edit executes
    /// a version later than the one recorded here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EvalSubjectKind {
    Agent,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvalDataset {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The subject the drawer offers by default when this dataset is opened without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_subject: Option<EvalSubject>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub edited_at: DateTime<Utc>,
    pub edited_by: String,
}

/// The agent-facing half of a case: exactly the inputs a standalone run feeds the agent.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EvalCaseInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_attachments: Option<Box<RawValue>>,
    /// Prior turns replayed through `Memory::Manual`, which bypasses stored memory so a
    /// recorded conversation reruns deterministically without polluting production memory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub messages: Option<Box<RawValue>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvalCase {
    pub id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub input: EvalCaseInput,
    /// A linked agent's runtime behaviour depends on its host flow's `tool_inputs` overrides.
    /// A case defaults to the agent's own authored defaults; naming a host flow here resolves
    /// that flow's overrides at run time instead, for when someone hits the discrepancy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_flow_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_inputs: Option<Box<RawValue>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<Box<RawValue>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<EvalCaseSource>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

/// Where a case was captured from, when it came from real traffic rather than being typed.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvalCaseSource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_path: Option<String>,
    pub captured_at: DateTime<Utc>,
}

/// The case fields a caller may set. `id`/`created_at`/`created_by` are assigned server-side so
/// a client cannot forge provenance or collide with an existing case.
#[derive(Deserialize, Debug)]
pub struct NewEvalCase {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub input: EvalCaseInput,
    #[serde(default)]
    pub host_flow_path: Option<String>,
    #[serde(default)]
    pub tool_inputs: Option<Box<RawValue>>,
    #[serde(default)]
    pub expected: Option<Box<RawValue>>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source: Option<EvalCaseSource>,
}

#[derive(Deserialize)]
pub struct CreateDataset {
    pub path: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_subject: Option<EvalSubject>,
}

#[derive(Deserialize)]
pub struct EditDataset {
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_subject: Option<EvalSubject>,
}

#[derive(Deserialize)]
pub struct CaseId {
    pub id: Uuid,
}

#[derive(Deserialize)]
/// The edit fields are spelled out rather than `#[serde(flatten)]`-ing `NewEvalCase`: flatten
/// deserializes through a buffered representation, which silently yields `None` for the
/// `Box<RawValue>` fields — an edited case would lose its conversation and tool inputs.
pub struct UpdateCase {
    pub id: Uuid,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub input: EvalCaseInput,
    #[serde(default)]
    pub host_flow_path: Option<String>,
    #[serde(default)]
    pub tool_inputs: Option<Box<RawValue>>,
    #[serde(default)]
    pub expected: Option<Box<RawValue>>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Serialize)]
pub struct ListCasesResponse {
    pub cases: Vec<EvalCase>,
    pub total: usize,
}
// -----------------------------------------------------------------------------------------------
// Paths and permissions
// -----------------------------------------------------------------------------------------------

/// Dataset paths are Windmill paths, so the folder they live in is what grants access to them.
fn check_path(path: &str) -> Result<()> {
    let segments = path.split('/').collect::<Vec<_>>();
    if segments.len() < 3
        || !matches!(segments[0], "u" | "f")
        || segments.iter().any(|s| s.is_empty())
        || segments.iter().any(|s| *s == "." || *s == "..")
    {
        return Err(Error::BadRequest(format!(
            "Invalid dataset path '{}': expected 'u/<user>/<name>' or 'f/<folder>/<name>'",
            path
        )));
    }
    Ok(())
}

/// A case is text: a message, the answer it was expected to produce, and at most a short replayed
/// conversation. Attachments are S3 references rather than inline bytes, so nothing here is meant
/// to be large. The caps exist so that one mistake — a whole file pasted into a message, a capture
/// loop left running — cannot grow a dataset past what a listing can load.
const MAX_CASE_BYTES: usize = 256 * 1024;
const MAX_CASES_PER_DATASET: i64 = 10_000;

/// Newest first, and only this many: the list feeds a picker, and a dataset that has been run
/// nightly for a year would otherwise send back every run of it.
const MAX_EXPERIMENTS_LISTED: i64 = 100;

/// A refused write surfaces as SQLSTATE 42501 with a message naming the table and the policy,
/// which is neither actionable nor meaningful to whoever made the request.
fn map_write_denied(authed: &ApiAuthed, path: &str, e: sqlx::Error) -> Error {
    if e.as_database_error().and_then(|d| d.code()).as_deref() == Some("42501") {
        return Error::NotAuthorized(format!(
            "User {} does not have write access to eval dataset {}",
            authed.username, path
        ));
    }
    e.into()
}

/// The dataset a write was aimed at is gone. Raised from the foreign key rather than from a
/// preceding existence check, so a dataset deleted mid-request cannot slip between the two.
fn is_missing_dataset(e: &sqlx::Error) -> bool {
    e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23503")
}

/// A write that matched no row is either a dataset that does not exist or one the caller can read
/// but not write. Row-level security cannot distinguish them — both are simply invisible to the
/// statement — so ask again with a plain read.
async fn write_refused(authed: &ApiAuthed, user_db: &UserDB, w_id: &str, path: &str) -> Error {
    let visible = async {
        let mut tx = user_db.clone().begin(authed).await?;
        let found = sqlx::query_scalar!(
            "SELECT path FROM eval_dataset WHERE workspace_id = $1 AND path = $2",
            w_id,
            path
        )
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok::<_, Error>(found.is_some())
    }
    .await;
    match visible {
        Ok(true) => Error::NotAuthorized(format!(
            "User {} does not have write access to eval dataset {}",
            authed.username, path
        )),
        Ok(false) => Error::NotFound(format!("Eval dataset {} not found", path)),
        Err(e) => e,
    }
}

/// Read the dataset the request names, through `user_db` so that a caller who cannot see it gets
/// the same answer as one asking for a dataset that does not exist.
async fn read_dataset(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    path: &str,
) -> Result<EvalDataset> {
    check_path(path)?;
    let mut tx = user_db.clone().begin(authed).await?;
    let row = sqlx::query!(
        "SELECT path, summary, description, default_subject, created_at, created_by, edited_at,
                edited_by
         FROM eval_dataset WHERE workspace_id = $1 AND path = $2",
        w_id,
        path
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let row = row.ok_or_else(|| Error::NotFound(format!("Eval dataset {} not found", path)))?;
    Ok(EvalDataset {
        path: row.path,
        summary: row.summary,
        description: row.description,
        default_subject: row
            .default_subject
            .and_then(|v| serde_json::from_value(v).ok()),
        created_at: row.created_at,
        created_by: row.created_by,
        edited_at: row.edited_at,
        edited_by: row.edited_by,
    })
}

/// Whether this caller may write the contents of a dataset: its cases, and the experiments that
/// run them.
///
/// `SELECT … FOR UPDATE` applies `eval_dataset`'s UPDATE policies on top of its SELECT policies,
/// so this asks the row itself who may write it instead of re-deriving the rule from the path in
/// Rust. A grant that lives in `extra_perms` is honoured without being mirrored here, and a
/// mirrored copy cannot drift from the policies.
async fn require_dataset_writable(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    path: &str,
) -> Result<()> {
    check_path(path)?;
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot modify eval datasets".to_string(),
        ));
    }
    let mut tx = user_db.clone().begin(authed).await?;
    let writable = sqlx::query_scalar!(
        "SELECT path FROM eval_dataset WHERE workspace_id = $1 AND path = $2 FOR UPDATE",
        w_id,
        path
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    if writable.is_some() {
        Ok(())
    } else {
        Err(write_refused(authed, user_db, w_id, path).await)
    }
}

/// jsonb columns are read as `serde_json::Value` and handed on as `RawValue`, which is what the
/// case shape stores: a case's `expected` and `tool_inputs` are arbitrary user JSON that this
/// module never looks inside.
fn to_raw(value: serde_json::Value) -> Result<Box<RawValue>> {
    Ok(serde_json::value::to_raw_value(&value)?)
}

fn from_raw(value: &RawValue) -> Result<serde_json::Value> {
    Ok(serde_json::from_str(value.get())?)
}

fn opt_to_raw(value: Option<serde_json::Value>) -> Result<Option<Box<RawValue>>> {
    value.map(to_raw).transpose()
}

fn opt_from_raw(value: Option<&Box<RawValue>>) -> Result<Option<serde_json::Value>> {
    value.map(|v| from_raw(v)).transpose()
}

/// Everything a case carries that a caller supplied, weighed against `MAX_CASE_BYTES`.
fn check_case_size(input: &EvalCaseInput, expected: Option<&Box<RawValue>>) -> Result<()> {
    let mut bytes = serde_json::to_vec(input)?.len();
    if let Some(expected) = expected {
        bytes += expected.get().len();
    }
    if bytes > MAX_CASE_BYTES {
        return Err(Error::BadRequest(format!(
            "This eval case is {} KiB, over the {} KiB limit. Attachments belong in workspace \
             storage and are referenced by a case, not stored inside it.",
            bytes / 1024,
            MAX_CASE_BYTES / 1024
        )));
    }
    Ok(())
}

// -----------------------------------------------------------------------------------------------
// Datasets
// -----------------------------------------------------------------------------------------------

pub async fn list_datasets(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<EvalDataset>> {
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query!(
        "SELECT path, summary, description, default_subject, created_at, created_by, edited_at,
                edited_by
         FROM eval_dataset WHERE workspace_id = $1 ORDER BY path",
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| EvalDataset {
                path: row.path,
                summary: row.summary,
                description: row.description,
                default_subject: row
                    .default_subject
                    .and_then(|v| serde_json::from_value(v).ok()),
                created_at: row.created_at,
                created_by: row.created_by,
                edited_at: row.edited_at,
                edited_by: row.edited_by,
            })
            .collect(),
    ))
}

pub async fn create_dataset(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(payload): Json<CreateDataset>,
) -> Result<String> {
    check_path(&payload.path)?;
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot create eval datasets".to_string(),
        ));
    }
    let default_subject = payload
        .default_subject
        .map(|s| serde_json::to_value(s))
        .transpose()?;
    let mut tx = user_db.begin(&authed).await?;
    // A path already taken returns no row; a path the caller may not write raises the policy
    // error `map_write_denied` translates. The two are distinct answers and must stay so.
    let created = sqlx::query_scalar!(
        "INSERT INTO eval_dataset
            (workspace_id, path, summary, description, default_subject, created_by, edited_by)
         VALUES ($1, $2, $3, $4, $5, $6, $6)
         ON CONFLICT (workspace_id, path) DO NOTHING
         RETURNING path",
        w_id,
        payload.path,
        payload.summary,
        payload.description,
        default_subject,
        authed.username,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| map_write_denied(&authed, &payload.path, e))?;
    tx.commit().await?;
    if created.is_none() {
        return Err(Error::BadRequest(format!(
            "Eval dataset {} already exists",
            payload.path
        )));
    }
    Ok(format!("Created eval dataset {}", payload.path))
}

pub async fn get_dataset(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, String)>,
) -> JsonResult<EvalDataset> {
    Ok(Json(read_dataset(&authed, &user_db, &w_id, &path).await?))
}

pub async fn update_dataset(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, String)>,
    Json(payload): Json<EditDataset>,
) -> Result<String> {
    check_path(&path)?;
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot modify eval datasets".to_string(),
        ));
    }
    let default_subject = payload
        .default_subject
        .map(|s| serde_json::to_value(s))
        .transpose()?;
    let mut tx = user_db.clone().begin(&authed).await?;
    let updated = sqlx::query_scalar!(
        "UPDATE eval_dataset
         SET summary = $3, description = $4, default_subject = $5, edited_at = now(),
             edited_by = $6
         WHERE workspace_id = $1 AND path = $2
         RETURNING path",
        w_id,
        path,
        payload.summary,
        payload.description,
        default_subject,
        authed.username,
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    if updated.is_none() {
        return Err(write_refused(&authed, &user_db, &w_id, &path).await);
    }
    Ok(format!("Updated eval dataset {}", path))
}

/// The cases, the experiments and their recorded case sets go with the dataset, through the
/// foreign keys. The jobs those experiments produced are not touched: they are jobs, with their
/// own retention, and a run that happened is not undone by curating the dataset away.
pub async fn delete_dataset(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, String)>,
) -> Result<String> {
    check_path(&path)?;
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot delete eval datasets".to_string(),
        ));
    }
    let mut tx = user_db.clone().begin(&authed).await?;
    let deleted = sqlx::query_scalar!(
        "DELETE FROM eval_dataset WHERE workspace_id = $1 AND path = $2 RETURNING path",
        w_id,
        path
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    if deleted.is_none() {
        return Err(write_refused(&authed, &user_db, &w_id, &path).await);
    }
    Ok(format!("Deleted eval dataset {}", path))
}

// -----------------------------------------------------------------------------------------------
// Cases
// -----------------------------------------------------------------------------------------------

/// The column list every case read shares. Ordered oldest first so a case keeps its position in
/// the list as the dataset grows.
async fn read_cases(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    dataset: &str,
    page: Option<(usize, usize)>,
) -> Result<Vec<EvalCase>> {
    let (limit, offset) = match page {
        Some((per_page, offset)) => (per_page as i64, offset as i64),
        None => (i64::MAX, 0),
    };
    let mut tx = user_db.clone().begin(authed).await?;
    let rows = sqlx::query!(
        "SELECT id, name, input, host_flow_path, tool_inputs, expected, tags, source, created_at,
                created_by
         FROM eval_case
         WHERE workspace_id = $1 AND dataset_path = $2
         ORDER BY created_at, id
         LIMIT $3 OFFSET $4",
        w_id,
        dataset,
        limit,
        offset
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    rows.into_iter()
        .map(|row| {
            Ok(EvalCase {
                id: row.id,
                name: row.name,
                input: serde_json::from_value(row.input)?,
                host_flow_path: row.host_flow_path,
                tool_inputs: opt_to_raw(row.tool_inputs)?,
                expected: opt_to_raw(row.expected)?,
                tags: row.tags,
                source: row.source.and_then(|v| serde_json::from_value(v).ok()),
                created_at: row.created_at,
                created_by: row.created_by,
            })
        })
        .collect()
}

pub async fn list_cases(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, String)>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<ListCasesResponse> {
    // Reading the dataset first so an unknown or unreadable one is a 404 rather than an empty
    // dataset: the case rows are invisible in both cases.
    read_dataset(&authed, &user_db, &w_id, &path).await?;
    let (per_page, offset) = paginate(pagination);
    let cases = read_cases(&authed, &user_db, &w_id, &path, Some((per_page, offset))).await?;
    let mut tx = user_db.begin(&authed).await?;
    let total = sqlx::query_scalar!(
        "SELECT count(*) AS \"count!\" FROM eval_case WHERE workspace_id = $1 AND dataset_path = $2",
        w_id,
        path
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(ListCasesResponse { cases, total: total as usize }))
}

pub async fn add_case(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, String)>,
    Json(payload): Json<NewEvalCase>,
) -> Result<String> {
    require_dataset_writable(&authed, &user_db, &w_id, &path).await?;
    check_case_size(&payload.input, payload.expected.as_ref())?;

    let count = sqlx::query_scalar!(
        "SELECT count(*) AS \"count!\" FROM eval_case WHERE workspace_id = $1 AND dataset_path = $2",
        w_id,
        path
    )
    .fetch_one(&db)
    .await?;
    if count >= MAX_CASES_PER_DATASET {
        return Err(Error::BadRequest(format!(
            "Eval dataset {} already holds {} cases, the maximum. Split it into several datasets.",
            path, MAX_CASES_PER_DATASET
        )));
    }

    let id = sqlx::query_scalar!(
        "INSERT INTO eval_case
            (workspace_id, dataset_path, name, input, host_flow_path, tool_inputs, expected, tags,
             source, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING id",
        w_id,
        path,
        payload.name,
        serde_json::to_value(&payload.input)?,
        payload.host_flow_path,
        opt_from_raw(payload.tool_inputs.as_ref())?,
        opt_from_raw(payload.expected.as_ref())?,
        &payload.tags,
        payload
            .source
            .map(|s| serde_json::to_value(s))
            .transpose()?,
        authed.username,
    )
    .fetch_one(&db)
    .await
    .map_err(|e| {
        if is_missing_dataset(&e) {
            Error::NotFound(format!("Eval dataset {} not found", path))
        } else {
            e.into()
        }
    })?;
    Ok(id.to_string())
}

pub async fn update_case(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, String)>,
    Json(payload): Json<UpdateCase>,
) -> Result<String> {
    require_dataset_writable(&authed, &user_db, &w_id, &path).await?;
    check_case_size(&payload.input, payload.expected.as_ref())?;
    let updated = sqlx::query_scalar!(
        "UPDATE eval_case
         SET name = $4, input = $5, host_flow_path = $6, tool_inputs = $7, expected = $8, tags = $9
         WHERE workspace_id = $1 AND dataset_path = $2 AND id = $3
         RETURNING id",
        w_id,
        path,
        payload.id,
        payload.name,
        serde_json::to_value(&payload.input)?,
        payload.host_flow_path,
        opt_from_raw(payload.tool_inputs.as_ref())?,
        opt_from_raw(payload.expected.as_ref())?,
        &payload.tags,
    )
    .fetch_optional(&db)
    .await?;
    if updated.is_none() {
        return Err(Error::NotFound(format!(
            "Eval case {} not found in {}",
            payload.id, path
        )));
    }
    Ok(format!("Updated eval case {}", payload.id))
}

pub async fn delete_case(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, String)>,
    Json(payload): Json<CaseId>,
) -> Result<String> {
    require_dataset_writable(&authed, &user_db, &w_id, &path).await?;
    let deleted = sqlx::query_scalar!(
        "DELETE FROM eval_case
         WHERE workspace_id = $1 AND dataset_path = $2 AND id = $3
         RETURNING id",
        w_id,
        path,
        payload.id
    )
    .fetch_optional(&db)
    .await?;
    if deleted.is_none() {
        return Err(Error::NotFound(format!(
            "Eval case {} not found in {}",
            payload.id, path
        )));
    }
    Ok(format!("Deleted eval case {}", payload.id))
}
// -----------------------------------------------------------------------------------------------
// Standalone runs
// -----------------------------------------------------------------------------------------------

/// A scorer is any runnable. Built-ins are hub scripts, and LLM-as-judge is a reusable agent
/// used here — none of it needs a scoring engine of its own.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScorerRef {
    pub kind: ScorerKind,
    pub path: String,
    /// Shown as the column header. Defaults to the last segment of the path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ScorerKind {
    Script,
    Flow,
    Agent,
}

impl ScorerRef {
    pub fn label(&self) -> String {
        self.name.clone().unwrap_or_else(|| {
            self.path
                .rsplit('/')
                .next()
                .unwrap_or(&self.path)
                .to_string()
        })
    }
}

/// Node id of the agent step; scorers are `score_0`, `score_1`… Results are read back by these
/// ids, so they are part of the stored shape, not an implementation detail.
pub const AGENT_NODE_ID: &str = "a";

pub fn scorer_node_id(index: usize) -> String {
    format!("score_{}", index)
}

/// The flow one case runs as: the agent, then a step per scorer. One job per case, so every
/// case keeps the run stamp, history query and trajectory view that a single run already has.
fn build_case_flow(
    agent_path: &str,
    case: &NewEvalCase,
    tool_inputs: Option<Box<RawValue>>,
    scorers: &[ScorerRef],
) -> Result<windmill_common::flows::FlowValue> {
    let mut input_transforms = serde_json::Map::new();
    for key in ["user_message", "user_attachments"] {
        input_transforms.insert(
            key.to_string(),
            serde_json::json!({ "type": "javascript", "expr": format!("flow_input.{}", key) }),
        );
    }
    // Replaying a recorded conversation goes through `Memory::Manual`, which takes the message
    // list verbatim and bypasses stored memory — so a replay is deterministic and does not
    // write back into the memory a production conversation is using.
    if let Some(messages) = &case.input.messages {
        input_transforms.insert(
            "memory".to_string(),
            serde_json::json!({
                "type": "static",
                "value": { "kind": "manual", "messages": messages }
            }),
        );
    }

    let mut agent_value = serde_json::Map::new();
    agent_value.insert("type".to_string(), serde_json::json!("aiagent"));
    agent_value.insert("agent".to_string(), serde_json::json!(agent_path));
    agent_value.insert("tools".to_string(), serde_json::json!([]));
    agent_value.insert(
        "input_transforms".to_string(),
        serde_json::Value::Object(input_transforms),
    );
    if let Some(tool_inputs) = tool_inputs {
        agent_value.insert(
            "tool_inputs".to_string(),
            serde_json::from_str(tool_inputs.get())?,
        );
    }

    let mut modules = vec![serde_json::json!({
        "id": AGENT_NODE_ID,
        "value": serde_json::Value::Object(agent_value),
    })];

    for (index, scorer) in scorers.iter().enumerate() {
        let output_expr = format!("results.{}.output", AGENT_NODE_ID);
        let value = match scorer.kind {
            // A judge agent is prompted with the case and the answer as one JSON message; a
            // script or flow receives them as named arguments.
            ScorerKind::Agent => serde_json::json!({
                "type": "aiagent",
                "agent": scorer.path,
                "tools": [],
                "input_transforms": {
                    "user_message": {
                        "type": "javascript",
                        "expr": format!(
                            "JSON.stringify({{ input: flow_input._eval_input, output: {}, expected: flow_input.expected }})",
                            output_expr
                        ),
                    }
                }
            }),
            ScorerKind::Script | ScorerKind::Flow => serde_json::json!({
                "type": if scorer.kind == ScorerKind::Script { "script" } else { "flow" },
                "path": scorer.path,
                "input_transforms": {
                    "input": { "type": "javascript", "expr": "flow_input._eval_input" },
                    "output": { "type": "javascript", "expr": output_expr },
                    "expected": { "type": "javascript", "expr": "flow_input.expected" },
                }
            }),
        };
        modules.push(serde_json::json!({ "id": scorer_node_id(index), "value": value }));
    }

    Ok(serde_json::from_value(
        serde_json::json!({ "modules": modules }),
    )?)
}

#[derive(Deserialize)]
pub struct RunEval {
    pub subject: EvalSubject,
    #[serde(default)]
    pub scorers: Vec<ScorerRef>,
    /// The case to run. Either supplied inline (the playground) or loaded from `dataset` +
    /// `case_id`, which additionally stamps the run so it can be found again.
    #[serde(default)]
    pub case: Option<NewEvalCase>,
    #[serde(default)]
    pub dataset: Option<String>,
    #[serde(default)]
    pub case_id: Option<Uuid>,
}

/// `v2_job.runnable_path` is varchar(255); paths that long are pathological but must not fail
/// the run, so an oversized stamp degrades to the agent path and the `_eval` args carry the
/// rest.
const MAX_RUNNABLE_PATH: usize = 255;

fn run_path(agent_path: &str, dataset: Option<&str>, case_id: Option<Uuid>) -> String {
    match (dataset, case_id) {
        (Some(dataset), Some(case_id)) => {
            let full = format!("{}/{}/{}", agent_path, dataset, case_id);
            if full.len() <= MAX_RUNNABLE_PATH {
                full
            } else {
                agent_path.to_string()
            }
        }
        _ => agent_path.to_string(),
    }
}

async fn current_resource_version(db: &DB, w_id: &str, path: &str) -> Result<Option<i64>> {
    let version = sqlx::query_scalar!(
        "SELECT id FROM resource_version WHERE workspace_id = $1 AND path = $2
         ORDER BY id DESC LIMIT 1",
        w_id,
        path
    )
    .fetch_optional(db)
    .await?;
    Ok(version)
}

/// A linked agent's tools bind to their host flow through that flow's `tool_inputs`, so a case
/// that names a host flow reproduces the flow's wiring rather than the agent's own defaults.
/// Read through `user_db` so naming a flow the caller cannot read does not leak its wiring.
async fn tool_inputs_from_host_flow(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    flow_path: &str,
    agent_path: &str,
) -> Result<Option<Box<RawValue>>> {
    let mut tx = user_db.clone().begin(authed).await?;
    let value = sqlx::query_scalar!(
        "SELECT flow_version.value AS \"value!\" FROM flow
         LEFT JOIN flow_version ON flow_version.id = flow.versions[array_upper(flow.versions, 1)]
         WHERE flow.workspace_id = $1 AND flow.path = $2",
        w_id,
        flow_path
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let value =
        value.ok_or_else(|| Error::NotFound(format!("Host flow {} not found", flow_path)))?;
    Ok(find_tool_inputs(&value, agent_path))
}

/// Depth-first search for the `tool_inputs` of the step linked to `agent_path`. Nested agent
/// tools are searched too, so a case can reproduce an agent used as another agent's tool.
fn find_tool_inputs(value: &serde_json::Value, agent_path: &str) -> Option<Box<RawValue>> {
    match value {
        serde_json::Value::Object(map) => {
            if map.get("type").and_then(|t| t.as_str()) == Some("aiagent")
                && map
                    .get("agent")
                    .and_then(|a| a.as_str())
                    .map(strip_res_prefix)
                    == Some(agent_path)
            {
                if let Some(tool_inputs) = map.get("tool_inputs") {
                    return serde_json::value::to_raw_value(tool_inputs).ok();
                }
            }
            map.values().find_map(|v| find_tool_inputs(v, agent_path))
        }
        serde_json::Value::Array(items) => {
            items.iter().find_map(|v| find_tool_inputs(v, agent_path))
        }
        _ => None,
    }
}

fn strip_res_prefix(path: &str) -> &str {
    path.trim_start_matches("$res:")
        .trim_start_matches("res://")
}

/// Push one case as its own job. Shared by a single run and by every case of an experiment,
/// so both produce the same stamped, self-describing job.
async fn push_case_run(
    authed: &ApiAuthed,
    db: &DB,
    user_db: &UserDB,
    w_id: &str,
    agent_path: &str,
    version: Option<i64>,
    case: &NewEvalCase,
    tool_inputs: Option<Box<RawValue>>,
    scorers: &[ScorerRef],
    dataset: Option<&str>,
    case_id: Option<Uuid>,
    experiment_id: Option<Uuid>,
    // Chosen by an experiment before it pushes anything, so the record of a run cannot be
    // missing the job it names. A single run lets `push` assign one.
    job_id: Option<Uuid>,
) -> Result<Uuid> {
    use windmill_common::{jobs::JobPayload, users::username_to_permissioned_as};
    use windmill_queue::{push, PushArgs, PushIsolationLevel};

    let flow_value = build_case_flow(agent_path, case, tool_inputs, scorers)?;

    let mut args = std::collections::HashMap::new();
    if let Some(user_message) = &case.input.user_message {
        args.insert(
            "user_message".to_string(),
            serde_json::value::to_raw_value(user_message)?,
        );
    }
    if let Some(user_attachments) = &case.input.user_attachments {
        args.insert("user_attachments".to_string(), user_attachments.clone());
    }
    if let Some(expected) = &case.expected {
        args.insert("expected".to_string(), expected.clone());
    }
    // The whole case input, for scorers: the message alone cannot explain an answer that came
    // from attachments or a replayed conversation. Only when something will read it.
    if !scorers.is_empty() {
        args.insert(
            "_eval_input".to_string(),
            serde_json::value::to_raw_value(&case.input)?,
        );
    }
    // Self-describing run: opened cold from the runs page, the job says what it was evaluating.
    // Extra flow inputs are inert — the agent step reads only user_message/user_attachments.
    args.insert(
        "_eval".to_string(),
        serde_json::value::to_raw_value(&serde_json::json!({
            "subject": { "kind": "agent", "path": agent_path, "version": version },
            "dataset": dataset,
            "case_id": case_id,
            "experiment_id": experiment_id,
        }))?,
    );

    let path = run_path(agent_path, dataset, case_id);
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into());
    let (uuid, tx) = push(
        db,
        tx,
        w_id,
        JobPayload::RawFlow { value: flow_value, path: Some(path), restarted_from: None },
        PushArgs::from(&args),
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        authed.token_prefix.as_deref(),
        authed.username_override.as_deref(),
        None,
        None,
        None,
        None,
        None,
        job_id,
        false,
        false,
        None,
        true,
        None,
        None,
        None,
        None,
        Some(&authed.clone().into()),
        false,
        None,
        authed.trigger_or_fallback(None),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(uuid)
}

/// Read the agent through `user_db` so a caller who cannot read the resource cannot run it.
async fn require_agent(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    agent_path: &str,
) -> Result<()> {
    let mut tx = user_db.clone().begin(authed).await?;
    let resource_type = sqlx::query_scalar!(
        "SELECT resource_type FROM resource WHERE workspace_id = $1 AND path = $2",
        w_id,
        agent_path
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    match resource_type.as_deref() {
        Some("ai_agent") => Ok(()),
        Some(other) => Err(Error::BadRequest(format!(
            "Resource {} is a {}, not an ai_agent",
            agent_path, other
        ))),
        None => Err(Error::NotFound(format!("Agent {} not found", agent_path))),
    }
}

/// One stored case, read through `user_db`: the case rows inherit their dataset's read policy, so
/// a caller who cannot see the dataset cannot run one of its cases either.
async fn read_case(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    dataset: &str,
    case_id: Uuid,
) -> Result<NewEvalCase> {
    let mut tx = user_db.clone().begin(authed).await?;
    let row = sqlx::query!(
        "SELECT name, input, host_flow_path, tool_inputs, expected, tags, source
         FROM eval_case
         WHERE workspace_id = $1 AND dataset_path = $2 AND id = $3",
        w_id,
        dataset,
        case_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let row = row.ok_or_else(|| {
        Error::NotFound(format!("Eval case {} not found in {}", case_id, dataset))
    })?;
    Ok(NewEvalCase {
        name: row.name,
        input: serde_json::from_value(row.input)?,
        host_flow_path: row.host_flow_path,
        tool_inputs: opt_to_raw(row.tool_inputs)?,
        expected: opt_to_raw(row.expected)?,
        tags: row.tags,
        source: row.source.and_then(|v| serde_json::from_value(v).ok()),
    })
}

pub async fn run_eval(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(payload): Json<RunEval>,
) -> Result<(axum::http::StatusCode, String)> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot run eval jobs".to_string(),
        ));
    }
    check_scopes(&authed, || "jobs:run".to_string())?;

    let EvalSubject { kind: _, path: agent_path, .. } = payload.subject;

    // Resolve the case before anything else: a stored case is the source of truth for what ran,
    // so an inline body must not be able to override one.
    let case = match (&payload.dataset, payload.case_id) {
        (Some(dataset), Some(case_id)) => {
            read_case(&authed, &user_db, &w_id, dataset, case_id).await?
        }
        (None, None) => payload.case.ok_or_else(|| {
            Error::BadRequest("Either a case or a dataset and case_id must be supplied".to_string())
        })?,
        _ => {
            return Err(Error::BadRequest(
                "dataset and case_id must be supplied together".to_string(),
            ))
        }
    };

    require_agent(&authed, &user_db, &w_id, &agent_path).await?;

    // The version the agent is at now. Recorded so the run stays attributable to a prompt state
    // later; it does not pin execution, which stays live.
    let version = current_resource_version(&db, &w_id, &agent_path).await?;

    let tool_inputs = match (&case.tool_inputs, &case.host_flow_path) {
        (Some(explicit), _) => Some(explicit.clone()),
        (None, Some(flow_path)) => {
            tool_inputs_from_host_flow(&authed, &user_db, &w_id, flow_path, &agent_path).await?
        }
        (None, None) => None,
    };

    let uuid = push_case_run(
        &authed,
        &db,
        &user_db,
        &w_id,
        &agent_path,
        version,
        &case,
        tool_inputs,
        &payload.scorers,
        payload.dataset.as_deref(),
        payload.case_id,
        None,
        None,
    )
    .await?;
    Ok((axum::http::StatusCode::CREATED, uuid.to_string()))
}

// -----------------------------------------------------------------------------------------------
// Experiments
// -----------------------------------------------------------------------------------------------

/// One run of a dataset.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvalExperiment {
    pub id: Uuid,
    pub dataset: String,
    pub subject: EvalSubject,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scorers: Vec<ScorerRef>,
    pub case_count: i64,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

/// A case as it was when the experiment ran, and the job it became. Recorded by value, not by
/// reference to `eval_case`: a dataset keeps changing, and a result set that cannot say which
/// inputs produced it is not reproducible. `case_id` is a plain column for the same reason —
/// deleting a case must not rewrite the history of the runs that used it.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExperimentCase {
    pub case_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub input: EvalCaseInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<Box<RawValue>>,
    pub job_id: Uuid,
}

#[derive(Deserialize)]
pub struct RunExperiment {
    pub dataset: String,
    pub subject: EvalSubject,
    #[serde(default)]
    pub scorers: Vec<ScorerRef>,
    /// Applies one host flow's tool bindings to every case. Per-case `host_flow_path` is only
    /// honoured by a single run: one experiment runs one wiring, or its rows would not be
    /// comparable with each other.
    #[serde(default)]
    pub host_flow_path: Option<String>,
}

pub async fn run_experiment(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(payload): Json<RunExperiment>,
) -> Result<String> {
    check_scopes(&authed, || "jobs:run".to_string())?;
    // A write, not a read: it persists an experiment into the dataset.
    require_dataset_writable(&authed, &user_db, &w_id, &payload.dataset).await?;

    let agent_path = payload.subject.path.clone();
    require_agent(&authed, &user_db, &w_id, &agent_path).await?;
    let version = current_resource_version(&db, &w_id, &agent_path).await?;

    let cases = read_cases(&authed, &user_db, &w_id, &payload.dataset, None).await?;
    if cases.is_empty() {
        return Err(Error::BadRequest(format!(
            "Eval dataset {} has no case to run",
            payload.dataset
        )));
    }

    let tool_inputs = match &payload.host_flow_path {
        Some(flow_path) => {
            tool_inputs_from_host_flow(&authed, &user_db, &w_id, flow_path, &agent_path).await?
        }
        None => None,
    };

    // Every job id is chosen here, and the whole experiment is recorded before anything is
    // queued. A launch that dies partway therefore leaves a recorded case whose job is missing —
    // which the results table shows — rather than a running job that no experiment accounts for
    // and nothing will ever collect.
    let experiment_id = Uuid::new_v4();
    let job_ids = cases.iter().map(|_| Uuid::new_v4()).collect::<Vec<_>>();
    let case_count = cases.len();

    let mut tx = db.begin().await?;
    sqlx::query!(
        "INSERT INTO eval_experiment
            (id, workspace_id, dataset_path, subject, scorers, created_by)
         VALUES ($1, $2, $3, $4, $5, $6)",
        experiment_id,
        w_id,
        payload.dataset,
        serde_json::to_value(EvalSubject {
            kind: EvalSubjectKind::Agent,
            path: agent_path.clone(),
            version,
        })?,
        serde_json::to_value(&payload.scorers)?,
        authed.username,
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        if is_missing_dataset(&e) {
            Error::NotFound(format!("Eval dataset {} not found", payload.dataset))
        } else {
            e.into()
        }
    })?;

    let ordinals = (0..case_count as i32).collect::<Vec<_>>();
    let case_ids = cases.iter().map(|c| c.id).collect::<Vec<_>>();
    let names = cases.iter().map(|c| c.name.clone()).collect::<Vec<_>>();
    let inputs = cases
        .iter()
        .map(|c| serde_json::to_value(&c.input))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let expecteds = cases
        .iter()
        .map(|c| opt_from_raw(c.expected.as_ref()))
        .collect::<Result<Vec<_>>>()?;
    sqlx::query!(
        "INSERT INTO eval_experiment_case
            (experiment_id, ordinal, case_id, name, input, expected, job_id)
         SELECT $1, ordinal, case_id, name, input, expected, job_id
         FROM UNNEST($2::int[], $3::uuid[], $4::text[], $5::jsonb[], $6::jsonb[], $7::uuid[])
              AS t(ordinal, case_id, name, input, expected, job_id)",
        experiment_id,
        &ordinals,
        &case_ids,
        &names as &[Option<String>],
        &inputs,
        &expecteds as &[Option<serde_json::Value>],
        &job_ids,
    )
    .execute(&mut *tx)
    .await?;
    // The foreign key is the guard against a dataset deleted while this was being assembled: the
    // commit fails and no job has been queued yet.
    tx.commit().await?;

    let mut launched = 0usize;
    let mut push_error: Option<Error> = None;
    for (case, job_id) in cases.into_iter().zip(job_ids.iter().copied()) {
        let new_case = NewEvalCase {
            name: case.name,
            input: case.input,
            host_flow_path: case.host_flow_path,
            tool_inputs: case.tool_inputs,
            expected: case.expected,
            tags: case.tags,
            source: case.source,
        };
        if let Err(e) = push_case_run(
            &authed,
            &db,
            &user_db,
            &w_id,
            &agent_path,
            version,
            &new_case,
            tool_inputs.clone(),
            &payload.scorers,
            Some(&payload.dataset),
            Some(case.id),
            Some(experiment_id),
            Some(job_id),
        )
        .await
        {
            push_error = Some(e);
            break;
        }
        launched += 1;
    }

    if let Some(e) = push_error {
        // Drop the cases that never made it to the queue, so the experiment holds exactly what
        // ran. Left behind, they would read as jobs that vanished.
        sqlx::query!(
            "DELETE FROM eval_experiment_case WHERE experiment_id = $1 AND ordinal >= $2",
            experiment_id,
            launched as i32
        )
        .execute(&db)
        .await?;
        if launched == 0 {
            sqlx::query!("DELETE FROM eval_experiment WHERE id = $1", experiment_id)
                .execute(&db)
                .await?;
            return Err(e);
        }
        return Err(Error::internal_err(format!(
            "Experiment {} launched {} of {} cases before failing: {}. The launched cases are \
             recorded under that experiment; rerunning starts a new one.",
            experiment_id, launched, case_count, e
        )));
    }
    Ok(experiment_id.to_string())
}

fn experiment_from_row(
    id: Uuid,
    dataset: String,
    subject: serde_json::Value,
    scorers: serde_json::Value,
    case_count: i64,
    created_at: DateTime<Utc>,
    created_by: String,
) -> Result<EvalExperiment> {
    Ok(EvalExperiment {
        id,
        dataset,
        subject: serde_json::from_value(subject)?,
        scorers: serde_json::from_value(scorers)?,
        case_count,
        created_at,
        created_by,
    })
}

pub async fn list_experiments(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, dataset)): Path<(String, String)>,
) -> JsonResult<Vec<EvalExperiment>> {
    read_dataset(&authed, &user_db, &w_id, &dataset).await?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query!(
        "SELECT e.id, e.subject, e.scorers, e.created_at, e.created_by,
                (SELECT count(*) FROM eval_experiment_case c WHERE c.experiment_id = e.id)
                    AS \"case_count!\"
         FROM eval_experiment e
         WHERE e.workspace_id = $1 AND e.dataset_path = $2
         ORDER BY e.created_at DESC
         LIMIT $3",
        w_id,
        dataset,
        MAX_EXPERIMENTS_LISTED
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    rows.into_iter()
        .map(|row| {
            experiment_from_row(
                row.id,
                dataset.clone(),
                row.subject,
                row.scorers,
                row.case_count,
                row.created_at,
                row.created_by,
            )
        })
        .collect::<Result<Vec<_>>>()
        .map(Json)
}

#[derive(Deserialize)]
pub struct ExperimentRef {
    pub id: Uuid,
}

/// One row per case: what it was asked, what the agent answered, and each scorer's number.
#[derive(Serialize)]
pub struct ExperimentRow {
    pub case_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub input: EvalCaseInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<Box<RawValue>>,
    pub job_id: Uuid,
    /// The case job's own status: `running` until it completes, then `success`, `failure`,
    /// `canceled` or `skipped`.
    pub status: String,
    /// The agent's answer, which is what a table cell shows. The whole trajectory stays
    /// reachable through `job_id`, so the row carries the text rather than the result object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// One entry per scorer, in the experiment's scorer order; `None` where it has not
    /// produced a number yet.
    pub scores: Vec<Option<f64>>,
}

#[derive(Serialize)]
pub struct ExperimentResults {
    pub experiment: EvalExperiment,
    pub rows: Vec<ExperimentRow>,
    pub scorer_labels: Vec<String>,
}

/// The agent's own result is `{output, messages, usage}`; the answer is its `output`.
fn agent_answer(result: &RawValue) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(result.get()).ok()?;
    match parsed.get("output") {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(other) => Some(other.to_string()),
        None => None,
    }
}

/// A scorer may return a bare number, a boolean, or `{score, label}`; anything else has no
/// number to plot and is left empty rather than guessed at.
fn extract_score(value: &RawValue) -> Option<f64> {
    let parsed: serde_json::Value = serde_json::from_str(value.get()).ok()?;
    match parsed {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::Bool(b) => Some(if b { 1.0 } else { 0.0 }),
        serde_json::Value::Object(map) => match map.get("score") {
            Some(serde_json::Value::Number(n)) => n.as_f64(),
            Some(serde_json::Value::Bool(b)) => Some(if *b { 1.0 } else { 0.0 }),
            // An agent scorer wraps its answer in `output`: a number, a boolean, a
            // structured {score}, or a string holding any of those.
            _ => match map.get("output") {
                Some(serde_json::Value::Number(n)) => n.as_f64(),
                Some(serde_json::Value::Bool(b)) => Some(if *b { 1.0 } else { 0.0 }),
                Some(serde_json::Value::Object(inner)) => match inner.get("score") {
                    Some(serde_json::Value::Number(n)) => n.as_f64(),
                    Some(serde_json::Value::Bool(b)) => Some(if *b { 1.0 } else { 0.0 }),
                    _ => None,
                },
                Some(serde_json::Value::String(s)) => s.trim().parse::<f64>().ok().or_else(|| {
                    serde_json::from_str::<serde_json::Value>(s)
                        .ok()
                        .and_then(|v| extract_score(&serde_json::value::to_raw_value(&v).ok()?))
                }),
                _ => None,
            },
        },
        _ => None,
    }
}

/// The rows a results table is built from. The job ids come out of `eval_experiment_case`, which
/// only this module writes, so they can be read on the unrestricted pool: the caller's access was
/// established by the dataset read above, and the ids are not caller-supplied.
pub async fn experiment_results(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, dataset)): Path<(String, String)>,
    Query(query): Query<ExperimentRef>,
) -> JsonResult<ExperimentResults> {
    read_dataset(&authed, &user_db, &w_id, &dataset).await?;
    let mut tx = user_db.begin(&authed).await?;
    let row = sqlx::query!(
        "SELECT e.subject, e.scorers, e.created_at, e.created_by,
                (SELECT count(*) FROM eval_experiment_case c WHERE c.experiment_id = e.id)
                    AS \"case_count!\"
         FROM eval_experiment e
         WHERE e.workspace_id = $1 AND e.dataset_path = $2 AND e.id = $3",
        w_id,
        dataset,
        query.id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        Error::NotFound(format!(
            "Experiment {} not found in eval dataset {}",
            query.id, dataset
        ))
    })?;
    let case_rows = sqlx::query!(
        "SELECT case_id, name, input, expected, job_id FROM eval_experiment_case
         WHERE experiment_id = $1 ORDER BY ordinal",
        query.id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    let experiment = experiment_from_row(
        query.id,
        dataset.clone(),
        row.subject,
        row.scorers,
        row.case_count,
        row.created_at,
        row.created_by,
    )?;
    let cases = case_rows
        .into_iter()
        .map(|r| {
            Ok(ExperimentCase {
                case_id: r.case_id,
                name: r.name,
                input: serde_json::from_value(r.input)?,
                expected: opt_to_raw(r.expected)?,
                job_id: r.job_id,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // One query for every case job's own status: reading the agent step's success reported a case
    // as successful even when a scorer step had failed.
    let job_ids: Vec<Uuid> = cases.iter().map(|c| c.job_id).collect();
    let statuses = sqlx::query!(
        "SELECT id, status::text AS \"status!\" FROM v2_job_completed
         WHERE id = ANY($1) AND workspace_id = $2",
        &job_ids,
        &w_id
    )
    .fetch_all(&db)
    .await?
    .into_iter()
    .map(|r| (r.id, r.status))
    .collect::<std::collections::HashMap<_, _>>();

    // Bounded concurrency rather than one await after another: a 100-case experiment with three
    // scorers is 400 lookups, and each helper call is itself several queries.
    use futures::StreamExt;
    let scorer_count = experiment.scorers.len();
    let rows = futures::stream::iter(cases.into_iter().map(|case| {
        let db = db.clone();
        let w_id = w_id.clone();
        async move {
            let output = windmill_queue::get_result_and_success_by_id_from_flow(
                &db,
                &w_id,
                &case.job_id,
                AGENT_NODE_ID,
                None,
            )
            .await
            .ok()
            .and_then(|(r, _)| agent_answer(&r));

            // Sequential within a case, concurrent across cases: nesting a second bounded stream
            // here would multiply the two bounds into 32 in-flight queries against a
            // 50-connection pool.
            let mut scores = Vec::with_capacity(scorer_count);
            for index in 0..scorer_count {
                scores.push(
                    windmill_queue::get_result_and_success_by_id_from_flow(
                        &db,
                        &w_id,
                        &case.job_id,
                        &scorer_node_id(index),
                        None,
                    )
                    .await
                    .ok()
                    .and_then(|(r, _)| extract_score(&r)),
                );
            }

            (case, output, scores)
        }
    }))
    .buffered(8)
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .map(|(case, output, scores)| ExperimentRow {
        case_id: case.case_id,
        name: case.name,
        input: case.input,
        expected: case.expected,
        job_id: case.job_id,
        status: statuses
            .get(&case.job_id)
            .cloned()
            .unwrap_or_else(|| "running".to_string()),
        output,
        scores,
    })
    .collect::<Vec<_>>();
    let scorer_labels = experiment.scorers.iter().map(|s| s.label()).collect();
    Ok(Json(ExperimentResults { experiment, rows, scorer_labels }))
}

// -----------------------------------------------------------------------------------------------
// Capturing a case from real traffic
// -----------------------------------------------------------------------------------------------

/// A case pre-filled from something that already ran, for the user to review before saving.
/// Nothing is written here — the client posts it back to `cases/add` once it looks right.
#[derive(Serialize)]
pub struct EvalCaseDraft {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub input: EvalCaseInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_flow_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_inputs: Option<Box<RawValue>>,
    /// What the captured run actually answered, kept as the reference a rerun is compared
    /// against, and what a scorer compares a rerun to. Capture time is the only moment it
    /// exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<Box<RawValue>>,
    pub source: EvalCaseSource,
    /// The agent the capture ran against, so the drawer can preselect the subject.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_path: Option<String>,
}

/// Capture the inputs of a past AI agent run. Reads through `user_db` so a caller who cannot
/// see the job cannot lift its user message out of it.
pub async fn case_draft_from_job(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, job_id)): Path<(String, Uuid)>,
) -> JsonResult<EvalCaseDraft> {
    // `UserDB` enforces row permissions but not a token's scopes, so without this an
    // `ai_evals:read` token would read job arguments — and any attachment they name — that
    // `jobs:read` is what actually gates.
    check_scopes(&authed, || "jobs:read".to_string())?;
    let mut tx = user_db.clone().begin(&authed).await?;
    let job = sqlx::query!(
        "SELECT kind::text AS \"kind!\", args as \"args: sqlx::types::Json<serde_json::Value>\", parent_job, flow_step_id
         FROM v2_job WHERE id = $1 AND workspace_id = $2",
        job_id,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let job = job.ok_or_else(|| Error::NotFound(format!("Job {} not found", job_id)))?;
    if job.kind != "aiagent" {
        return Err(Error::BadRequest(format!(
            "Job {} is a {} job, only AI agent runs can be captured as a case",
            job_id, job.kind
        )));
    }

    let args = job.args.map(|a| a.0).unwrap_or(serde_json::Value::Null);
    let input = EvalCaseInput {
        user_message: args
            .get("user_message")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        user_attachments: args
            .get("user_attachments")
            .and_then(|v| serde_json::value::to_raw_value(v).ok()),
        messages: None,
    };

    // The agent step's own definition lives in its parent flow, which is also where the host
    // flow's `tool_inputs` are: capturing them is what lets the case reproduce the wiring the
    // run actually used rather than the agent's authored defaults.
    let mut host_flow_path = None;
    let mut agent_path = None;
    let mut tool_inputs = None;
    if let (Some(parent_job), Some(step_id)) = (job.parent_job, job.flow_step_id.as_deref()) {
        let parent = sqlx::query!(
            "SELECT runnable_path, raw_flow as \"raw_flow: sqlx::types::Json<serde_json::Value>\", runnable_id, kind::text AS \"kind!\"
             FROM v2_job WHERE id = $1 AND workspace_id = $2",
            parent_job,
            &w_id
        )
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(parent) = parent {
            // Only a path that resolves to a flow is a host flow. A preview parent may
            // carry either: the flow editor's step test uses the real flow path, while an
            // eval run uses a synthetic stamp that would fail to rerun.
            host_flow_path = match parent.runnable_path.clone() {
                Some(path) => {
                    let exists = sqlx::query_scalar!(
                        "SELECT EXISTS(SELECT 1 FROM flow WHERE workspace_id = $1 AND path = $2) AS \"exists!\"",
                        &w_id,
                        &path
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    exists.then_some(path)
                }
                None => None,
            };
            let value = match parent.raw_flow {
                Some(raw_flow) => Some(raw_flow.0),
                None => match (parent.kind.as_str(), parent.runnable_id) {
                    ("flow", Some(id)) => {
                        sqlx::query_scalar!(
                            "SELECT value AS \"value!\" FROM flow_version WHERE id = $1",
                            id
                        )
                        .fetch_optional(&mut *tx)
                        .await?
                    }
                    // A branch or loop body runs under a `flownode` parent, which carries
                    // no raw_flow: its definition lives in flow_node.
                    ("flownode", Some(id)) => {
                        sqlx::query_scalar!(
                            "SELECT flow AS \"flow!\" FROM flow_node WHERE id = $1",
                            id
                        )
                        .fetch_optional(&mut *tx)
                        .await?
                    }
                    _ => None,
                },
            };
            if let Some(value) = value {
                if let Some(module) = find_module(&value, step_id) {
                    agent_path = module
                        .get("agent")
                        .and_then(|a| a.as_str())
                        .map(|a| strip_res_prefix(a).to_string());
                    tool_inputs = module
                        .get("tool_inputs")
                        .and_then(|t| serde_json::value::to_raw_value(t).ok());
                }
            }
        }
    }
    tx.commit().await?;

    // A host flow only matters when it can actually be reapplied, which needs both the flow
    // and the link back to the agent it overrides.
    if agent_path.is_none() {
        host_flow_path = None;
        tool_inputs = None;
    }

    Ok(Json(EvalCaseDraft {
        name: input
            .user_message
            .as_deref()
            .map(|m| windmill_common::utils::truncate_with_ellipsis(m, 40)),
        input,
        host_flow_path,
        tool_inputs,
        expected: None,
        source: EvalCaseSource {
            job_id: Some(job_id),
            conversation_id: None,
            agent_path: agent_path.clone(),
            captured_at: Utc::now(),
        },
        agent_path,
    }))
}

/// Find the `aiagent` module with the given id anywhere in a flow value, including inside
/// loops, branches and the tool set of another agent.
fn find_module<'a>(
    value: &'a serde_json::Value,
    module_id: &str,
) -> Option<&'a serde_json::Map<String, serde_json::Value>> {
    match value {
        serde_json::Value::Object(map) => {
            if map.get("id").and_then(|i| i.as_str()) == Some(module_id) {
                if let Some(serde_json::Value::Object(inner)) = map.get("value") {
                    if inner.get("type").and_then(|t| t.as_str()) == Some("aiagent") {
                        return Some(inner);
                    }
                }
            }
            map.values().find_map(|v| find_module(v, module_id))
        }
        serde_json::Value::Array(items) => items.iter().find_map(|v| find_module(v, module_id)),
        _ => None,
    }
}

/// Capture a whole conversation as one case: the trailing user turn becomes the message the
/// agent is run on, and everything before it is replayed through `Memory::Manual`.
pub async fn case_draft_from_conversation(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, conversation_id)): Path<(String, Uuid)>,
) -> JsonResult<EvalCaseDraft> {
    // Returns a whole transcript, so it needs the scope that gates transcripts.
    check_scopes(&authed, || "flow_conversations:read".to_string())?;
    let mut tx = user_db.clone().begin(&authed).await?;
    let conversation = sqlx::query!(
        "SELECT flow_path, title FROM flow_conversation WHERE id = $1 AND workspace_id = $2",
        conversation_id,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let conversation = conversation
        .ok_or_else(|| Error::NotFound(format!("Conversation {} not found", conversation_id)))?;

    // Tool messages are excluded: their content is a tool result keyed to a call id that this
    // replay will not reissue, so feeding them back would desynchronise the message list.
    let rows = sqlx::query!(
        "SELECT message_type::text AS \"message_type!\", content
         FROM flow_conversation_message
         WHERE conversation_id = $1 AND message_type IN ('user', 'assistant')
         ORDER BY created_seq ASC",
        conversation_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    // The case re-asks the conversation's *last user turn*: everything before it is the
    // context to replay, and whatever the agent answered after it is what production actually
    // produced, kept as the reference to compare a rerun against. Splitting at the last user
    // turn rather than at the end is what makes a finished conversation — which ends on the
    // assistant — yield a runnable case instead of history with nothing to answer.
    let last_user = rows.iter().rposition(|r| r.message_type == "user");
    let (user_message, context, answer) = match last_user {
        Some(idx) => (
            rows[idx].content.clone(),
            &rows[..idx],
            rows.get(idx + 1).map(|r| r.content.clone()),
        ),
        None => (String::new(), &rows[..], None),
    };
    let turns = context
        .iter()
        .map(|r| serde_json::json!({ "role": r.message_type, "content": r.content }))
        .collect::<Vec<_>>();

    Ok(Json(EvalCaseDraft {
        name: conversation.title,
        input: EvalCaseInput {
            user_message: Some(user_message).filter(|m| !m.is_empty()),
            user_attachments: None,
            messages: if turns.is_empty() {
                None
            } else {
                Some(serde_json::value::to_raw_value(&turns)?)
            },
        },
        expected: answer
            .map(|a| serde_json::value::to_raw_value(&a))
            .transpose()?,
        host_flow_path: Some(conversation.flow_path),
        tool_inputs: None,
        source: EvalCaseSource {
            job_id: None,
            conversation_id: Some(conversation_id),
            agent_path: None,
            captured_at: Utc::now(),
        },
        agent_path: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(json: &str) -> Box<RawValue> {
        serde_json::from_str(json).unwrap()
    }

    /// A scorer is any runnable, so its answer arrives in whatever shape that runnable returns:
    /// a script's bare value, or an agent's answer wrapped in `output` and often stringified.
    /// A shape that goes unrecognised is a silently empty column, not an error.
    #[test]
    fn extract_score_reads_every_documented_scorer_shape() {
        assert_eq!(extract_score(&raw("0.75")), Some(0.75));
        assert_eq!(extract_score(&raw("true")), Some(1.0));
        assert_eq!(extract_score(&raw(r#"{"score": 0.5}"#)), Some(0.5));
        assert_eq!(extract_score(&raw(r#"{"score": false}"#)), Some(0.0));

        // agent scorers: the answer is under `output`
        assert_eq!(extract_score(&raw(r#"{"output": 0.25}"#)), Some(0.25));
        assert_eq!(extract_score(&raw(r#"{"output": true}"#)), Some(1.0));
        assert_eq!(extract_score(&raw(r#"{"output": "0.9"}"#)), Some(0.9));
        assert_eq!(
            extract_score(&raw(r#"{"output": {"score": 0.8}}"#)),
            Some(0.8)
        );
        assert_eq!(
            extract_score(&raw(r#"{"output": "{\"score\": 0.4}"}"#)),
            Some(0.4)
        );

        // nothing numeric to plot: left empty rather than guessed at
        assert_eq!(extract_score(&raw(r#"{"output": "not a score"}"#)), None);
        assert_eq!(extract_score(&raw(r#"{"verdict": "good"}"#)), None);
    }

    /// The stamp is what makes a run findable again, and overflowing `runnable_path`'s
    /// varchar(255) would truncate it into a path that matches nothing.
    #[test]
    fn run_path_falls_back_to_the_agent_when_the_stamp_would_overflow() {
        let case_id = Uuid::nil();
        assert_eq!(
            run_path("f/a/agent", Some("f/e/ds"), Some(case_id)),
            format!("f/a/agent/f/e/ds/{}", case_id)
        );

        let long_dataset = format!("f/e/{}", "d".repeat(240));
        assert_eq!(
            run_path("f/a/agent", Some(&long_dataset), Some(case_id)),
            "f/a/agent"
        );

        // No case to point at: the run is a one-off and stamps only its subject.
        assert_eq!(run_path("f/a/agent", None, None), "f/a/agent");
        assert_eq!(run_path("f/a/agent", Some("f/e/ds"), None), "f/a/agent");
    }
}
