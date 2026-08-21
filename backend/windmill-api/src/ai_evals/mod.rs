//! Eval datasets for reusable AI agents.
//!
//! A dataset is a curated set of cases: the inputs an agent is expected to handle, and what it was
//! expected to answer. Datasets, cases and experiments are rows (`eval_dataset`, `eval_case`,
//! `eval_experiment`, `eval_experiment_case`), so a dataset is permissioned, cascaded and queried
//! like any other workspace object.
//!
//! A run's trajectory is not stored here: the tool calls, their arguments and their results belong
//! to the jobs that made them, and `job_id` is the way to them. What the results table is made of
//! is — each cell's answer, its outcome and every scorer's verdict are copied onto the run's rows
//! the first time they can be read, since jobs have their own retention and a recorded run has to
//! outlive the jobs that produced it.
//!
//! Reads and writes of a dataset and its cases both go through `user_db`, so row-level security is
//! the authority throughout: `eval_dataset` carries the usual read and write policies, and
//! `eval_case` carries a read policy derived from its dataset (`see_parent_dataset`) plus write
//! policies that check the dataset is *writable* (`eval_dataset_writable`, in the migration). A
//! dataset and its cases therefore move in one transaction, governed by those policies, with no
//! access decided a second time in Rust.
//!
//! The experiment tables are the exception. Their rows are written from two places — a launch,
//! which holds dataset write, and the harvest, which holds only *read* of the run it copies onto
//! its own rows — so they carry read policies only and are written on the unrestricted pool after
//! the API has checked the appropriate access. See `run_experiment` and `collect_experiment`.

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
        .route("/cases/add/{*path}", post(add_case))
        .route("/cases/save/{*path}", post(save_cases))
        .route("/cases/update/{*path}", post(update_case))
        .route("/cases/delete/{*path}", post(delete_case))
        .route("/scorer_defaults", get(scorer_defaults))
        .route("/run_payload", get(run_payload))
        .route("/experiments/run", post(run_experiment))
        .route("/experiments/collect", post(collect_experiment))
        .route("/scorers/recent", get(recent_scorers))
        .route("/subject_state", get(subject_state))
        .route("/experiments/list_all", get(list_all_experiments))
        .route("/experiments/results/{*path}", get(experiment_results))
}

/// What `eval_dataset.path` and `.summary` hold. Checked here rather than left to the columns: a
/// value the column refuses comes back as an internal database error, which tells the caller
/// nothing about which field was too long.
const MAX_DATASET_PATH_CHARS: usize = 255;
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

/// Dataset paths are Windmill paths, so the folder they live in is what grants access to them.
fn check_path(path: &str) -> Result<()> {
    if path.chars().count() > MAX_DATASET_PATH_CHARS {
        return Err(Error::BadRequest(format!(
            "This dataset's path is {} characters, over the {} the column holds.",
            path.chars().count(),
            MAX_DATASET_PATH_CHARS
        )));
    }
    let segments = path.split('/').collect::<Vec<_>>();
    if segments.len() < 3
        || !matches!(segments[0], "u" | "f" | "g")
        || segments.iter().any(|s| s.is_empty())
        || segments.iter().any(|s| *s == "." || *s == "..")
    {
        return Err(Error::BadRequest(format!(
            "Invalid dataset path '{}': expected 'u/<user>/<name>', 'f/<folder>/<name>' or 'g/<group>/<name>'",
            path
        )));
    }
    Ok(())
}

/// A case is text: a message and the answer it was expected to produce. Attachments are S3
/// references rather than inline bytes, so nothing here is meant to be large. The caps exist so that one mistake — a whole file pasted into a message, a capture
/// loop left running — cannot grow a dataset past what a listing can load.

const MAX_CASE_BYTES: usize = 256 * 1024;
/// Also what a listing returns in one page, so a dataset is always read whole: the editor holds
/// every case at once and writes them together, and half a set on screen is a Save that looks
/// like it dropped the rest.
const MAX_CASES_PER_DATASET: i64 = 1_000;

