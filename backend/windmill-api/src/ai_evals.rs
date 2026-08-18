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
        .route("/score", post(score_experiment))
        .route("/scorer_defaults", get(scorer_defaults))
        .route("/experiments/run", post(run_experiment))
        .route("/experiments/score_again", post(score_again))
        .route("/scorers/recent", get(recent_scorers))
        .route("/runs/score_case", post(score_case_run))
        .route("/runs/score_result", get(score_case_result))
        .route("/subject_state", get(subject_state))
        .route("/experiments/list/{*path}", get(list_experiments))
        .route("/experiments/results/{*path}", get(experiment_results))
        .route("/case_draft/from_job/{job_id}", get(case_draft_from_job))
}

/// What a run is executed against. Kept as `(kind, path, version)` rather than a bare agent
/// path so flow-scoped evaluation is a later superset instead of a rewrite.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvalSubject {
    #[serde(default = "default_subject_kind")]
    pub kind: EvalSubjectKind,
    /// The agent resource under test.
    pub path: String,
    /// The resource version at the moment the run was enqueued. Recorded, never pinned: the step
    /// resolves the agent live, as it does in production.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    /// The agent's undeployed configuration, read from its draft server-side. Present exactly
    /// when `kind` is `agent_draft`, and it is the whole definition of what ran.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft: Option<AgentDraft>,
    /// Hash of that configuration. A draft moves without the version moving, so this is the only
    /// thing that can say a run describes an agent that has since been edited. Stamped
    /// server-side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_hash: Option<String>,
}

/// Key order is not meaningful and `serde_json` preserves insertion order here, so it is sorted
/// away before hashing: the same configuration must hash the same however it was assembled.
fn canonical_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            let sorted = map
                .iter()
                .collect::<std::collections::BTreeMap<_, _>>()
                .into_iter()
                .map(|(k, v)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(k).unwrap_or_default(),
                        canonical_json(v)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{}}}", sorted)
        }
        serde_json::Value::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        other => other.to_string(),
    }
}

fn draft_hash(draft: &AgentDraft) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(canonical_json(&draft.input_transforms).as_bytes());
    hasher.update(b"|");
    hasher.update(canonical_json(&serde_json::Value::Array(draft.tools.clone())).as_bytes());
    hex::encode(hasher.finalize())[..32].to_string()
}

fn default_subject_kind() -> EvalSubjectKind {
    EvalSubjectKind::Agent
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EvalSubjectKind {
    Agent,
    /// A saved agent's undeployed draft. The value is read server-side and inlined, because a
    /// linked step resolves the resource live and so would run what the draft replaces. Its own
    /// subject, so its runs never mix with the deployed agent's history.
    AgentDraft,
}

/// The brain and tools of an agent, as the flow editor holds them.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentDraft {
    /// The agent's input transforms: provider, system prompt, output type and the rest. The
    /// message and attachments are supplied by the case and override anything named here.
    #[serde(default)]
    pub input_transforms: serde_json::Value,
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,
}

impl EvalSubject {
    /// What is recorded of a subject: enough to say what ran, without the configuration itself,
    /// which is large and already described by its hash.
    fn stamp(&self) -> EvalSubject {
        EvalSubject {
            kind: self.kind.clone(),
            path: self.path.clone(),
            version: self.version,
            draft: None,
            // Kept when there is no configuration to hash: a subject read back from a run's own
            // stamp carries the hash and not the draft, and recomputing would erase it.
            draft_hash: self
                .draft
                .as_ref()
                .map(draft_hash)
                .or_else(|| self.draft_hash.clone()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvalDataset {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The subject the pane offers by default when this dataset is opened without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_subject: Option<EvalSubject>,
    /// The columns of the results table, in display order.
    #[serde(default)]
    pub scorers: Vec<Scorer>,
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
    #[serde(default)]
    pub scorers: Vec<Scorer>,
}

#[derive(Deserialize)]
pub struct EditDataset {
    /// Renames the dataset. Its cases and experiments follow through the foreign keys, so a
    /// rename is a rename rather than a copy that leaves history behind.
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_subject: Option<EvalSubject>,
    /// Left out to keep the dataset's columns as they are; sent to replace them wholesale.
    #[serde(default)]
    pub scorers: Option<Vec<Scorer>>,
}

#[derive(Deserialize)]
pub struct CaseId {
    pub id: Uuid,
}

#[derive(Deserialize)]
/// The edit fields are spelled out rather than `#[serde(flatten)]`-ing `NewEvalCase`: flatten
/// deserializes through a buffered representation, which silently yields `None` for the
/// `Box<RawValue>` fields — an edited case would lose its attachments and tool inputs.
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

/// A case is text: a message and the answer it was expected to produce. Attachments are S3
/// references rather than inline bytes, so nothing here is meant to be large. The caps exist so that one mistake — a whole file pasted into a message, a capture
/// loop left running — cannot grow a dataset past what a listing can load.
const MAX_CASE_BYTES: usize = 256 * 1024;
const MAX_CASES_PER_DATASET: i64 = 10_000;

/// Newest first, and only this many: the list feeds a picker, and a dataset that has been run
/// nightly for a year would otherwise send back every run of it.
const MAX_EXPERIMENTS_LISTED: i64 = 100;

/// Enough to cover the scorers a workspace actually reuses, few enough to stay a list you read
/// rather than one you search.
const MAX_RECENT_SCORERS: usize = 12;

/// A saved run is assembled from case runs the user has just made; more than a dataset's worth of
/// them is not a partial rerun, it is a mistake.
const MAX_CASES_PER_RUN: usize = 1_000;

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
        "SELECT path, summary, description, default_subject, scorers, created_at, created_by,
                edited_at, edited_by
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
        scorers: serde_json::from_value(row.scorers).unwrap_or_default(),
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
        "SELECT path, summary, description, default_subject, scorers, created_at, created_by,
                edited_at, edited_by
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
                scorers: serde_json::from_value(row.scorers).unwrap_or_default(),
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
    let mut scorers = payload.scorers;
    assign_scorer_ids(&mut scorers)?;
    let scorers = serde_json::to_value(&scorers)?;
    let mut tx = user_db.begin(&authed).await?;
    // A path already taken returns no row; a path the caller may not write raises the policy
    // error `map_write_denied` translates. The two are distinct answers and must stay so.
    let created = sqlx::query_scalar!(
        "INSERT INTO eval_dataset
            (workspace_id, path, summary, description, default_subject, scorers, created_by,
             edited_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
         ON CONFLICT (workspace_id, path) DO NOTHING
         RETURNING path",
        w_id,
        payload.path,
        payload.summary,
        payload.description,
        default_subject,
        scorers,
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
    let scorers = match payload.scorers {
        Some(mut scorers) => {
            assign_scorer_ids(&mut scorers)?;
            Some(serde_json::to_value(&scorers)?)
        }
        None => None,
    };
    let new_path = match payload.path.filter(|p| *p != path) {
        Some(new_path) => {
            check_path(&new_path)?;
            Some(new_path)
        }
        None => None,
    };
    let mut tx = user_db.clone().begin(&authed).await?;
    let updated = sqlx::query_scalar!(
        "UPDATE eval_dataset
         SET path = COALESCE($8, path), summary = $3, description = $4, default_subject = $5,
             scorers = COALESCE($6, scorers), edited_at = now(), edited_by = $7
         WHERE workspace_id = $1 AND path = $2
         RETURNING path",
        w_id,
        path,
        payload.summary,
        payload.description,
        default_subject,
        scorers,
        authed.username,
        new_path.as_deref(),
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| {
        if e.as_database_error().and_then(|e| e.code()).as_deref() == Some("23505") {
            Error::BadRequest(format!(
                "Eval dataset {} already exists",
                new_path.as_deref().unwrap_or(&path)
            ))
        } else {
            e.into()
        }
    })?;
    tx.commit().await?;
    let Some(updated) = updated else {
        return Err(write_refused(&authed, &user_db, &w_id, &path).await);
    };
    Ok(format!("Updated eval dataset {}", updated))
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
/// A stored case as the fields a run needs. The two differ only by the provenance the server
/// assigns, which a run does not carry.
fn from_stored_case(case: EvalCase) -> NewEvalCase {
    NewEvalCase {
        name: case.name,
        input: case.input,
        host_flow_path: case.host_flow_path,
        tool_inputs: case.tool_inputs,
        expected: case.expected,
        tags: case.tags,
        source: case.source,
    }
}

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

/// A scorer is a column of the results table.
///
/// `id` is assigned when the scorer is added to a dataset and never reused: it is what makes a
/// column the same column across experiments when the scorer is renamed or its definition is
/// edited, and a delta is only ever computed between two scores carrying the same id.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Scorer {
    /// Assigned on write when a new scorer arrives without one, so a client cannot collide two
    /// columns onto one id.
    #[serde(default)]
    pub id: String,
    /// The column header. Defaults to the kind, or the last segment of the path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// A score at or above this counts as a pass, and the column reports a pass rate beside its
    /// mean. Deliberately outside `definition`: where the line sits is an interpretation of the
    /// score rather than part of producing it, so moving it re-reads every score already
    /// recorded instead of invalidating them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pass_if: Option<f64>,
    #[serde(flatten)]
    pub def: ScorerDef,
}

/// Two kinds, both runnables, so every column is the same sort of thing: something with a path, a
/// version, and code you can open. A judge is an `ai_agent` resource sent the run to grade; a
/// script receives the run as an argument. Both are created in one click from a template, which is
/// what keeps the choice between them about how you want to score rather than about setup cost.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScorerDef {
    Script { path: String },
    Agent { path: String },
}

impl ScorerDef {
    pub fn path(&self) -> &str {
        match self {
            ScorerDef::Script { path } | ScorerDef::Agent { path } => path,
        }
    }

    /// The wire name of the kind, as the client sends it.
    fn kind_str(&self) -> &'static str {
        match self {
            ScorerDef::Script { .. } => "script",
            ScorerDef::Agent { .. } => "agent",
        }
    }

    fn kind_label(&self) -> &'static str {
        match self {
            ScorerDef::Script { .. } => "Script",
            ScorerDef::Agent { .. } => "Judge agent",
        }
    }
}

impl Scorer {
    /// Whether a score counts as a pass. `None` when the column has no threshold, which is what
    /// keeps a column of plain numbers from being rendered as if it had one.
    pub fn passed(&self, score: Option<f64>) -> Option<bool> {
        match (self.pass_if, score) {
            (Some(threshold), Some(score)) => Some(score >= threshold),
            _ => None,
        }
    }

    /// What produced a score. Recorded with it so a comparison can say the scorer changed instead
    /// of letting that change read as a difference between two agents. `resolved` is the script
    /// hash or resource version that actually ran, which the path alone does not pin.
    pub fn definition(&self, resolved: Option<&str>) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.def.kind_label().as_bytes());
        hasher.update(b":");
        hasher.update(self.def.path().as_bytes());
        if let Some(resolved) = resolved {
            hasher.update(b"@");
            hasher.update(resolved.as_bytes());
        }
        hex::encode(hasher.finalize())[..32].to_string()
    }
}

