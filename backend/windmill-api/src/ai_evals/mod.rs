//! Eval datasets for reusable AI agents.
//!
//! Five tables: `eval_dataset` and the `eval_case` rows it holds are the curated inputs;
//! `eval_experiment`, `eval_experiment_case` and `eval_score` are one run of them, written once
//! and only ever read afterwards.
//!
//! Datasets and cases go through `user_db`, so row-level security is the only access authority:
//! `eval_case`'s policies derive from its dataset's (`eval_dataset_writable`, in the migration).
//! The experiment tables carry read policies only and are written on the unrestricted pool after
//! the API has checked access — see `run_experiment` and `collect_experiment`.

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
    utils::{check_proper_path, paginate, Pagination},
};

use crate::db::{ApiAuthed, DB};
use windmill_api_auth::check_scopes;

pub(crate) mod datasets;
pub(crate) mod payload;
pub(crate) mod results;
pub(crate) mod run;
pub(crate) mod scorers;
pub(crate) mod scoring;
pub(crate) mod subject;
pub(crate) mod template;

pub(crate) use datasets::*;
pub(crate) use payload::*;
pub(crate) use results::*;
pub(crate) use run::*;
pub(crate) use scorers::*;
pub(crate) use scoring::*;
pub(crate) use subject::*;
pub(crate) use template::*;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/datasets/list", get(list_datasets))
        .route("/datasets/create", post(create_dataset))
        .route("/datasets/get/{*path}", get(get_dataset))
        .route("/datasets/update/{*path}", post(update_dataset))
        .route("/datasets/delete/{*path}", post(delete_dataset))
        .route("/cases/list/{*path}", get(list_cases))
        .route("/scorer_defaults", get(scorer_defaults))
        .route("/run_payload", get(run_payload))
        .route("/experiments/run", post(run_experiment))
        .route("/experiments/collect", post(collect_experiment))
        .route("/scorers/recent", get(recent_scorers))
        .route("/subject_state", get(subject_state))
        .route("/experiments/list_all", get(list_all_experiments))
        .route("/experiments/results/{*path}", get(experiment_results))
}

/// Checked here rather than left to the column, whose own refusal comes back as an internal
/// database error naming no field.
const MAX_DATASET_SUMMARY_CHARS: usize = 1000;

fn check_summary(summary: Option<&str>) -> Result<()> {
    match summary {
        Some(summary) if summary.chars().count() > MAX_DATASET_SUMMARY_CHARS => {
            Err(Error::BadRequest(format!(
                "This dataset's summary is {} characters, over the {} the column holds.",
                summary.chars().count(),
                MAX_DATASET_SUMMARY_CHARS
            )))
        }
        _ => Ok(()),
    }
}

/// A case is text — attachments are S3 references rather than inline bytes.
const MAX_CASE_BYTES: usize = 256 * 1024;
/// The whole case set together, so cases at the per-case cap cannot add up to a dataset a listing
/// or a run must hold hundreds of megabytes of at once.
const MAX_DATASET_BYTES: usize = 16 * 1024 * 1024;
/// Also what a listing returns in one page, so a dataset is always read whole: the editor holds
/// every case at once and writes them together, and half a set on screen is a Save that drops the
/// rest.
const MAX_CASES_PER_DATASET: i64 = 1_000;

const MAX_EXPERIMENTS_LISTED: i64 = 100;

const MAX_RECENT_SCORERS: usize = 12;

/// A run's work is cases × scorers, so this bounds how far one request fans out.
const MAX_SCORERS_PER_DATASET: usize = 20;

/// The dataset a write was aimed at is gone. Raised from the foreign key rather than from a
/// preceding existence check, so a dataset deleted mid-request cannot slip between the two.
fn is_missing_dataset(e: &sqlx::Error) -> bool {
    e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23503")
}