/// Newest first, and only this many: the list feeds a picker, and a dataset that has been run
/// nightly for a year would otherwise send back every run of it.
const MAX_EXPERIMENTS_LISTED: i64 = 100;

/// Enough to cover the scorers a workspace actually reuses, few enough to stay a list you read
/// rather than one you search.
const MAX_RECENT_SCORERS: usize = 12;

/// A run scores every case by every column, so the work a dataset schedules is cases × scorers;
/// this keeps one request from fanning out past what a results table can show anyway.
const MAX_SCORERS_PER_DATASET: usize = 20;

/// A run answers every case of its dataset in one flow; more cases than this is a dataset to
/// split, not a run to start.
const MAX_CASES_PER_RUN: usize = 1_000;

/// The dataset a write was aimed at is gone. Raised from the foreign key rather than from a
/// preceding existence check, so a dataset deleted mid-request cannot slip between the two.
fn is_missing_dataset(e: &sqlx::Error) -> bool {
    e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23503")
}

/// A `user_db` write the row-level policies refused surfaces as SQLSTATE 42501 (`insufficient
/// privilege`), whose message names the table and the policy. Turn it into one about access.
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
    Ok(EvalDataset {
        path: row.path,
        summary: row.summary,
        scorers: serde_json::from_value(row.scorers).unwrap_or_default(),
        created_at: row.created_at,
        created_by: row.created_by,
        edited_at: row.edited_at,
        edited_by: row.edited_by,
    })
}

/// The dataset and its cases as one snapshot, both read in one transaction so a launch cannot
/// observe the cases from before an edit beside the scorers from after it — a dataset state that
/// never existed. `None` when the dataset is not there to read.
pub(crate) async fn read_dataset_and_cases(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    path: &str,
) -> Result<(EvalDataset, Vec<EvalCase>)> {
    check_path(path)?;
    let mut tx = user_db.clone().begin(authed).await?;
    let row = sqlx::query!(
        "SELECT path, summary, scorers, created_at, created_by, edited_at, edited_by
         FROM eval_dataset WHERE workspace_id = $1 AND path = $2",
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
        "SELECT id, name, input, expected, created_at, created_by
         FROM eval_case
         WHERE workspace_id = $1 AND dataset_path = $2
         ORDER BY created_at, id",
        w_id,
        path
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let dataset = EvalDataset {
        path: row.path,
        summary: row.summary,
        scorers: serde_json::from_value(row.scorers).unwrap_or_default(),
        created_at: row.created_at,
        created_by: row.created_by,
        edited_at: row.edited_at,
        edited_by: row.edited_by,
    };
    let cases = case_rows
        .into_iter()
        .map(|row| {
            Ok(EvalCase {
                id: row.id,
                name: row.name,
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
/// so the row itself answers who may write it. A grant in `extra_perms` is honoured without being
/// mirrored here, where a copy could drift.
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
/// case shape stores: a case's `expected` is arbitrary user JSON that this module never looks
/// inside.
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

/// What `eval_case.name` holds. Checked here rather than left to the column, because a case is
/// written after the dataset it belongs to: a name the column refuses would commit the dataset and
/// then fail its cases, leaving an empty dataset that a retry says already exists.
const MAX_CASE_NAME_CHARS: usize = 255;

/// Everything a case carries that a caller supplied, weighed against what the row can hold.
fn check_case(
    name: Option<&str>,
    input: &EvalCaseInput,
    expected: Option<&Box<RawValue>>,
) -> Result<()> {
    if let Some(name) = name {
        if name.chars().count() > MAX_CASE_NAME_CHARS {
            return Err(Error::BadRequest(format!(
                "This eval case's name is {} characters, over the {} the column holds.",
                name.chars().count(),
                MAX_CASE_NAME_CHARS
            )));
        }
    }
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