/// Ids are assigned here rather than trusted from the client: two columns sharing one id would
/// silently merge two scorers' history into one.
fn assign_scorer_ids(scorers: &mut Vec<Scorer>) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for scorer in scorers.iter_mut() {
        if scorer.id.is_empty() || !seen.insert(scorer.id.clone()) {
            scorer.id = Uuid::new_v4().simple().to_string();
            seen.insert(scorer.id.clone());
        }
        if scorer.id.len() > 64 {
            return Err(Error::BadRequest("Scorer id is too long".to_string()));
        }
        if let Some(name) = &scorer.name {
            if name.len() > 120 {
                return Err(Error::BadRequest(format!(
                    "Scorer name {} is too long, 120 characters at most",
                    name
                )));
            }
        }
        if scorer.def.path().trim().is_empty() {
            return Err(Error::BadRequest(format!(
                "{} scorer needs a path",
                scorer.def.kind_label()
            )));
        }
    }
    Ok(())
}

/// Node id of the agent step. The answer is read back by this id, so it is part of the stored
/// shape rather than an implementation detail.
pub const AGENT_NODE_ID: &str = "a";

/// The flow one case runs as: a single agent step. Scoring is not part of it — a score is
/// produced from the answer this job stored, which is what lets a scorer added later score an
/// experiment that already ran without calling the agent again.
fn build_case_flow(
    subject: &EvalSubject,
    tool_inputs: Option<Box<RawValue>>,
) -> Result<windmill_common::flows::FlowValue> {
    // A draft runs the step exactly as authored: its own brain transforms are the module's, and
    // the case supplies the message and the attachments over the top.
    let mut input_transforms = match subject.draft.as_ref().map(|d| &d.input_transforms) {
        Some(serde_json::Value::Object(map)) => map.clone(),
        _ => serde_json::Map::new(),
    };
    for key in ["user_message", "user_attachments"] {
        input_transforms.insert(
            key.to_string(),
            serde_json::json!({ "type": "javascript", "expr": format!("flow_input.{}", key) }),
        );
    }

    let mut agent_value = serde_json::Map::new();
    agent_value.insert("type".to_string(), serde_json::json!("aiagent"));
    match &subject.draft {
        Some(draft) => {
            agent_value.insert("tools".to_string(), serde_json::json!(draft.tools));
        }
        None => {
            agent_value.insert("agent".to_string(), serde_json::json!(subject.path));
            agent_value.insert("tools".to_string(), serde_json::json!([]));
        }
    }
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

    Ok(serde_json::from_value(serde_json::json!({
        "modules": [{ "id": AGENT_NODE_ID, "value": serde_json::Value::Object(agent_value) }]
    }))?)
}

#[derive(Deserialize)]
pub struct RunEval {
    pub subject: EvalSubject,
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
    subject: &EvalSubject,
    case: &NewEvalCase,
    tool_inputs: Option<Box<RawValue>>,
    dataset: Option<&str>,
    case_id: Option<Uuid>,
    experiment_id: Option<Uuid>,
    // Chosen before anything is queued, so the record of a run cannot be missing the job it
    // names. A scratch run with nothing to record lets `push` assign one.
    job_id: Option<Uuid>,
) -> Result<Uuid> {
    use windmill_common::{jobs::JobPayload, users::username_to_permissioned_as};
    use windmill_queue::{push, PushArgs, PushIsolationLevel};

    let flow_value = build_case_flow(subject, tool_inputs)?;

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
    // The whole case input, for scoring: the message alone cannot explain an answer that came
    // from an attachment, and a scratch run has no experiment row to read it back from.
    args.insert(
        "_eval_input".to_string(),
        serde_json::value::to_raw_value(&case.input)?,
    );
    // Self-describing run: opened cold from the runs page, the job says what it was evaluating.
    // Extra flow inputs are inert — the agent step reads only user_message/user_attachments.
    args.insert(
        "_eval".to_string(),
        serde_json::value::to_raw_value(&serde_json::json!({
            "subject": subject.stamp(),
            "dataset": dataset,
            "case_id": case_id,
            "experiment_id": experiment_id,
        }))?,
    );

    let path = run_path(&subject.path, dataset, case_id);
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

/// The agent's undeployed draft, as the configuration to run it with. The draft holds the same
/// shape as the deployed resource, so its brain becomes the module's input transforms and its
/// tools the module's tools: the unlinked branch of the executor then runs exactly the value
/// sitting in the draft, which a linked step never would.
/// An `ai_agent` value as the configuration to run it with: its brain becomes the module's input
/// transforms, its tools the module's tools. The same conversion for a draft and for what is
/// deployed, so the two hash comparably — which is what lets a draft run be recognised as the
/// version it became.
fn config_to_draft(value: serde_json::Value) -> Result<AgentDraft> {
    let mut config = match value {
        serde_json::Value::Object(map) => map,
        _ => return Err(Error::BadRequest("The agent is not an object".to_string())),
    };
    let tools = match config.remove("tools") {
        Some(serde_json::Value::Array(tools)) => tools,
        _ => vec![],
    };
    // Every brain key becomes a static transform: `$res:`/`$var:` in them are resolved by the
    // same argument machinery a linked step's resource goes through.
    let input_transforms = config
        .into_iter()
        .map(|(key, value)| (key, serde_json::json!({ "type": "static", "value": value })))
        .collect::<serde_json::Map<_, _>>();
    Ok(AgentDraft { input_transforms: serde_json::Value::Object(input_transforms), tools })
}

/// What the agent hashes to as deployed. A draft run whose hash is this one ran exactly what is
/// deployed now, however it got there: that is a run of the version, not a run of a draft.
async fn deployed_agent_hash(db: &DB, w_id: &str, path: &str) -> Result<Option<String>> {
    let value = sqlx::query_scalar!(
        "SELECT value FROM resource
         WHERE workspace_id = $1 AND path = $2 AND resource_type = 'ai_agent'",
        w_id,
        path
    )
    .fetch_optional(db)
    .await?
    .flatten();
    Ok(value
        .map(config_to_draft)
        .transpose()?
        .as_ref()
        .map(draft_hash))
}

async fn agent_draft_config(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    agent_path: &str,
) -> Result<AgentDraft> {
    require_agent(authed, user_db, w_id, agent_path).await?;
    let mut tx = user_db.clone().begin(authed).await?;
    // Read through `user_db` alongside the resource itself, so a draft is only reachable to
    // someone who can read what it is a draft of.
    let value = sqlx::query_scalar!(
        "SELECT value FROM draft WHERE workspace_id = $1 AND path = $2 AND typ = 'resource'",
        w_id,
        agent_path
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let Some(value) = value else {
        return Err(Error::BadRequest(format!(
            "Agent {} has no undeployed changes to run",
            agent_path
        )));
    };
    // A resource draft wraps the value it is a draft of, alongside the path and description the
    // save form carries. The resource editor files that value under `args`; older drafts and
    // hand-written ones use `value`.
    let inner = ["args", "value"]
        .iter()
        .find_map(|key| value.get(*key).filter(|v| v.is_object()).cloned())
        .unwrap_or(value);
    config_to_draft(inner).map_err(|_| {
        Error::BadRequest(format!(
            "The draft of agent {} is not an object",
            agent_path
        ))
    })
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

#[derive(Serialize)]
pub struct RunEvalResponse {
    pub job_id: Uuid,
}

pub async fn run_eval(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(payload): Json<RunEval>,
) -> JsonResult<RunEvalResponse> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot run eval jobs".to_string(),
        ));
    }
    check_scopes(&authed, || "jobs:run".to_string())?;

    let mut subject = payload.subject;
    validate_subject(&subject)?;

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

    resolve_subject(&authed, &db, &user_db, &w_id, &mut subject).await?;

    let tool_inputs = match (&case.tool_inputs, &case.host_flow_path) {
        (Some(explicit), _) => Some(explicit.clone()),
        (None, Some(flow_path)) => {
            tool_inputs_from_host_flow(&authed, &user_db, &w_id, flow_path, &subject.path).await?
        }
        (None, None) => None,
    };

    // Nothing is recorded here. One case is run to see what it does, and what it does is a job;
    // whether it becomes part of the dataset's history is a decision taken afterwards, by saving
    // it as a run. The job carries `_eval`, which is what that save reads it back from.
    let (dataset, case_id) = (payload.dataset.as_deref(), payload.case_id);
    if let Some(dataset) = dataset {
        require_dataset_writable(&authed, &user_db, &w_id, dataset).await?;
    }

    let job_id = push_case_run(
        &authed,
        &db,
        &user_db,
        &w_id,
        &subject,
        &case,
        tool_inputs,
        dataset,
        case_id,
        None,
        None,
    )
    .await?;
    Ok(Json(RunEvalResponse { job_id }))
}

/// Fill in what the client cannot: the version a saved agent is at, or the configuration sitting
/// in its draft. Both run paths do this identically, and a subject that skipped it would run the
/// wrong definition or record no version.
async fn resolve_subject(
    authed: &ApiAuthed,
    db: &DB,
    user_db: &UserDB,
    w_id: &str,
    subject: &mut EvalSubject,
) -> Result<()> {
    match subject.kind {
        EvalSubjectKind::Agent => {
            require_agent(authed, user_db, w_id, &subject.path).await?;
            // The version the agent is at now. Recorded so the run stays attributable to a prompt
            // state later; it does not pin execution, which stays live.
            subject.version = current_resource_version(db, w_id, &subject.path).await?;
        }
        EvalSubjectKind::AgentDraft => {
            subject.draft = Some(agent_draft_config(authed, user_db, w_id, &subject.path).await?);
            // The version the draft is an edit of. It is not a version of its own, but a run
            // of "v15 plus unsaved edits" is not attributable without knowing which v15.
            subject.version = current_resource_version(db, w_id, &subject.path).await?;
        }
    }
    Ok(())
}

/// The configuration is never taken from the client: a saved agent runs by reference and its
/// draft is read from the workspace, so a request carrying one would run something other than the
/// resource it names.
fn validate_subject(subject: &EvalSubject) -> Result<()> {
    if subject.draft.is_some() {
        return Err(Error::BadRequest(
            "An agent's configuration is read from the workspace; remove it from the request"
                .to_string(),
        ));
    }
    if subject.path.trim().is_empty() {
        return Err(Error::BadRequest(
            "The subject needs a path: it is the agent a run is filed under".to_string(),
        ));
    }
    Ok(())
}

// -----------------------------------------------------------------------------------------------
// Experiments
// -----------------------------------------------------------------------------------------------

/// One run of a dataset: written once when the dataset is run, and only ever read afterwards.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvalExperiment {
    pub id: Uuid,
    pub dataset: String,
    pub subject: EvalSubject,
    /// This subject's nth run of this dataset, allocated once and never reused. What an
    /// experiment is called: "Run 7" survives history being pruned, which a position computed
    /// when the list is read would not.
    pub run_number: i32,
    /// A name for the run, beside the number it already carries. Nothing sets one yet: naming
    /// runs is not part of the surface, and this is what a future one would fill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The run whose answers this one measured again. Set exactly when nothing was run: the cells
    /// are that run's, and so is the version they are attributed to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scored_from: Option<Uuid>,
    pub case_count: i64,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

#[derive(Deserialize)]
pub struct RunExperiment {
    pub dataset: String,
    pub subject: EvalSubject,
    /// Applies one host flow's tool bindings to every case. Per-case `host_flow_path` is only
    /// honoured by a single run: one experiment runs one wiring, or its rows would not be
    /// comparable with each other.
    #[serde(default)]
    pub host_flow_path: Option<String>,
}