/// A `user_db` write the row-level policies refused surfaces as SQLSTATE 42501, whose message
/// names the table and the policy. Turn it into one about access.
fn map_rls_denied(path: &str, action: &str, e: sqlx::Error) -> Error {
    if e.as_database_error().and_then(|d| d.code()).as_deref() == Some("42501") {
        return Error::NotAuthorized(format!("Not allowed to {} eval dataset {}", action, path));
    }
    e.into()
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

/// One `eval_dataset` row, from the columns every read of the table selects.
fn dataset_from_row(
    path: String,
    summary: Option<String>,
    scorers: serde_json::Value,
    created_at: DateTime<Utc>,
    created_by: String,
    edited_at: DateTime<Utc>,
    edited_by: String,
) -> Result<EvalDataset> {
    Ok(EvalDataset {
        path,
        summary,
        scorers: parse_scorers(scorers)?,
        created_at,
        created_by,
        edited_at,
        edited_by,
    })
}

/// A dataset's columns. Only this module writes them, through serde, so a value that does not
/// parse is corruption rather than input: defaulting to no columns would let the next save mint
/// fresh scorer ids and orphan every score already recorded.
pub(crate) fn parse_scorers(scorers: serde_json::Value) -> Result<Vec<Scorer>> {
    serde_json::from_value(scorers)
        .map_err(|e| Error::internal_err(format!("eval dataset scorers are not readable: {e}")))
}

/// Read the dataset the request names, through `user_db` so that a caller who cannot see it gets
/// the same answer as one asking for a dataset that does not exist.
async fn read_dataset(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    path: &str,
) -> Result<EvalDataset> {
    check_proper_path(path)?;
    let mut tx = user_db.clone().begin(authed).await?;
    let row = sqlx::query!(
        "SELECT path, summary, scorers, created_at, created_by,
                edited_at, edited_by
         FROM eval_dataset WHERE workspace_id = $1 AND path = $2",
        w_id,
        path
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let row = row.ok_or_else(|| Error::NotFound(format!("Eval dataset {} not found", path)))?;
    dataset_from_row(
        row.path,
        row.summary,
        row.scorers,
        row.created_at,
        row.created_by,
        row.edited_at,
        row.edited_by,
    )
}

/// The dataset and its cases as one snapshot, so a launch cannot record the cases from before an
/// edit beside the scorers from after it. One transaction is not enough: `user_db` runs at READ
/// COMMITTED, where each statement takes a fresh snapshot, so the row is taken `FOR UPDATE` —
/// which an edit's own `FOR UPDATE` and a case write's foreign-key lock both conflict with.
pub(crate) async fn read_dataset_and_cases(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    path: &str,
) -> Result<(EvalDataset, Vec<EvalCase>)> {
    check_proper_path(path)?;
    let mut tx = user_db.clone().begin(authed).await?;
    let row = sqlx::query!(
        "SELECT path, summary, scorers, created_at, created_by, edited_at, edited_by
         FROM eval_dataset WHERE workspace_id = $1 AND path = $2 FOR UPDATE",
        w_id,
        path
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = row else {
        tx.commit().await?;
        return Err(Error::NotFound(format!("Eval dataset {} not found", path)));
    };
    let case_rows = sqlx::query!(
        "SELECT id, input, expected, created_at, created_by
         FROM eval_case
         WHERE workspace_id = $1 AND dataset_path = $2
         ORDER BY created_at, id",
        w_id,
        path
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let dataset = dataset_from_row(
        row.path,
        row.summary,
        row.scorers,
        row.created_at,
        row.created_by,
        row.edited_at,
        row.edited_by,
    )?;
    let cases = case_rows
        .into_iter()
        .map(|row| {
            Ok(EvalCase {
                id: row.id,
                input: serde_json::from_value(row.input)?,
                expected: opt_to_raw(row.expected)?,
                created_at: row.created_at,
                created_by: row.created_by,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((dataset, cases))
}

/// Whether this caller may write a dataset's contents: its cases, and the experiments that run
/// them.
///
/// `SELECT … FOR UPDATE` applies `eval_dataset`'s UPDATE policies on top of its SELECT policies,
/// so the row itself answers who may write it, and a grant in `extra_perms` is honoured without
/// being mirrored here.
async fn require_dataset_writable(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    path: &str,
) -> Result<()> {
    check_proper_path(path)?;
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

/// jsonb columns are read as `serde_json::Value` and handed on as `RawValue`: a case's `expected`
/// is arbitrary user JSON that this module never looks inside.
fn opt_to_raw(value: Option<serde_json::Value>) -> Result<Option<Box<RawValue>>> {
    value
        .map(|v| Ok(serde_json::value::to_raw_value(&v)?))
        .transpose()
}

fn opt_from_raw(value: Option<&Box<RawValue>>) -> Result<Option<serde_json::Value>> {
    value
        .map(|v| Ok(serde_json::from_str(v.get())?))
        .transpose()
}

fn check_case(input: &EvalCaseInput, expected: Option<&Box<RawValue>>) -> Result<()> {
    // The shape the agent step reads its attachments in, checked when the case is written rather
    // than when a run deserialises the step's arguments, which is after the case was queued.
    if let Some(attachments) = &input.user_attachments {
        if serde_json::from_str::<Vec<windmill_types::s3::S3Object>>(attachments.get()).is_err() {
            return Err(Error::BadRequest(
                "A case's user_attachments is a list of S3 objects, each with an `s3` key naming \
                 the file"
                    .to_string(),
            ));
        }
    }
    check_case_size(input, expected)
}

/// The bytes one case weighs against its own and the dataset's caps.
fn case_bytes(input: &EvalCaseInput, expected: Option<&Box<RawValue>>) -> Result<usize> {
    let mut bytes = serde_json::to_vec(input)?.len();
    if let Some(expected) = expected {
        bytes += expected.get().len();
    }
    Ok(bytes)
}

/// What a whole case set can be refused for, before any of it is written.
fn check_case_set<'a>(
    cases: impl ExactSizeIterator<Item = (&'a EvalCaseInput, Option<&'a Box<RawValue>>)>,
) -> Result<()> {
    if cases.len() as i64 > MAX_CASES_PER_DATASET {
        return Err(Error::BadRequest(format!(
            "An eval dataset holds at most {} cases. Split them into several datasets.",
            MAX_CASES_PER_DATASET
        )));
    }
    let mut total = 0usize;
    for (input, expected) in cases {
        check_case(input, expected)?;
        total += case_bytes(input, expected)?;
    }
    if total > MAX_DATASET_BYTES {
        return Err(Error::BadRequest(format!(
            "This dataset is {} KiB of cases, over the {} KiB limit. Attachments belong in \
             workspace storage and are referenced by a case, not stored inside it.",
            total / 1024,
            MAX_DATASET_BYTES / 1024
        )));
    }
    Ok(())
}

fn check_case_size(input: &EvalCaseInput, expected: Option<&Box<RawValue>>) -> Result<()> {
    let bytes = case_bytes(input, expected)?;
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