/// Open a run of this dataset. A run is a fixed point: it is written once and then only ever
/// read, which is what makes it worth comparing against.
///
/// Runs are numbered per agent rather than per dataset, and the deployed agent and its draft
/// share that numbering: they are the same agent, and "Run 7" of it should mean one thing whether
/// it ran the deployed value or the edits waiting on top of it.
async fn new_run(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    w_id: &str,
    dataset: &str,
    subject: &EvalSubject,
    username: &str,
    scored_from: Option<Uuid>,
) -> Result<Uuid> {
    // Two runs starting together would otherwise read the same run number. Held for the rest of
    // this transaction, which pushes no jobs.
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtext('ai_eval_open:' || $1 || '/' || $2 || '/' || $3))",
        w_id,
        dataset,
        subject.path,
    )
    .execute(&mut **tx)
    .await?;
    let run_number = sqlx::query_scalar!(
        "SELECT coalesce(max(run_number), 0) + 1 FROM eval_experiment
         WHERE workspace_id = $1 AND dataset_path = $2 AND subject ->> 'path' = $3",
        w_id,
        dataset,
        subject.path,
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(1);
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO eval_experiment
            (id, workspace_id, dataset_path, subject, run_number, created_by, scored_from)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        id,
        w_id,
        dataset,
        serde_json::to_value(subject.stamp())?,
        run_number,
        username,
        scored_from,
    )
    .execute(&mut **tx)
    .await
    .map_err(|e| {
        if is_missing_dataset(&e) {
            Error::NotFound(format!("Eval dataset {} not found", dataset))
        } else {
            e.into()
        }
    })?;
    Ok(id)
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

    let mut subject = payload.subject;
    validate_subject(&subject)?;
    resolve_subject(&authed, &db, &user_db, &w_id, &mut subject).await?;

    let cases = read_cases(&authed, &user_db, &w_id, &payload.dataset, None).await?;
    if cases.is_empty() {
        return Err(Error::BadRequest(format!(
            "Eval dataset {} has no case to run",
            payload.dataset
        )));
    }
    if cases.len() > MAX_CASES_PER_RUN {
        return Err(Error::BadRequest(format!(
            "An eval dataset of more than {} cases cannot be run at once",
            MAX_CASES_PER_RUN
        )));
    }

    let tool_inputs = match &payload.host_flow_path {
        Some(flow_path) => {
            tool_inputs_from_host_flow(&authed, &user_db, &w_id, flow_path, &subject.path).await?
        }
        None => None,
    };

    // Every job id is chosen here, and the whole experiment is recorded before anything is
    // queued. A launch that dies partway therefore leaves a recorded case whose job is missing —
    // which the results table shows — rather than a running job that no experiment accounts for
    // and nothing will ever collect.
    let job_ids = cases.iter().map(|_| Uuid::new_v4()).collect::<Vec<_>>();
    let case_count = cases.len();

    let mut tx = db.begin().await?;
    let experiment_id =
        new_run(&mut tx, &w_id, &payload.dataset, &subject, &authed.username, None).await?;

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
    let versions = vec![subject.version; case_count];
    let hashes = vec![subject.draft.as_ref().map(draft_hash); case_count];
    sqlx::query!(
        "INSERT INTO eval_experiment_case
            (experiment_id, ordinal, case_id, name, input, expected, job_id, subject_version,
             subject_draft_hash)
         SELECT $1, ordinal, case_id, name, input, expected, job_id, subject_version,
                subject_draft_hash
         FROM UNNEST($2::int[], $3::uuid[], $4::text[], $5::jsonb[], $6::jsonb[], $7::uuid[],
                     $8::bigint[], $9::text[])
              AS t(ordinal, case_id, name, input, expected, job_id, subject_version,
                   subject_draft_hash)",
        experiment_id,
        &ordinals,
        &case_ids,
        &names as &[Option<String>],
        &inputs,
        &expecteds as &[Option<serde_json::Value>],
        &job_ids,
        &versions as &[Option<i64>],
        &hashes as &[Option<String>],
    )
    .execute(&mut *tx)
    .await?;
    // The foreign key is the guard against a dataset deleted while this was being assembled: the
    // commit fails and no job has been queued yet.
    tx.commit().await?;

    let mut launched = 0usize;
    let mut push_error: Option<Error> = None;
    for (case, job_id) in cases.into_iter().zip(job_ids.iter().copied()) {
        let case_id = case.id;
        let new_case = from_stored_case(case);
        if let Err(e) = push_case_run(
            &authed,
            &db,
            &user_db,
            &w_id,
            &subject,
            &new_case,
            tool_inputs.clone(),
            Some(&payload.dataset),
            Some(case_id),
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

#[derive(Deserialize)]
pub struct ScoreAgain {
    pub dataset: String,
    /// The run whose answers are measured again.
    pub experiment_id: Uuid,
}

/// Open a run that reuses another run's answers.
///
/// Scoring is separate from running, so a scorer edited or added after a run should be able to
/// measure what that run already answered. A run is permanent, so it cannot be measured in place:
/// this makes a run of its own, holding the same answers and one new set of scores. Nothing calls
/// the agent.
///
/// The answers and their provenance are copied whole rather than mixed, which is what makes the
/// new run readable: every cell of it was produced by the version the parent recorded, so it is
/// attributed to that version and goes stale against the current agent exactly as the parent does.
/// Scoring it is the caller's next step, on the same route any run is scored by.
pub async fn score_again(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(payload): Json<ScoreAgain>,
) -> JsonResult<Uuid> {
    check_scopes(&authed, || "jobs:run".to_string())?;
    // A write: it persists a run into the dataset.
    require_dataset_writable(&authed, &user_db, &w_id, &payload.dataset).await?;
    // Reading it validates that it is a run of this dataset, and 404s rather than copying from one
    // the caller made up.
    let parent = read_experiment(&db, &w_id, &payload.dataset, payload.experiment_id).await?;

    let mut tx = db.begin().await?;
    let id = new_run(
        &mut tx,
        &w_id,
        &payload.dataset,
        &parent.subject,
        &authed.username,
        Some(payload.experiment_id),
    )
    .await?;
    let copied = sqlx::query_scalar!(
        "INSERT INTO eval_experiment_case
            (experiment_id, ordinal, case_id, name, input, expected, job_id, subject_version,
             subject_draft_hash, started_at)
         SELECT $1, ordinal, case_id, name, input, expected, job_id, subject_version,
                subject_draft_hash, started_at
         FROM eval_experiment_case WHERE experiment_id = $2
         RETURNING ordinal",
        id,
        payload.experiment_id,
    )
    .fetch_all(&mut *tx)
    .await?;
    if copied.is_empty() {
        return Err(Error::BadRequest(format!(
            "Run {} has no result to measure again",
            payload.experiment_id
        )));
    }
    tx.commit().await?;
    Ok(Json(id))
}

fn experiment_from_row(
    id: Uuid,
    dataset: String,
    subject: serde_json::Value,
    run_number: i32,
    label: Option<String>,
    scored_from: Option<Uuid>,
    case_count: i64,
    created_at: DateTime<Utc>,
    created_by: String,
) -> Result<EvalExperiment> {
    Ok(EvalExperiment {
        id,
        dataset,
        subject: serde_json::from_value(subject)?,
        run_number,
        label,
        scored_from,
        case_count,
        created_at,
        created_by,
    })
}

#[derive(Deserialize)]
pub struct ListExperimentsQuery {
    /// Restrict to one agent's runs, which is what a pane opened on an agent shows: the dataset
    /// may have been run against several, and another agent's numbers are not this agent's
    /// history. Both what was deployed and what was drafted are that agent's history, so this
    /// does not discriminate by kind.
    #[serde(default)]
    pub subject_path: Option<String>,
}

pub async fn list_experiments(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, dataset)): Path<(String, String)>,
    Query(query): Query<ListExperimentsQuery>,
) -> JsonResult<Vec<EvalExperiment>> {
    read_dataset(&authed, &user_db, &w_id, &dataset).await?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query!(
        "SELECT e.id, e.subject, e.run_number, e.label, e.scored_from, e.created_at, e.created_by,
                (SELECT count(*) FROM eval_experiment_case c WHERE c.experiment_id = e.id)
                    AS \"case_count!\"
         FROM eval_experiment e
         WHERE e.workspace_id = $1 AND e.dataset_path = $2
               AND ($4::text IS NULL OR e.subject ->> 'path' = $4)
         ORDER BY e.created_at DESC
         LIMIT $3",
        w_id,
        dataset,
        MAX_EXPERIMENTS_LISTED,
        query.subject_path,
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
                row.run_number,
                row.label,
                row.scored_from,
                row.case_count,
                row.created_at,
                row.created_by,
            )
        })
        .collect::<Result<Vec<_>>>()
        .map(Json)
}

#[derive(Serialize)]
pub struct RecentScorer {
    #[serde(flatten)]
    pub scorer: Scorer,
    /// The dataset it is a column of, which is where the user last saw it.
    pub dataset: String,
}

#[derive(Deserialize)]
pub struct RecentScorersQuery {
    /// Only scorers of this kind, which is the one the add form was opened for.
    #[serde(default)]
    pub kind: Option<String>,
}

/// The scorers already in use in this workspace, most recently edited dataset first. A new
/// dataset starts with no columns, and retyping the path of the judge you already have is the
/// sort of work that makes people not bother.
///
/// Two filters, both of them real: the datasets are read through `user_db`, so a scorer only
/// appears if its dataset does, and the runnable itself is checked the same way, so the list is
/// scorers the caller could actually run rather than paths they cannot open.
pub async fn recent_scorers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<RecentScorersQuery>,
) -> JsonResult<Vec<RecentScorer>> {
    let mut tx = user_db.begin(&authed).await?;
    let datasets = sqlx::query!(
        "SELECT path, scorers FROM eval_dataset
         WHERE workspace_id = $1 ORDER BY edited_at DESC LIMIT 100",
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;

    let mut seen = std::collections::HashSet::new();
    let mut recent: Vec<RecentScorer> = vec![];
    for row in datasets {
        let scorers: Vec<Scorer> = serde_json::from_value(row.scorers).unwrap_or_default();
        for scorer in scorers {
            if query
                .kind
                .as_deref()
                .is_some_and(|kind| kind != scorer.def.kind_str())
            {
                continue;
            }
            let key = (scorer.def.kind_str(), scorer.def.path().to_string());
            if seen.insert(key) {
                recent.push(RecentScorer { scorer, dataset: row.path.clone() });
            }
        }
    }
    recent.truncate(MAX_RECENT_SCORERS);

    let script_paths = recent
        .iter()
        .filter(|r| matches!(r.scorer.def, ScorerDef::Script { .. }))
        .map(|r| r.scorer.def.path().to_string())
        .collect::<Vec<_>>();
    let agent_paths = recent
        .iter()
        .filter(|r| matches!(r.scorer.def, ScorerDef::Agent { .. }))
        .map(|r| r.scorer.def.path().to_string())
        .collect::<Vec<_>>();
    let readable_scripts = sqlx::query_scalar!(
        "SELECT path FROM script WHERE workspace_id = $1 AND path = ANY($2) AND deleted = false",
        w_id,
        &script_paths
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect::<std::collections::HashSet<_>>();
    let readable_agents = sqlx::query_scalar!(
        "SELECT path FROM resource WHERE workspace_id = $1 AND path = ANY($2)",
        w_id,
        &agent_paths
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect::<std::collections::HashSet<_>>();
    tx.commit().await?;

    recent.retain(|r| match &r.scorer.def {
        ScorerDef::Script { path } => readable_scripts.contains(path),
        ScorerDef::Agent { path } => readable_agents.contains(path),
    });
    Ok(Json(recent))
}

#[derive(Deserialize)]
pub struct ScoreCaseRun {
    pub dataset: String,
    pub case_id: Uuid,
    /// The case run to score, which is not recorded anywhere yet.
    pub job_id: Uuid,
}

/// Score a case run that is not part of any run yet. Rerunning one case is worth looking at only
/// with its numbers beside it, and waiting for a save to see them would mean deciding blind — so
/// the scorers run here, and the scoring jobs are carried into the run when it is saved rather
/// than recomputed, because a judge asked twice does not answer twice the same.
pub async fn score_case_run(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(payload): Json<ScoreCaseRun>,
) -> JsonResult<Uuid> {
    check_scopes(&authed, || "jobs:run".to_string())?;
    let dataset = read_dataset(&authed, &user_db, &w_id, &payload.dataset).await?;
    let scorers = dataset.scorers.iter().collect::<Vec<_>>();
    if scorers.is_empty() {
        return Err(Error::BadRequest(format!(
            "Eval dataset {} has no scorer",
            payload.dataset
        )));
    }
    let case = read_case(&authed, &user_db, &w_id, &payload.dataset, payload.case_id).await?;

    let completed = sqlx::query!(
        "SELECT status::text AS \"status!\", duration_ms FROM v2_job_completed
         WHERE id = $1 AND workspace_id = $2",
        payload.job_id,
        w_id
    )
    .fetch_optional(&db)
    .await?
    .ok_or_else(|| Error::BadRequest("That run has not finished".to_string()))?;
    if completed.status != "success" {
        return Err(Error::BadRequest(
            "That run produced no answer to score".to_string(),
        ));
    }

    let run = build_run_payload(
        &db,
        &w_id,
        payload.job_id,
        case.input,
        case.expected,
        completed.status,
        Some(completed.duration_ms),
    )
    .await?;
    let mut args = std::collections::HashMap::new();
    args.insert("run".to_string(), serde_json::value::to_raw_value(&run)?);
    args.insert(
        "rendered".to_string(),
        serde_json::value::to_raw_value(&render_run(&run))?,
    );
    let job_id = push_scoring_job(
        &authed,
        &db,
        &user_db,
        &w_id,
        &payload.dataset,
        &scorers,
        args,
    )
    .await?;
    Ok(Json(job_id))
}

#[derive(Deserialize)]
pub struct ScoreResultQuery {
    pub dataset: String,
    pub job_id: Uuid,
}

#[derive(Serialize)]
pub struct TrialScore {
    pub scorer_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checks: Option<Box<RawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The scoring job has not finished, or its result is not readable yet.
    pub pending: bool,
}

/// The verdicts of a scoring job on a trial run. Computed on read and stored nowhere: a trial
/// belongs to no run, so there is no row to store it in.
pub async fn score_case_result(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<ScoreResultQuery>,
) -> JsonResult<Vec<TrialScore>> {
    let dataset = read_dataset(&authed, &user_db, &w_id, &query.dataset).await?;
    let status = sqlx::query_scalar!(
        "SELECT status::text FROM v2_job_completed WHERE id = $1 AND workspace_id = $2",
        query.job_id,
        w_id
    )
    .fetch_optional(&db)
    .await?
    .flatten();

    let mut scores = vec![];
    for scorer in &dataset.scorers {
        if status.is_none() {
            scores.push(TrialScore {
                scorer_id: scorer.id.clone(),
                score: None,
                reason: None,
                checks: None,
                error: None,
                pending: true,
            });
            continue;
        }
        match read_verdict(&db, &w_id, query.job_id, &scorer.id, status.as_deref()).await {
            Some((score, reason, checks, error)) => scores.push(TrialScore {
                scorer_id: scorer.id.clone(),
                score,
                reason,
                checks: checks
                    .map(|c| serde_json::value::to_raw_value(&c))
                    .transpose()?,
                error,
                pending: false,
            }),
            None => scores.push(TrialScore {
                scorer_id: scorer.id.clone(),
                score: None,
                reason: None,
                checks: None,
                error: None,
                pending: true,
            }),
        }
    }
    Ok(Json(scores))
}

#[derive(Deserialize)]
pub struct SubjectStateQuery {
    pub path: String,
}

#[derive(Serialize)]
pub struct SubjectState {
    /// The version the agent is on now.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    /// What its draft hashes to now, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_hash: Option<String>,
    pub has_undeployed_changes: bool,
}

/// What the agent is right now: the version it is deployed at, and what its draft hashes to.
///
/// Small on purpose. The results endpoint reports the same thing, but it harvests scores and
/// reads every job to do it, so it is not something to poll — and without polling, an agent
/// edited while the table is open goes on looking current until the pane is reopened.
pub async fn subject_state(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<SubjectStateQuery>,
) -> JsonResult<SubjectState> {
    // Reading the agent through `user_db` is what gates the rest: a draft's existence is
    // information about the resource it is a draft of.
    require_agent(&authed, &user_db, &w_id, &query.path).await?;
    let version = current_resource_version(&db, &w_id, &query.path).await?;
    let draft = agent_draft_config(&authed, &user_db, &w_id, &query.path)
        .await
        .ok();
    Ok(Json(SubjectState {
        version,
        draft_hash: draft.as_ref().map(draft_hash),
        has_undeployed_changes: draft.is_some(),
    }))
}

// -----------------------------------------------------------------------------------------------
// Scoring
// -----------------------------------------------------------------------------------------------

/// What every scorer is handed, whether it is a judge prompt, a script or a flow. An agent is
/// judged on its behaviour, so the final answer is the smaller half of the evidence: the calls it
/// made, with their arguments and results, are the rest.
///
/// Built from the job the run already stored, which is what lets a scorer added later score an
/// experiment that has already run.
#[derive(Serialize, Debug, Clone)]
pub struct EvalRunPayload {
    pub input: EvalCaseInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Box<RawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<Box<RawValue>>,
    pub tool_calls: Vec<EvalToolCall>,
    /// The tools that were actually called, with the schema they were called against. A tool
    /// whose schema could not be resolved carries `null`, and a scorer validating arguments must
    /// treat that as unchecked rather than as a failure.
    pub tools: Vec<EvalToolDef>,
    pub metrics: EvalMetrics,
    pub status: String,
    pub job_id: Uuid,
}

#[derive(Serialize, Debug, Clone)]
pub struct EvalToolCall {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Box<RawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Box<RawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
    /// Set when the result was too large to carry and was cut down.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct EvalToolDef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Box<RawValue>>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EvalMetrics {
    pub steps: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
    /// The provider's token counts, when it reported any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Box<RawValue>>,
}

/// A tool result large enough to swamp a judge's context is cut here. The scorer is told, so a
/// check reading a truncated result can say so instead of failing on the missing tail.
const MAX_TOOL_RESULT_BYTES: usize = 4 * 1024;

fn truncate_value(value: Box<RawValue>) -> (Box<RawValue>, bool) {
    if value.get().len() <= MAX_TOOL_RESULT_BYTES {
        return (value, false);
    }
    let cut: String = value.get().chars().take(MAX_TOOL_RESULT_BYTES).collect();
    match serde_json::value::to_raw_value(&format!("{}… [truncated]", cut)) {
        Ok(v) => (v, true),
        Err(_) => (value, false),
    }
}

/// Assemble the payload from a completed case job: the agent step's own result carries the answer
/// and the message list, and every message that made a tool call names the job that ran it.
async fn build_run_payload(
    db: &DB,
    w_id: &str,
    job_id: Uuid,
    input: EvalCaseInput,
    expected: Option<Box<RawValue>>,
    status: String,
    duration_ms: Option<i64>,
) -> Result<EvalRunPayload> {
    let agent_result = windmill_queue::get_result_and_success_by_id_from_flow(
        db,
        w_id,
        &job_id,
        AGENT_NODE_ID,
        None,
    )
    .await
    .ok()
    .map(|(r, _)| r);

    let parsed: Option<serde_json::Value> = agent_result
        .as_ref()
        .and_then(|r| serde_json::from_str(r.get()).ok());
    let output = parsed
        .as_ref()
        .and_then(|p| p.get("output"))
        .map(|o| serde_json::value::to_raw_value(o))
        .transpose()?;
    let usage = parsed
        .as_ref()
        .and_then(|p| p.get("usage"))
        .map(|u| serde_json::value::to_raw_value(u))
        .transpose()?;

    // Walk the messages in order: a tool call is an `agent_action` on the message that made it.
    let mut calls: Vec<(String, Option<Uuid>, Option<Box<RawValue>>)> = vec![];
    if let Some(messages) = parsed
        .as_ref()
        .and_then(|p| p.get("messages"))
        .and_then(|m| m.as_array())
    {
        for message in messages {
            let Some(action) = message.get("agent_action") else {
                continue;
            };
            match action.get("type").and_then(|t| t.as_str()) {
                Some("tool_call") => calls.push((
                    action
                        .get("function_name")
                        .and_then(|f| f.as_str())
                        .unwrap_or("tool")
                        .to_string(),
                    action
                        .get("job_id")
                        .and_then(|j| j.as_str())
                        .and_then(|j| Uuid::parse_str(j).ok()),
                    None,
                )),
                // An MCP call runs inside the agent rather than as a job, so its arguments are on
                // the action itself and there is no result to read back.
                Some("mcp_tool_call") => calls.push((
                    action
                        .get("function_name")
                        .and_then(|f| f.as_str())
                        .unwrap_or("tool")
                        .to_string(),
                    None,
                    action
                        .get("arguments")
                        .map(|a| serde_json::value::to_raw_value(a))
                        .transpose()?,
                )),
                _ => {}
            }
        }
    }

    let call_job_ids: Vec<Uuid> = calls.iter().filter_map(|(_, id, _)| *id).collect();
    let mut jobs = std::collections::HashMap::new();
    if !call_job_ids.is_empty() {
        // The schema comes from the script the call actually ran, which is the only version a
        // check on its arguments may fairly be made against.
        let rows = sqlx::query!(
            "SELECT j.id, j.args AS \"args: sqlx::types::Json<Box<RawValue>>\",
                    c.result AS \"result: sqlx::types::Json<Box<RawValue>>\",
                    c.status::text AS status, c.duration_ms,
                    s.schema AS \"schema: sqlx::types::Json<Box<RawValue>>\"
             FROM v2_job j
             LEFT JOIN v2_job_completed c ON c.id = j.id
             LEFT JOIN script s ON s.workspace_id = j.workspace_id AND s.hash = j.runnable_id
             WHERE j.id = ANY($1) AND j.workspace_id = $2",
            &call_job_ids,
            w_id
        )
        .fetch_all(db)
        .await?;
        for row in rows {
            jobs.insert(row.id, row);
        }
    }

    let mut tool_calls = Vec::with_capacity(calls.len());
    let mut tools: Vec<EvalToolDef> = vec![];
    for (name, call_job_id, inline_args) in calls {
        let row = call_job_id.and_then(|id| jobs.get(&id));
        let (result, truncated) = match row.and_then(|r| r.result.as_ref()) {
            Some(result) => {
                let (value, truncated) = truncate_value(result.0.clone());
                (Some(value), truncated)
            }
            None => (None, false),
        };
        let failed = row
            .and_then(|r| r.status.as_deref())
            .map(|s| s != "success")
            .unwrap_or(false);
        if !tools.iter().any(|t| t.name == name) {
            tools.push(EvalToolDef {
                name: name.clone(),
                schema: row.and_then(|r| r.schema.as_ref()).map(|s| s.0.clone()),
            });
        }
        tool_calls.push(EvalToolCall {
            name,
            args: inline_args.or_else(|| row.and_then(|r| r.args.as_ref()).map(|a| a.0.clone())),
            result,
            error: failed
                .then(|| {
                    row.and_then(|r| r.result.as_ref())
                        .map(|r| r.0.get().to_string())
                })
                .flatten(),
            duration_ms: row.map(|r| r.duration_ms),
            truncated,
        });
    }

    Ok(EvalRunPayload {
        metrics: EvalMetrics { steps: tool_calls.len(), duration_ms, usage },
        input,
        output,
        expected,
        tool_calls,
        tools,
        status,
        job_id,
    })
}

/// The system prompt a judge agent is created with. It is the agent's own, so editing a judge is
/// editing that resource — there is no second copy of the grading contract on the dataset.
pub const JUDGE_SYSTEM_PROMPT: &str = r#"You are grading one run of an AI agent.

Score how well the agent handled the request, from 0 to 1. Judge the whole trajectory, not only the
final answer. Penalise asking for information already in the request, calling a tool twice with the
same arguments, and tool errors left unrecovered.

Reply with JSON only, of the form {"score": <number between 0 and 1>, "reason": <one sentence>}."#;

fn render_json(value: Option<&RawValue>) -> String {
    value
        .map(|v| v.get().to_string())
        .unwrap_or_else(|| "(none)".to_string())
}

/// Tool calls as the judge reads them: numbered, in order, with arguments, result and duration.
fn render_tool_calls(calls: &[EvalToolCall]) -> String {
    if calls.is_empty() {
        return "(none)".to_string();
    }
    calls
        .iter()
        .enumerate()
        .map(|(index, call)| {
            let args = call.args.as_ref().map(|a| a.get()).unwrap_or("{}");
            let outcome = match (&call.error, &call.result) {
                (Some(error), _) => format!("error: {}", error),
                (None, Some(result)) => result.get().to_string(),
                (None, None) => "(no result)".to_string(),
            };
            let timing = call
                .duration_ms
                .map(|ms| format!(" ({}ms)", ms))
                .unwrap_or_default();
            format!(
                "{}. {}({}) -> {}{}",
                index + 1,
                call.name,
                args,
                outcome,
                timing
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// One run, as a judge is shown it.
fn render_run(run: &EvalRunPayload) -> String {
    format!(
        "Request: {}\nTool calls, in order:\n{}\nAnswer: {}\nExpected: {}",
        run.input.user_message.as_deref().unwrap_or("(none)"),
        render_tool_calls(&run.tool_calls),
        render_json(run.output.as_deref()),
        render_json(run.expected.as_deref()),
    )
}

/// Module id of a scorer inside a scoring job. Scorer ids are hex, so this is a valid identifier.
fn scorer_module_id(scorer_id: &str) -> String {
    format!("s_{}", scorer_id)
}

/// One job per run, one module per scorer being scored. Every module reads the same `run` from the
/// flow input, so a judge and a script see exactly the same evidence.
fn build_scoring_flow(scorers: &[&Scorer]) -> Result<windmill_common::flows::FlowValue> {
    let mut modules = vec![];
    for scorer in scorers {
        let value = match &scorer.def {
            // A judge is an agent handed the run as its message; its own system prompt is the
            // grading contract, which is why editing a judge means editing that agent.
            ScorerDef::Agent { path } => serde_json::json!({
                "type": "aiagent",
                "agent": path,
                "tools": [],
                "input_transforms": {
                    "user_message": { "type": "javascript", "expr": "flow_input.rendered" },
                }
            }),
            // `run` is the whole payload; `input`, `output` and `expected` are the same values
            // spelled out, so a three-line scorer does not have to reach into it.
            ScorerDef::Script { path } => serde_json::json!({
                "type": "script",
                "path": path,
                "input_transforms": {
                    "run": { "type": "javascript", "expr": "flow_input.run" },
                    "input": { "type": "javascript", "expr": "flow_input.run.input" },
                    "output": { "type": "javascript", "expr": "flow_input.run.output" },
                    "expected": { "type": "javascript", "expr": "flow_input.run.expected" },
                }
            }),
        };
        modules.push(serde_json::json!({ "id": scorer_module_id(&scorer.id), "value": value }));
    }
    Ok(serde_json::from_value(
        serde_json::json!({ "modules": modules }),
    )?)
}

/// The version of a scorer's runnable that would run now. Part of what a score records, so a
/// script edited between two experiments is visible as a change of scorer rather than of agent.
async fn resolve_definition(db: &DB, w_id: &str, scorer: &Scorer) -> Result<String> {
    let resolved = match &scorer.def {
        ScorerDef::Script { path } => sqlx::query_scalar!(
            "SELECT hash FROM script WHERE workspace_id = $1 AND path = $2 AND deleted = false
             ORDER BY created_at DESC LIMIT 1",
            w_id,
            path
        )
        .fetch_optional(db)
        .await?
        .map(|h| h.to_string()),
        ScorerDef::Agent { path } => current_resource_version(db, w_id, path)
            .await?
            .map(|v| v.to_string()),
    };
    Ok(scorer.definition(resolved.as_deref()))
}

#[derive(Deserialize)]
pub struct ScoreRequest {
    pub dataset: String,
    pub experiment_id: Uuid,
    /// The columns to score. All of the dataset's scorers when absent.
    #[serde(default)]
    pub scorer_ids: Option<Vec<String>>,
    /// The cases to score. The whole experiment when absent.
    #[serde(default)]
    pub case_ids: Option<Vec<Uuid>>,
    /// Score cells that already carry a score from the same definition. Off by default, so
    /// scoring an experiment twice costs nothing the second time.
    #[serde(default)]
    pub force: bool,
}

#[derive(Serialize)]
pub struct ScoreResponse {
    /// Cells settled in the API, with no job.
    pub scored: usize,
    /// Scoring jobs queued.
    pub jobs: usize,
    /// Cells already carrying a score from the current definition.
    pub skipped: usize,
    /// Cells whose run has not finished, or did not produce an answer to score.
    pub unscorable: usize,
}

/// Score an experiment from the answers it already stored: one run, one column, the whole
/// experiment, or a column across history are the same operation at different grains. The agent is
/// never called.
pub async fn score_experiment(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(payload): Json<ScoreRequest>,
) -> JsonResult<ScoreResponse> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot score eval runs".to_string(),
        ));
    }
    check_scopes(&authed, || "jobs:run".to_string())?;
    require_dataset_writable(&authed, &user_db, &w_id, &payload.dataset).await?;

    let dataset = read_dataset(&authed, &user_db, &w_id, &payload.dataset).await?;
    let scorers: Vec<Scorer> = match &payload.scorer_ids {
        Some(ids) => {
            for id in ids {
                if !dataset.scorers.iter().any(|s| &s.id == id) {
                    return Err(Error::NotFound(format!(
                        "Eval dataset {} has no scorer {}",
                        payload.dataset, id
                    )));
                }
            }
            dataset
                .scorers
                .iter()
                .filter(|s| ids.contains(&s.id))
                .cloned()
                .collect()
        }
        None => dataset.scorers.clone(),
    };
    if scorers.is_empty() {
        return Err(Error::BadRequest(format!(
            "Eval dataset {} has no scorer to score with",
            payload.dataset
        )));
    }

    let mut definitions = std::collections::HashMap::new();
    for scorer in &scorers {
        definitions.insert(
            scorer.id.clone(),
            resolve_definition(&db, &w_id, scorer).await?,
        );
    }

    // The cells, and what is already known about them. Read on the unrestricted pool: the caller's
    // access was established by the dataset read above, and no id here is caller-supplied.
    let cells = sqlx::query!(
        "SELECT c.ordinal, c.case_id, c.input, c.expected, c.job_id,
                j.status::text AS status, j.duration_ms
         FROM eval_experiment_case c
         LEFT JOIN v2_job_completed j ON j.id = c.job_id AND j.workspace_id = $2
         JOIN eval_experiment e ON e.id = c.experiment_id
         WHERE c.experiment_id = $1 AND e.workspace_id = $2 AND e.dataset_path = $3
         ORDER BY c.ordinal",
        payload.experiment_id,
        w_id,
        payload.dataset,
    )
    .fetch_all(&db)
    .await?;
    if cells.is_empty() {
        return Err(Error::NotFound(format!(
            "Experiment {} not found in eval dataset {}",
            payload.experiment_id, payload.dataset
        )));
    }

    let existing = sqlx::query!(
        "SELECT ordinal, scorer_id, definition, score, job_id FROM eval_score
         WHERE experiment_id = $1",
        payload.experiment_id
    )
    .fetch_all(&db)
    .await?
    .into_iter()
    .map(|r| ((r.ordinal, r.scorer_id), (r.definition, r.score, r.job_id)))
    .collect::<std::collections::HashMap<_, _>>();

    let mut response = ScoreResponse { scored: 0, jobs: 0, skipped: 0, unscorable: 0 };

    for cell in cells {
        if let Some(case_ids) = &payload.case_ids {
            if !case_ids.contains(&cell.case_id) {
                continue;
            }
        }
        let wanted: Vec<&Scorer> = scorers
            .iter()
            .filter(|scorer| {
                if payload.force {
                    return true;
                }
                match existing.get(&(cell.ordinal, scorer.id.clone())) {
                    // A pending job for the current definition is already the answer to this.
                    Some((definition, score, job_id)) => {
                        definition != &definitions[&scorer.id]
                            || (score.is_none() && job_id.is_none())
                    }
                    None => true,
                }
            })
            .collect();
        response.skipped += scorers.len() - wanted.len();
        if wanted.is_empty() {
            continue;
        }
        // Only a finished run has an answer to score. An unfinished or failed one is left alone
        // rather than scored zero: the table already shows what happened to it.
        if cell.status.as_deref() != Some("success") {
            response.unscorable += wanted.len();
            continue;
        }

        let expected = opt_to_raw(cell.expected)?;
        let run = build_run_payload(
            &db,
            &w_id,
            cell.job_id,
            serde_json::from_value(cell.input)?,
            expected,
            cell.status.clone().unwrap_or_else(|| "success".to_string()),
            Some(cell.duration_ms),
        )
        .await?;

        let mut args = std::collections::HashMap::new();
        args.insert("run".to_string(), serde_json::value::to_raw_value(&run)?);
        // The run as a judge reads it. Rendered once per run rather than per scorer: what a judge
        // is shown is the run, and what it is asked lives in its own system prompt.
        args.insert(
            "rendered".to_string(),
            serde_json::value::to_raw_value(&render_run(&run))?,
        );

        let job_id = push_scoring_job(
            &authed,
            &db,
            &user_db,
            &w_id,
            &payload.dataset,
            &wanted,
            args,
        )
        .await?;
        for scorer in wanted {
            upsert_score(
                &db,
                payload.experiment_id,
                cell.ordinal,
                &scorer.id,
                &definitions[&scorer.id],
                None,
                None,
                None,
                Some(job_id),
            )
            .await?;
        }
        response.jobs += 1;
    }

    Ok(Json(response))
}

async fn upsert_score(
    db: &DB,
    experiment_id: Uuid,
    ordinal: i32,
    scorer_id: &str,
    definition: &str,
    score: Option<f64>,
    reason: Option<String>,
    error: Option<String>,
    job_id: Option<Uuid>,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO eval_score
            (experiment_id, ordinal, scorer_id, score, reason, error, definition, job_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (experiment_id, ordinal, scorer_id) DO UPDATE
            SET score = EXCLUDED.score, reason = EXCLUDED.reason, error = EXCLUDED.error,
                checks = NULL, definition = EXCLUDED.definition, job_id = EXCLUDED.job_id,
                created_at = now()",
        experiment_id,
        ordinal,
        scorer_id,
        score,
        reason,
        error,
        definition,
        job_id,
    )
    .execute(db)
    .await?;
    Ok(())
}

async fn push_scoring_job(
    authed: &ApiAuthed,
    db: &DB,
    user_db: &UserDB,
    w_id: &str,
    dataset: &str,
    scorers: &[&Scorer],
    args: std::collections::HashMap<String, Box<RawValue>>,
) -> Result<Uuid> {
    use windmill_common::{jobs::JobPayload, users::username_to_permissioned_as};
    use windmill_queue::{push, PushArgs, PushIsolationLevel};

    let flow_value = build_scoring_flow(scorers)?;
    let path = {
        let full = format!("{}/scoring", dataset);
        if full.len() <= MAX_RUNNABLE_PATH {
            full
        } else {
            dataset.to_string()
        }
    };
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
        None,
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

/// Read finished scoring jobs into the scores they produced. Called before results are served, so
/// a score outlives the job that produced it and the retention on that job.
async fn harvest_scores(db: &DB, w_id: &str, experiment_id: Uuid) -> Result<()> {
    let pending = sqlx::query!(
        "SELECT s.ordinal, s.scorer_id, s.job_id AS \"job_id!\", j.status::text AS status
         FROM eval_score s
         JOIN v2_job_completed j ON j.id = s.job_id AND j.workspace_id = $2
         WHERE s.experiment_id = $1 AND s.job_id IS NOT NULL AND s.score IS NULL
               AND s.error IS NULL",
        experiment_id,
        w_id
    )
    .fetch_all(db)
    .await?;
    for row in pending {
        let Some((score, reason, checks, error)) =
            read_verdict(db, w_id, row.job_id, &row.scorer_id, row.status.as_deref()).await
        else {
            continue;
        };
        sqlx::query!(
            "UPDATE eval_score SET score = $4, reason = $5, checks = $6, error = $7
             WHERE experiment_id = $1 AND ordinal = $2 AND scorer_id = $3",
            experiment_id,
            row.ordinal,
            row.scorer_id,
            score,
            reason,
            checks,
            error,
        )
        .execute(db)
        .await?;
    }
    Ok(())
}

/// One scorer's verdict, read out of the scoring job that produced it. `None` while the job has
/// finished but this module's result is not readable yet, which is a state to wait through rather
/// than to record as a failure.
async fn read_verdict(
    db: &DB,
    w_id: &str,
    scoring_job: Uuid,
    scorer_id: &str,
    job_status: Option<&str>,
) -> Option<(
    Option<f64>,
    Option<String>,
    Option<serde_json::Value>,
    Option<String>,
)> {
    let module = scorer_module_id(scorer_id);
    let result = windmill_queue::get_result_and_success_by_id_from_flow(
        db,
        w_id,
        &scoring_job,
        &module,
        None,
    )
    .await
    .ok();
    Some(match result {
        Some((value, _)) => {
            let (score, reason, checks) = extract_verdict(&value);
            match score {
                Some(_) => (score, reason, checks, None),
                None if job_status == Some("success") => (
                    None,
                    reason,
                    checks,
                    Some("The scorer returned no number to plot".to_string()),
                ),
                None => (
                    None,
                    reason,
                    checks,
                    Some("The scoring job failed".to_string()),
                ),
            }
        }
        None if job_status == Some("success") => return None,
        None => (None, None, None, Some("The scoring job failed".to_string())),
    })
}

/// A scorer may return a bare number, a boolean, or `{score, reason, checks}`; an agent wraps its
/// answer in `output`, sometimes as a string holding any of those. Anything with no number in it
/// is left empty rather than guessed at.
/// A fenced code block as the model wrote it, reduced to what is inside the fence. The opening
/// fence carries a language tag often enough that the first line goes with it.
fn unfence(text: &str) -> &str {
    let trimmed = text.trim();
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    let inner = match rest.split_once('\n') {
        Some((_language, body)) => body,
        None => rest,
    };
    inner.trim_end().trim_end_matches("```").trim()
}

fn extract_verdict(value: &RawValue) -> (Option<f64>, Option<String>, Option<serde_json::Value>) {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(value.get()) else {
        return (None, None, None);
    };
    fn as_number(value: &serde_json::Value) -> Option<f64> {
        match value {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }
    if let Some(number) = as_number(&parsed) {
        return (Some(number), None, None);
    }
    let serde_json::Value::Object(map) = &parsed else {
        // A judge often answers with JSON inside a string, and often fences it as markdown even
        // when told to reply with JSON only. Both are the model doing what it was asked; refusing
        // to read them is what turns a good verdict into "no number to plot".
        if let serde_json::Value::String(text) = &parsed {
            if let Ok(inner) = serde_json::from_str::<serde_json::Value>(unfence(text)) {
                if let Ok(raw) = serde_json::value::to_raw_value(&inner) {
                    return extract_verdict(&raw);
                }
            }
        }
        return (None, None, None);
    };
    if let Some(score) = map.get("score").and_then(as_number) {
        return (
            Some(score),
            map.get("reason")
                .and_then(|r| r.as_str())
                .map(|r| r.to_string()),
            map.get("checks").cloned(),
        );
    }
    match map.get("output") {
        Some(output) => match serde_json::value::to_raw_value(output) {
            Ok(raw) => extract_verdict(&raw),
            Err(_) => (None, None, None),
        },
        None => (None, None, None),
    }
}

#[derive(Deserialize)]
pub struct ExperimentRef {
    pub id: Uuid,
    /// The experiment every column is compared against. A delta is only ever computed between two
    /// scores of the same scorer id.
    #[serde(default)]
    pub baseline: Option<Uuid>,
}

/// One scorer's verdict on one run, and how it compares with the baseline.
#[derive(Serialize)]
pub struct CellScore {
    pub scorer_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checks: Option<Box<RawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// A scoring job is still running for this cell.
    pub pending: bool,
    /// Which side of the scorer's threshold the score fell on, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline: Option<f64>,
    /// The baseline's score for this scorer was produced by a different definition of it, so the
    /// delta is a change of scorer as much as a change of agent.
    pub definition_changed: bool,
}

/// One row per case: what it was asked, what the agent answered, and each scorer's cell.
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
    /// The agent version this cell ran against. Cells of one experiment can differ, which is what
    /// the table says instead of averaging two versions silently.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_version: Option<i64>,
    /// For a draft run, the hash of the configuration this cell ran. A draft moves without its
    /// version changing, so this is what says the row describes an agent that has been edited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_draft_hash: Option<String>,
    /// One entry per scorer of the dataset, in column order.
    pub scores: Vec<CellScore>,
}

/// A column's summary. There is no single number for a dataset: averaging a judge with an exact
/// match would invent one.
#[derive(Serialize)]
pub struct ScorerMean {
    pub scorer_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mean: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_mean: Option<f64>,
    /// The share of scored cells that passed, for a column with a threshold. Reported beside the
    /// mean rather than instead of it: a pass rate says how many cases are good enough, and a
    /// mean says by how much, and neither answers the other's question.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pass_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_pass_rate: Option<f64>,
    pub scored: usize,
    /// Cells the baseline has no score for, which is what the offer to score it counts.
    pub missing_in_baseline: usize,
    pub definition_changed: bool,
}

#[derive(Serialize)]
pub struct ExperimentResults {
    pub experiment: EvalExperiment,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline: Option<EvalExperiment>,
    /// The columns, which belong to the dataset rather than to the experiment.
    pub scorers: Vec<Scorer>,
    pub rows: Vec<ExperimentRow>,
    pub means: Vec<ScorerMean>,
    /// Cells scoring lower than the baseline, across every column.
    pub regressed: usize,
    /// The version the subject is on now. A row that ran against an earlier one describes an
    /// agent that no longer exists, which is the difference between a stale number and a wrong
    /// one. Absent for a draft, which has no versions to be behind.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_current_version: Option<i64>,
    /// The agent has edits that were never deployed. A run resolves the resource live and so
    /// executes the deployed value: without this, editing an agent and running evals reads as
    /// testing the edits when it is testing what they replace.
    pub subject_has_undeployed_changes: bool,
    /// What the draft hashes to now, for a draft subject. A row carrying a different one ran a
    /// configuration that has since been edited, which is the draft's answer to a version bump.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_current_draft_hash: Option<String>,
    /// What the agent hashes to as deployed. A draft run carrying this hash ran exactly what is
    /// deployed now — the edits were saved — so it is a run of that version rather than of a
    /// draft, and saying otherwise would strand it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_deployed_hash: Option<String>,
}

/// The agent's own result is `{output, messages}`; the answer is its `output`.
fn agent_answer(result: &RawValue) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(result.get()).ok()?;
    match parsed.get("output") {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(other) => Some(other.to_string()),
        None => None,
    }
}

struct ScoreRow {
    score: Option<f64>,
    reason: Option<String>,
    checks: Option<serde_json::Value>,
    error: Option<String>,
    definition: String,
    job_id: Option<Uuid>,
}

/// Every score of one experiment, keyed by the cell and the scorer that produced it.
async fn load_scores(
    db: &DB,
    experiment_id: Uuid,
) -> Result<std::collections::HashMap<(i32, String), ScoreRow>> {
    Ok(sqlx::query!(
        "SELECT ordinal, scorer_id, score, reason, checks, error, definition, job_id
         FROM eval_score WHERE experiment_id = $1",
        experiment_id
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|r| {
        (
            (r.ordinal, r.scorer_id),
            ScoreRow {
                score: r.score,
                reason: r.reason,
                checks: r.checks,
                error: r.error,
                definition: r.definition,
                job_id: r.job_id,
            },
        )
    })
    .collect())
}

async fn read_experiment(db: &DB, w_id: &str, dataset: &str, id: Uuid) -> Result<EvalExperiment> {
    let row = sqlx::query!(
        "SELECT e.subject, e.run_number, e.label, e.scored_from, e.created_at, e.created_by,
                (SELECT count(*) FROM eval_experiment_case c WHERE c.experiment_id = e.id)
                    AS \"case_count!\"
         FROM eval_experiment e
         WHERE e.workspace_id = $1 AND e.dataset_path = $2 AND e.id = $3",
        w_id,
        dataset,
        id
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        Error::NotFound(format!(
            "Experiment {} not found in eval dataset {}",
            id, dataset
        ))
    })?;
    experiment_from_row(
        id,
        dataset.to_string(),
        row.subject,
        row.run_number,
        row.label,
        row.scored_from,
        row.case_count,
        row.created_at,
        row.created_by,
    )
}

/// Recognise a draft run that has since been deployed, and record it as the version it became.
///
/// The hash of what it executed is the proof, so this is a resolution rather than a rewrite: it
/// says which version the run was, which nobody could say while the configuration sat in a draft.
/// Written once rather than derived on every read, because deriving it only against what is
/// deployed *now* makes the answer expire — the next deployment would send a run that already read
/// `v21` back to `v18 + edits`, and a label that moves backwards is worse than one that never
/// moved.
///
/// Written on the unrestricted pool, like the scores harvested beside it: the caller's access was
/// established by the dataset read, and nothing here comes from the caller.
async fn resolve_deployed_draft(
    db: &DB,
    w_id: &str,
    dataset: &str,
    experiment: &mut EvalExperiment,
    deployed_hash: Option<&str>,
    deployed_version: Option<i64>,
) -> Result<()> {
    if experiment.subject.kind != EvalSubjectKind::AgentDraft {
        return Ok(());
    }
    let (Some(hash), Some(deployed_hash), Some(version)) = (
        experiment.subject.draft_hash.as_deref(),
        deployed_hash,
        deployed_version,
    ) else {
        return Ok(());
    };
    if hash != deployed_hash {
        return Ok(());
    }
    // The hash stays: it is what identifies the configuration, and what this resolution rests on.
    experiment.subject.kind = EvalSubjectKind::Agent;
    experiment.subject.version = Some(version);
    sqlx::query!(
        "UPDATE eval_experiment
         SET subject = jsonb_set(
                 jsonb_set(subject, '{kind}', '\"agent\"'),
                 '{version}', to_jsonb($4::bigint))
         WHERE workspace_id = $1 AND dataset_path = $2 AND id = $3
               AND subject ->> 'kind' = 'agent_draft'",
        w_id,
        dataset,
        experiment.id,
        version,
    )
    .execute(db)
    .await?;
    // The cells that ran that configuration are dated by the version too. Their hash has served
    // its purpose — it was what dated a cell with no version to name it by — and leaving it would
    // make the run go on reading as a draft's after the next deployment.
    sqlx::query!(
        "UPDATE eval_experiment_case
         SET subject_version = $3, subject_draft_hash = NULL
         WHERE experiment_id = $1 AND subject_draft_hash = $2",
        experiment.id,
        hash,
        version,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The rows a results table is built from. The job ids come out of `eval_experiment_case`, which
/// only this module writes, so they can be read on the unrestricted pool: the caller's access was
/// established by the dataset read below, and the ids are not caller-supplied.
pub async fn experiment_results(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, dataset)): Path<(String, String)>,
    Query(query): Query<ExperimentRef>,
) -> JsonResult<ExperimentResults> {
    let dataset_row = read_dataset(&authed, &user_db, &w_id, &dataset).await?;
    let scorers = dataset_row.scorers;

    let mut experiment = read_experiment(&db, &w_id, &dataset, query.id).await?;
    // Finished scoring jobs are read into their score rows here, so a score outlives the job that
    // produced it rather than being recomputed from a job that may have been retained away.
    harvest_scores(&db, &w_id, query.id).await?;
    let scores = load_scores(&db, query.id).await?;

    let baseline = match query.baseline.filter(|id| *id != query.id) {
        Some(id) => {
            let baseline = read_experiment(&db, &w_id, &dataset, id).await?;
            harvest_scores(&db, &w_id, id).await?;
            Some((baseline, load_scores(&db, id).await?))
        }
        None => None,
    };
    // The baseline is compared case by case, so its cells are keyed by the case they ran.
    let baseline_ordinals = match &baseline {
        Some((baseline, _)) => sqlx::query!(
            "SELECT case_id, ordinal FROM eval_experiment_case WHERE experiment_id = $1",
            baseline.id
        )
        .fetch_all(&db)
        .await?
        .into_iter()
        .map(|r| (r.case_id, r.ordinal))
        .collect::<std::collections::HashMap<_, _>>(),
        None => Default::default(),
    };

    let case_rows = sqlx::query!(
        "SELECT ordinal, case_id, name, input, expected, job_id, subject_version,
                subject_draft_hash
         FROM eval_experiment_case
         WHERE experiment_id = $1 ORDER BY ordinal",
        query.id
    )
    .fetch_all(&db)
    .await?;

    // One query for every case job's own status, rather than inferring it from the answer.
    let job_ids: Vec<Uuid> = case_rows.iter().map(|c| c.job_id).collect();
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

    // Bounded concurrency rather than one await after another: a 100-case experiment is 100
    // lookups, and each is itself several queries.
    use futures::StreamExt;
    let answers = futures::stream::iter(job_ids.iter().copied().map(|job_id| {
        let db = db.clone();
        let w_id = w_id.clone();
        async move {
            let output = windmill_queue::get_result_and_success_by_id_from_flow(
                &db,
                &w_id,
                &job_id,
                AGENT_NODE_ID,
                None,
            )
            .await
            .ok()
            .and_then(|(r, _)| agent_answer(&r));
            (job_id, output)
        }
    }))
    .buffered(8)
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .collect::<std::collections::HashMap<_, _>>();

    let mut sums = vec![(0.0f64, 0usize); scorers.len()];
    let mut baseline_sums = vec![(0.0f64, 0usize); scorers.len()];
    let mut passes = vec![0usize; scorers.len()];
    let mut baseline_passes = vec![0usize; scorers.len()];
    let mut missing_in_baseline = vec![0usize; scorers.len()];
    let mut definition_changed = vec![false; scorers.len()];
    let mut regressed = 0usize;
    let mut rows = Vec::with_capacity(case_rows.len());

    for case in case_rows {
        let mut cells = Vec::with_capacity(scorers.len());
        for (index, scorer) in scorers.iter().enumerate() {
            let current = scores.get(&(case.ordinal, scorer.id.clone()));
            let baseline_score = baseline.as_ref().and_then(|(_, baseline_scores)| {
                baseline_ordinals
                    .get(&case.case_id)
                    .and_then(|ordinal| baseline_scores.get(&(*ordinal, scorer.id.clone())))
            });
            if let Some(score) = current.and_then(|c| c.score) {
                sums[index].0 += score;
                sums[index].1 += 1;
                if scorer.passed(Some(score)) == Some(true) {
                    passes[index] += 1;
                }
            }
            if let Some(score) = baseline_score.and_then(|b| b.score) {
                baseline_sums[index].0 += score;
                baseline_sums[index].1 += 1;
                if scorer.passed(Some(score)) == Some(true) {
                    baseline_passes[index] += 1;
                }
            } else if baseline.is_some() {
                missing_in_baseline[index] += 1;
            }
            let changed = match (current, baseline_score) {
                (Some(current), Some(baseline)) => current.definition != baseline.definition,
                _ => false,
            };
            if changed {
                definition_changed[index] = true;
            }
            if let (Some(score), Some(previous)) = (
                current.and_then(|c| c.score),
                baseline_score.and_then(|b| b.score),
            ) {
                if score < previous {
                    regressed += 1;
                }
            }
            cells.push(CellScore {
                scorer_id: scorer.id.clone(),
                score: current.and_then(|c| c.score),
                reason: current.and_then(|c| c.reason.clone()),
                checks: current
                    .and_then(|c| c.checks.clone())
                    .map(|c| serde_json::value::to_raw_value(&c))
                    .transpose()?,
                error: current.and_then(|c| c.error.clone()),
                pending: current
                    .map(|c| c.score.is_none() && c.error.is_none() && c.job_id.is_some())
                    .unwrap_or(false),
                passed: scorer.passed(current.and_then(|c| c.score)),
                baseline: baseline_score.and_then(|b| b.score),
                definition_changed: changed,
            });
        }
        rows.push(ExperimentRow {
            case_id: case.case_id,
            name: case.name,
            input: serde_json::from_value(case.input)?,
            expected: opt_to_raw(case.expected)?,
            status: statuses
                .get(&case.job_id)
                .cloned()
                .unwrap_or_else(|| "running".to_string()),
            output: answers.get(&case.job_id).cloned().flatten(),
            subject_version: case.subject_version,
            subject_draft_hash: case.subject_draft_hash,
            job_id: case.job_id,
            scores: cells,
        });
    }

    let means = scorers
        .iter()
        .enumerate()
        .map(|(index, scorer)| ScorerMean {
            scorer_id: scorer.id.clone(),
            mean: (sums[index].1 > 0).then(|| sums[index].0 / sums[index].1 as f64),
            baseline_mean: (baseline_sums[index].1 > 0)
                .then(|| baseline_sums[index].0 / baseline_sums[index].1 as f64),
            pass_rate: (scorer.pass_if.is_some() && sums[index].1 > 0)
                .then(|| passes[index] as f64 / sums[index].1 as f64),
            baseline_pass_rate: (scorer.pass_if.is_some() && baseline_sums[index].1 > 0)
                .then(|| baseline_passes[index] as f64 / baseline_sums[index].1 as f64),
            scored: sums[index].1,
            missing_in_baseline: missing_in_baseline[index],
            definition_changed: definition_changed[index],
        })
        .collect();

    // What the subject is now: the version it is on, whether it has edits waiting, and — for a
    // draft — what those edits hash to, so a row that ran an earlier draft can say so.
    let mut subject_current_draft_hash = None;
    let subject_deployed_hash = deployed_agent_hash(&db, &w_id, &experiment.subject.path).await?;
    let (subject_current_version, subject_has_undeployed_changes) = match experiment.subject.kind {
        EvalSubjectKind::Agent => {
            let version = current_resource_version(&db, &w_id, &experiment.subject.path).await?;
            // Whether a resource has a draft is information about that resource, and `draft`
            // carries no policies of its own: the agent is read through `user_db` first, so a
            // caller who cannot see it is told nothing about it.
            let mut tx = user_db.begin(&authed).await?;
            let visible = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM resource WHERE workspace_id = $1 AND path = $2)",
                w_id,
                experiment.subject.path
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(false);
            tx.commit().await?;
            let undeployed = visible
                && sqlx::query_scalar!(
                    "SELECT EXISTS(SELECT 1 FROM draft
                     WHERE workspace_id = $1 AND path = $2 AND typ = 'resource')",
                    w_id,
                    experiment.subject.path
                )
                .fetch_one(&db)
                .await?
                .unwrap_or(false);
            (version, undeployed)
        }
        // A draft run already ran the draft: what matters is whether one is still there to run,
        // and which deployed version it is now an edit of.
        EvalSubjectKind::AgentDraft => {
            let mut tx = user_db.clone().begin(&authed).await?;
            let still_drafted = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM resource r
                 WHERE r.workspace_id = $1 AND r.path = $2
                       AND EXISTS(SELECT 1 FROM draft d
                                  WHERE d.workspace_id = $1 AND d.path = r.path
                                        AND d.typ = 'resource'))",
                w_id,
                experiment.subject.path
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(false);
            tx.commit().await?;
            if still_drafted {
                subject_current_draft_hash = Some(draft_hash(
                    &agent_draft_config(&authed, &user_db, &w_id, &experiment.subject.path).await?,
                ));
            }
            let version = current_resource_version(&db, &w_id, &experiment.subject.path).await?;
            (version, still_drafted)
        }
    };

    // A draft run whose configuration has since been deployed is a run of that version.
    let mut baseline = baseline.map(|(baseline, _)| baseline);
    resolve_deployed_draft(
        &db,
        &w_id,
        &dataset,
        &mut experiment,
        subject_deployed_hash.as_deref(),
        subject_current_version,
    )
    .await?;
    if let Some(baseline) = baseline.as_mut() {
        // The compare-to list holds this agent's runs, but the id is the caller's: a run of another
        // agent must not be stamped with this one's version.
        if baseline.subject.path == experiment.subject.path {
            resolve_deployed_draft(
                &db,
                &w_id,
                &dataset,
                baseline,
                subject_deployed_hash.as_deref(),
                subject_current_version,
            )
            .await?;
        }
    }

    Ok(Json(ExperimentResults {
        experiment,
        baseline,
        scorers,
        rows,
        means,
        regressed,
        subject_current_version,
        subject_has_undeployed_changes,
        subject_current_draft_hash,
        subject_deployed_hash,
    }))
}

/// What a script scorer starts from. The assertions are in `main` and the helpers below it, so
/// the file reads as the checks that were chosen rather than as a library to learn.
pub const SCORER_SCRIPT_TEMPLATE: &str = r#"// A scorer receives one run and returns a number between 0 and 1, a boolean, or
// { score, reason, checks } — checks show up in the case detail.
type ToolCall = {
  name: string
  args?: Record<string, unknown>
  result?: unknown
  error?: string
  duration_ms?: number
  truncated?: boolean
}

type EvalRun = {
  input: { user_message?: string; user_attachments?: unknown[] }
  output?: unknown
  expected?: unknown
  tool_calls: ToolCall[]
  tools: { name: string; schema?: Record<string, unknown> }[]
  metrics: { steps: number; duration_ms?: number; usage?: Record<string, unknown> }
  status: string
  job_id: string
}

export async function main(run: EvalRun) {
  const checks = [
    check('resolves the request', contains(run.output, String(run.expected ?? ''))),
    check('arguments match the schema', args_schema_valid(run)),
    check('no repeated calls', no_repeated_calls(run)),
    check('no step errors', no_step_errors(run)),
    check('under 6 steps', run.metrics.steps <= 6),
    check('under 30 seconds', under_ms(run, 30_000))
  ]
  const passed = checks.filter((c) => c.passed).length
  return { score: passed / checks.length, checks }
}

// Helpers. Edit or delete freely.

function check(name: string, passed: boolean, detail?: string) {
  return { name, passed, detail }
}

function text(value: unknown): string {
  return typeof value === 'string' ? value : JSON.stringify(value ?? '')
}

function exact_match(output: unknown, expected: unknown): boolean {
  return text(output).trim() === text(expected).trim()
}

// Key order and whitespace insensitive.
function json_equals(a: unknown, b: unknown): boolean {
  const sort = (value: unknown): unknown =>
    Array.isArray(value)
      ? value.map(sort)
      : value && typeof value === 'object'
        ? Object.fromEntries(
            Object.entries(value as Record<string, unknown>)
              .sort(([x], [y]) => x.localeCompare(y))
              .map(([k, v]) => [k, sort(v)])
          )
        : value
  return JSON.stringify(sort(a)) === JSON.stringify(sort(b))
}

function contains(output: unknown, needle: string): boolean {
  return needle.trim().length > 0 && text(output).toLowerCase().includes(needle.trim().toLowerCase())
}

function matches(output: unknown, re: RegExp): boolean {
  return re.test(text(output))
}

// Allow list: every named tool was called.
function tool_called(run: EvalRun, names: string[]): boolean {
  return names.every((name) => run.tool_calls.some((call) => call.name === name))
}

// Deny list: none of the named tools was called.
function tool_not_called(run: EvalRun, names: string[]): boolean {
  return !run.tool_calls.some((call) => names.includes(call.name))
}

// Every call validated against the schema of the tool it called. A tool whose schema could not be
// resolved is not checked rather than failed.
function args_schema_valid(run: EvalRun): boolean {
  return run.tool_calls.every((call) => {
    const schema = run.tools.find((tool) => tool.name === call.name)?.schema as
      | { properties?: Record<string, { type?: string }>; required?: string[] }
      | undefined
    if (!schema?.properties) return true
    const args = call.args ?? {}
    for (const key of schema.required ?? []) {
      if (args[key] === undefined || args[key] === null) return false
    }
    for (const [key, value] of Object.entries(args)) {
      const expected = schema.properties[key]?.type
      if (!expected) continue
      const actual = Array.isArray(value) ? 'array' : value === null ? 'null' : typeof value
      if (expected === 'integer' ? !Number.isInteger(value) : expected !== actual) return false
    }
    return true
  })
}

// The same tool called twice with the same arguments.
function no_repeated_calls(run: EvalRun): boolean {
  const seen = new Set<string>()
  for (const call of run.tool_calls) {
    const key = `${call.name}:${JSON.stringify(call.args ?? {})}`
    if (seen.has(key)) return false
    seen.add(key)
  }
  return true
}

function no_step_errors(run: EvalRun): boolean {
  return run.status === 'success' && run.tool_calls.every((call) => !call.error)
}

function under_ms(run: EvalRun, max: number): boolean {
  return (run.metrics.duration_ms ?? 0) <= max
}

function under_tokens(run: EvalRun, max: number): boolean {
  const usage = (run.metrics.usage ?? {}) as Record<string, number>
  return (usage.input_tokens ?? 0) + (usage.output_tokens ?? 0) <= max
}

// Windmill keeps no provider price table, so the rate is yours to set: dollars per 1k tokens.
function cost_under(run: EvalRun, usd: number, rate: { input: number; output: number }): boolean {
  const usage = (run.metrics.usage ?? {}) as Record<string, number>
  const cost =
    ((usage.input_tokens ?? 0) / 1000) * rate.input +
    ((usage.output_tokens ?? 0) / 1000) * rate.output
  return cost <= usd
}
"#;

#[derive(Serialize)]
pub struct ScorerDefaults {
    /// The system prompt a judge agent is created with. It lives on that agent afterwards.
    pub judge_prompt: String,
    /// The starting point for a script scorer, held here so the shape a scorer is handed and the
    /// template that reads it cannot drift apart.
    pub script_template: String,
}

pub async fn scorer_defaults() -> JsonResult<ScorerDefaults> {
    Ok(Json(ScorerDefaults {
        judge_prompt: JUDGE_SYSTEM_PROMPT.to_string(),
        script_template: SCORER_SCRIPT_TEMPLATE.to_string(),
    }))
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
    // What the run answered becomes the case's expected value: capture time is the only moment a
    // reference answer exists for free, and every scorer is handed one. A run that failed or is
    // still going has nothing to offer, and the caller can always write their own.
    let expected = sqlx::query!(
        "SELECT result AS \"result: sqlx::types::Json<Box<RawValue>>\", status::text AS \"status!\"
         FROM v2_job_completed WHERE id = $1 AND workspace_id = $2",
        job_id,
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .filter(|r| r.status == "success")
    .and_then(|r| r.result)
    .and_then(|r| agent_answer(&r.0))
    .map(|answer| serde_json::value::to_raw_value(&answer))
    .transpose()?;

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
        expected,
        source: EvalCaseSource {
            job_id: Some(job_id),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(json: &str) -> Box<RawValue> {
        serde_json::from_str(json).unwrap()
    }

    /// A scorer's answer arrives in whatever shape its runnable returns: a script's bare value or
    /// object, or a judge's answer wrapped in `output` and often stringified. A shape that goes
    /// unrecognised is a silently empty cell, not an error.
    #[test]
    fn extract_verdict_reads_every_documented_scorer_shape() {
        let score = |json: &str| extract_verdict(&raw(json)).0;
        assert_eq!(score("0.75"), Some(0.75));
        assert_eq!(score("true"), Some(1.0));
        assert_eq!(score(r#"{"score": 0.5}"#), Some(0.5));
        assert_eq!(score(r#"{"score": false}"#), Some(0.0));

        // judges and agent scorers: the answer is under `output`, sometimes as a string
        assert_eq!(score(r#"{"output": 0.25}"#), Some(0.25));
        assert_eq!(score(r#"{"output": "0.9"}"#), Some(0.9));
        assert_eq!(score(r#"{"output": {"score": 0.8}}"#), Some(0.8));
        assert_eq!(score(r#"{"output": "{\"score\": 0.4}"}"#), Some(0.4));

        // a judge told to reply with JSON only, replying with JSON only, in a code fence
        assert_eq!(
            score("{\"output\": \"```json\\n{\\\"score\\\": 0.15}\\n```\"}"),
            Some(0.15)
        );
        assert_eq!(score("{\"output\": \"```\\n0.6\\n```\"}"), Some(0.6));

        // nothing numeric to plot: left empty rather than guessed at
        assert_eq!(score(r#"{"output": "not a score"}"#), None);
        assert_eq!(score(r#"{"verdict": "good"}"#), None);

        let (score, reason, checks) = extract_verdict(&raw(
            r#"{"score": 0.5, "reason": "half", "checks": [{"name": "a"}]}"#,
        ));
        assert_eq!((score, reason), (Some(0.5), Some("half".to_string())));
        assert!(checks.is_some());
    }

    /// The definition hash is what tells a comparison that the scorer changed. The path alone
    /// would miss an edit to the script itself, which is the most common way a column stops
    /// meaning what it meant.
    #[test]
    fn definition_moves_with_the_runnable_and_not_with_its_name() {
        let script = |path: &str, name: Option<&str>| Scorer {
            id: "s1".to_string(),
            name: name.map(|n| n.to_string()),
            pass_if: None,
            def: ScorerDef::Script { path: path.to_string() },
        };
        // Renaming a column is not a change of scorer: same runnable, same version.
        assert_eq!(
            script("f/e/s", None).definition(Some("1234")),
            script("f/e/s", Some("Tool discipline")).definition(Some("1234"))
        );
        // Same script, newly deployed: the column says the scorer changed.
        assert_ne!(
            script("f/e/s", None).definition(Some("1234")),
            script("f/e/s", None).definition(Some("5678"))
        );
        // A judge agent and a script sharing a path are not the same column.
        let agent = Scorer {
            id: "s1".to_string(),
            name: None,
            pass_if: None,
            def: ScorerDef::Agent { path: "f/e/s".to_string() },
        };
        assert_ne!(
            agent.definition(Some("1")),
            script("f/e/s", None).definition(Some("1"))
        );
        // Where the pass line sits reads a score rather than produces one. If it entered the hash,
        // setting a threshold would mark every score already recorded as coming from a different
        // scorer, and the pass rate it exists to give would arrive with the whole column flagged.
        let mut thresholded = script("f/e/s", None);
        thresholded.pass_if = Some(0.7);
        assert_eq!(
            thresholded.definition(Some("1234")),
            script("f/e/s", None).definition(Some("1234"))
        );
        assert_eq!(thresholded.passed(Some(0.7)), Some(true));
        assert_eq!(thresholded.passed(Some(0.69)), Some(false));
        assert_eq!(script("f/e/s", None).passed(Some(0.1)), None);
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
