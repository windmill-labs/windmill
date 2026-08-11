//! Eval datasets for reusable AI agents.
//!
//! A dataset is a curated set of cases — the inputs an agent is expected to handle — kept in
//! workspace object storage rather than in Postgres, as two objects per dataset:
//!
//! ```text
//! wmill_eval_datasets/meta/<path>.json     the dataset's own metadata
//! wmill_eval_datasets/cases/<path>.jsonl   one JSON case per line
//! ```
//!
//! Metadata is split from the case bulk so listing datasets never downloads cases.

#[cfg(feature = "parquet")]
use axum::routing::{get, post};
use axum::Router;

#[cfg(feature = "parquet")]
pub use with_storage::*;

pub fn workspaced_service() -> Router {
    #[cfg(feature = "parquet")]
    {
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
            .route("/experiments/results", post(experiment_results))
            .route("/case_draft/from_job/{job_id}", get(case_draft_from_job))
            .route(
                "/case_draft/from_conversation/{conversation_id}",
                get(case_draft_from_conversation),
            )
    }
    #[cfg(not(feature = "parquet"))]
    {
        Router::new()
    }
}

#[cfg(feature = "parquet")]
mod with_storage {
    use std::sync::Arc;

    use axum::{
        extract::{Path, Query},
        Extension, Json,
    };
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use serde_json::value::RawValue;
    use sqlx::Postgres;
    use uuid::Uuid;
    use windmill_common::{
        db::UserDB,
        error::{Error, JsonResult, Result},
        utils::{paginate, Pagination},
    };
    use windmill_object_store::{
        build_object_store_client, object_store_error_to_error,
        object_store_reexports::{ObjectStore, ObjectStoreError, Path as ObjectPath, PutPayload},
        ObjectStoreResource,
    };

    use crate::db::{ApiAuthed, DB};
    use crate::job_helpers_oss::get_workspace_s3_resource;
    use windmill_api_auth::check_scopes;

    const META_PREFIX: &str = "wmill_eval_datasets/meta";
    const CASES_PREFIX: &str = "wmill_eval_datasets/cases";

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

    // -------------------------------------------------------------------------------------------
    // Paths and permissions
    // -------------------------------------------------------------------------------------------

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

    /// Object storage has no per-object ACL, so a dataset's permissions are the permissions of the
    /// Windmill path it is named by, checked here. There is no per-dataset `extra_perms`, and
    /// anyone who can read the workspace bucket directly can read every dataset — as with any
    /// other workspace file.
    fn require_can_read(authed: &ApiAuthed, path: &str) -> Result<()> {
        check_path(path)?;
        if authed.is_admin {
            return Ok(());
        }
        let segments = path.split('/').collect::<Vec<_>>();
        let readable = match segments[0] {
            "u" => segments[1] == authed.username,
            _ => authed
                .folders
                .iter()
                .any(|(name, _, _)| name == segments[1]),
        };
        if readable {
            Ok(())
        } else {
            Err(Error::NotAuthorized(format!(
                "User {} does not have read access to eval dataset {}",
                authed.username, path
            )))
        }
    }

    fn require_can_write(authed: &ApiAuthed, path: &str) -> Result<()> {
        check_path(path)?;
        if authed.is_operator {
            return Err(Error::NotAuthorized(
                "Operators cannot modify eval datasets".to_string(),
            ));
        }
        if authed.is_admin {
            return Ok(());
        }
        let segments = path.split('/').collect::<Vec<_>>();
        let writable = match segments[0] {
            "u" => segments[1] == authed.username,
            _ => authed
                .folders
                .iter()
                .any(|(name, write, _)| name == segments[1] && *write),
        };
        if writable {
            Ok(())
        } else {
            Err(Error::NotAuthorized(format!(
                "User {} does not have write access to eval dataset {}",
                authed.username, path
            )))
        }
    }

    fn meta_key(path: &str) -> String {
        format!("{}/{}.json", META_PREFIX, path)
    }

    fn cases_key(path: &str) -> String {
        format!("{}/{}.jsonl", CASES_PREFIX, path)
    }

    // -------------------------------------------------------------------------------------------
    // Object storage
    // -------------------------------------------------------------------------------------------

    async fn object_store(
        authed: &ApiAuthed,
        db: &DB,
        user_db: UserDB,
        w_id: &str,
    ) -> Result<Arc<dyn ObjectStore>> {
        let (_, resource) =
            get_workspace_s3_resource(authed, db, Some(user_db), w_id, None).await?;
        let resource: ObjectStoreResource = resource.ok_or_else(|| {
            Error::BadRequest(
                "Eval datasets are stored in the workspace object storage, which is not \
                 configured for this workspace. Set it in workspace settings first."
                    .to_string(),
            )
        })?;
        build_object_store_client(&resource).await
    }

    async fn get_object(client: &Arc<dyn ObjectStore>, key: &str) -> Result<Option<bytes::Bytes>> {
        match client.get(&ObjectPath::from(key)).await {
            Ok(result) => Ok(Some(
                result.bytes().await.map_err(object_store_error_to_error)?,
            )),
            Err(ObjectStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(object_store_error_to_error(e)),
        }
    }

    async fn put_object(client: &Arc<dyn ObjectStore>, key: &str, body: Vec<u8>) -> Result<()> {
        client
            .put(&ObjectPath::from(key), PutPayload::from(body))
            .await
            .map_err(object_store_error_to_error)?;
        Ok(())
    }

    async fn read_dataset(client: &Arc<dyn ObjectStore>, path: &str) -> Result<EvalDataset> {
        let bytes = get_object(client, &meta_key(path))
            .await?
            .ok_or_else(|| Error::NotFound(format!("Eval dataset {} not found", path)))?;
        serde_json::from_slice(&bytes).map_err(|e| {
            Error::internal_err(format!(
                "Corrupted eval dataset metadata for {}: {}",
                path, e
            ))
        })
    }

    /// A blank or absent object is an empty dataset: `create_dataset` writes only the metadata, so
    /// the cases object does not exist until the first case is added.
    async fn read_cases(client: &Arc<dyn ObjectStore>, path: &str) -> Result<Vec<EvalCase>> {
        let Some(bytes) = get_object(client, &cases_key(path)).await? else {
            return Ok(vec![]);
        };
        let text = String::from_utf8(bytes.to_vec()).map_err(|e| {
            Error::internal_err(format!("Corrupted eval cases for {}: {}", path, e))
        })?;
        text.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                serde_json::from_str::<EvalCase>(line).map_err(|e| {
                    Error::internal_err(format!("Corrupted eval case in {}: {}", path, e))
                })
            })
            .collect()
    }

    async fn write_cases(
        client: &Arc<dyn ObjectStore>,
        path: &str,
        cases: &[EvalCase],
    ) -> Result<()> {
        let mut body = Vec::new();
        for case in cases {
            body.extend_from_slice(&serde_json::to_vec(case)?);
            body.push(b'\n');
        }
        put_object(client, &cases_key(path), body).await
    }

    /// Serializes the read-modify-write of one dataset's cases across every API server. Without
    /// it, two cases captured from production runs at the same moment both read the same file and
    /// the second PUT silently drops the first.
    ///
    /// `try` rather than the blocking form because the lock is held across two object-store
    /// round-trips: a hung storage call must fail this one dataset's request rather than park
    /// connections behind it.
    async fn lock_dataset(
        tx: &mut sqlx::Transaction<'_, Postgres>,
        w_id: &str,
        path: &str,
        attempts: u32,
    ) -> Result<()> {
        let key = format!("ai_eval_dataset:{}/{}", w_id, path);
        for attempt in 0..attempts {
            let acquired = sqlx::query_scalar!(
                "SELECT pg_try_advisory_xact_lock(hashtext($1)) AS \"acquired!\"",
                key
            )
            .fetch_one(&mut **tx)
            .await?;
            if acquired {
                return Ok(());
            }
            if attempt + 1 < attempts {
                tokio::time::sleep(LOCK_RETRY_DELAY).await;
            }
        }
        Err(Error::Generic(
            axum::http::StatusCode::CONFLICT,
            format!(
                "Eval dataset {} is being written by another request, try again",
                path
            ),
        ))
    }

    const LOCK_ATTEMPTS: u32 = 20;
    /// Recording an experiment waits longer than a case edit: its jobs are already queued, so
    /// giving up here leaves them running with nothing to attribute them to.
    const LOCK_ATTEMPTS_LAUNCH: u32 = 100;
    const LOCK_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(100);

    /// Take the dataset's write lock, then read-modify-write its cases. The transaction is only
    /// ever a lock holder — the cases themselves live in object storage — so it is committed as
    /// soon as the PUT lands.
    async fn mutate_cases<F>(
        authed: &ApiAuthed,
        db: &DB,
        user_db: UserDB,
        w_id: &str,
        path: &str,
        mutate: F,
    ) -> Result<()>
    where
        F: FnOnce(&mut Vec<EvalCase>) -> Result<()>,
    {
        require_can_write(authed, path)?;
        let client = object_store(authed, db, user_db, w_id).await?;
        let mut tx = db.begin().await?;
        lock_dataset(&mut tx, w_id, path, LOCK_ATTEMPTS).await?;
        // Reading the metadata under the lock is what makes "the dataset exists" hold for the
        // whole mutation, so a concurrent delete cannot resurrect the cases object.
        read_dataset(&client, path).await?;
        let mut cases = read_cases(&client, path).await?;
        mutate(&mut cases)?;
        write_cases(&client, path, &cases).await?;
        tx.commit().await?;
        Ok(())
    }

    // -------------------------------------------------------------------------------------------
    // Datasets
    // -------------------------------------------------------------------------------------------

    pub async fn list_datasets(
        authed: ApiAuthed,
        Extension(db): Extension<DB>,
        Extension(user_db): Extension<UserDB>,
        Path(w_id): Path<String>,
    ) -> JsonResult<Vec<EvalDataset>> {
        use futures::{StreamExt, TryStreamExt};

        let client = object_store(&authed, &db, user_db, &w_id).await?;
        let keys: Vec<String> = client
            .list(Some(&ObjectPath::from(META_PREFIX)))
            .map_ok(|meta| meta.location.to_string())
            .try_collect()
            .await
            .map_err(object_store_error_to_error)?;

        let readable = keys
            .iter()
            .filter_map(|key| {
                key.strip_prefix(&format!("{}/", META_PREFIX))
                    .and_then(|p| p.strip_suffix(".json"))
            })
            .filter(|path| require_can_read(&authed, path).is_ok())
            .map(|path| path.to_string())
            .collect::<Vec<_>>();

        // Fetched concurrently: the metadata is one object per dataset, and doing them in
        // sequence makes listing cost a round-trip per dataset.
        let mut datasets = futures::stream::iter(readable.into_iter().map(|path| {
            let client = client.clone();
            async move {
                let result = read_dataset(&client, &path).await;
                (path, result)
            }
        }))
        .buffer_unordered(16)
        .filter_map(|(path, result)| async move {
            match result {
                Ok(dataset) => Some(dataset),
                // One unreadable object must not blank the whole list.
                Err(e) => {
                    tracing::warn!("skipping eval dataset {}: {}", path, e);
                    None
                }
            }
        })
        .collect::<Vec<_>>()
        .await;
        datasets.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(Json(datasets))
    }

    pub async fn create_dataset(
        authed: ApiAuthed,
        Extension(db): Extension<DB>,
        Extension(user_db): Extension<UserDB>,
        Path(w_id): Path<String>,
        Json(payload): Json<CreateDataset>,
    ) -> Result<String> {
        require_can_write(&authed, &payload.path)?;
        let client = object_store(&authed, &db, user_db, &w_id).await?;
        // Under the same lock as every other writer: the exists-check and the write must not
        // straddle a concurrent delete, or the delete's second DELETE lands on a dataset this
        // request just created.
        let mut tx = db.begin().await?;
        lock_dataset(&mut tx, &w_id, &payload.path, LOCK_ATTEMPTS).await?;
        if get_object(&client, &meta_key(&payload.path))
            .await?
            .is_some()
        {
            return Err(Error::BadRequest(format!(
                "Eval dataset {} already exists",
                payload.path
            )));
        }
        let now = Utc::now();
        let dataset = EvalDataset {
            path: payload.path.clone(),
            summary: payload.summary,
            description: payload.description,
            default_subject: payload.default_subject,
            created_at: now,
            created_by: authed.username.clone(),
            edited_at: now,
            edited_by: authed.username.clone(),
        };
        put_object(
            &client,
            &meta_key(&payload.path),
            serde_json::to_vec(&dataset)?,
        )
        .await?;
        tx.commit().await?;
        Ok(format!("Created eval dataset {}", payload.path))
    }

    pub async fn get_dataset(
        authed: ApiAuthed,
        Extension(db): Extension<DB>,
        Extension(user_db): Extension<UserDB>,
        Path((w_id, path)): Path<(String, String)>,
    ) -> JsonResult<EvalDataset> {
        require_can_read(&authed, &path)?;
        let client = object_store(&authed, &db, user_db, &w_id).await?;
        Ok(Json(read_dataset(&client, &path).await?))
    }

    pub async fn update_dataset(
        authed: ApiAuthed,
        Extension(db): Extension<DB>,
        Extension(user_db): Extension<UserDB>,
        Path((w_id, path)): Path<(String, String)>,
        Json(payload): Json<EditDataset>,
    ) -> Result<String> {
        require_can_write(&authed, &path)?;
        let client = object_store(&authed, &db, user_db, &w_id).await?;
        let mut tx = db.begin().await?;
        lock_dataset(&mut tx, &w_id, &path, LOCK_ATTEMPTS).await?;
        let mut dataset = read_dataset(&client, &path).await?;
        dataset.summary = payload.summary;
        dataset.description = payload.description;
        dataset.default_subject = payload.default_subject;
        dataset.edited_at = Utc::now();
        dataset.edited_by = authed.username.clone();
        put_object(&client, &meta_key(&path), serde_json::to_vec(&dataset)?).await?;
        tx.commit().await?;
        Ok(format!("Updated eval dataset {}", path))
    }

    pub async fn delete_dataset(
        authed: ApiAuthed,
        Extension(db): Extension<DB>,
        Extension(user_db): Extension<UserDB>,
        Path((w_id, path)): Path<(String, String)>,
    ) -> Result<String> {
        require_can_write(&authed, &path)?;
        let client = object_store(&authed, &db, user_db, &w_id).await?;
        let mut tx = db.begin().await?;
        lock_dataset(&mut tx, &w_id, &path, LOCK_ATTEMPTS).await?;
        // Experiments, then cases, then metadata. The metadata is what `create_dataset` checks
        // for, so deleting it first would let a failure partway through leave case copies behind
        // *and* unblock recreating the path — the new dataset would open holding the old one's
        // data. This order leaves an empty dataset instead, which is visible and retryable.
        //
        // `list` matches on whole path segments, so this prefix cannot reach a sibling dataset
        // whose path merely starts with the same characters (`f/t/foo` vs `f/t/foobar`). Anything
        // that replaces it with string-prefix filtering would delete the sibling's experiments.
        use futures::TryStreamExt;
        let experiment_keys: Vec<String> = client
            .list(Some(&ObjectPath::from(
                format!("{}/{}", EXPERIMENTS_PREFIX, path).as_str(),
            )))
            .map_ok(|meta| meta.location.to_string())
            .try_collect()
            .await
            .map_err(object_store_error_to_error)?;

        for key in experiment_keys
            .into_iter()
            .chain([cases_key(&path), meta_key(&path)])
        {
            match client.delete(&ObjectPath::from(key.as_str())).await {
                Ok(()) => {}
                Err(ObjectStoreError::NotFound { .. }) => {}
                Err(e) => return Err(object_store_error_to_error(e)),
            }
        }
        tx.commit().await?;
        Ok(format!("Deleted eval dataset {}", path))
    }

    // -------------------------------------------------------------------------------------------
    // Cases
    // -------------------------------------------------------------------------------------------

    pub async fn list_cases(
        authed: ApiAuthed,
        Extension(db): Extension<DB>,
        Extension(user_db): Extension<UserDB>,
        Path((w_id, path)): Path<(String, String)>,
        Query(pagination): Query<Pagination>,
    ) -> JsonResult<ListCasesResponse> {
        require_can_read(&authed, &path)?;
        let client = object_store(&authed, &db, user_db, &w_id).await?;
        read_dataset(&client, &path).await?;
        let cases = read_cases(&client, &path).await?;
        let total = cases.len();
        let (per_page, offset) = paginate(pagination);
        let page = cases.into_iter().skip(offset).take(per_page).collect();
        Ok(Json(ListCasesResponse { cases: page, total }))
    }

    pub async fn add_case(
        authed: ApiAuthed,
        Extension(db): Extension<DB>,
        Extension(user_db): Extension<UserDB>,
        Path((w_id, path)): Path<(String, String)>,
        Json(payload): Json<NewEvalCase>,
    ) -> Result<String> {
        let id = Uuid::new_v4();
        let case = EvalCase {
            id,
            name: payload.name,
            input: payload.input,
            host_flow_path: payload.host_flow_path,
            tool_inputs: payload.tool_inputs,
            expected: payload.expected,
            tags: payload.tags,
            source: payload.source,
            created_at: Utc::now(),
            created_by: authed.username.clone(),
        };
        mutate_cases(&authed, &db, user_db, &w_id, &path, move |cases| {
            cases.push(case);
            Ok(())
        })
        .await?;
        Ok(id.to_string())
    }

    pub async fn update_case(
        authed: ApiAuthed,
        Extension(db): Extension<DB>,
        Extension(user_db): Extension<UserDB>,
        Path((w_id, path)): Path<(String, String)>,
        Json(payload): Json<UpdateCase>,
    ) -> Result<String> {
        let UpdateCase { id, name, input, host_flow_path, tool_inputs, expected, tags } = payload;
        mutate_cases(&authed, &db, user_db, &w_id, &path, move |cases| {
            let case = cases
                .iter_mut()
                .find(|c| c.id == id)
                .ok_or_else(|| Error::NotFound(format!("Eval case {} not found", id)))?;
            case.name = name;
            case.input = input;
            case.host_flow_path = host_flow_path;
            case.tool_inputs = tool_inputs;
            case.expected = expected;
            case.tags = tags;
            Ok(())
        })
        .await?;
        Ok(format!("Updated eval case {}", id))
    }

    pub async fn delete_case(
        authed: ApiAuthed,
        Extension(db): Extension<DB>,
        Extension(user_db): Extension<UserDB>,
        Path((w_id, path)): Path<(String, String)>,
        Json(payload): Json<CaseId>,
    ) -> Result<String> {
        let id = payload.id;
        mutate_cases(&authed, &db, user_db, &w_id, &path, move |cases| {
            let before = cases.len();
            cases.retain(|c| c.id != id);
            if cases.len() == before {
                return Err(Error::NotFound(format!("Eval case {} not found", id)));
            }
            Ok(())
        })
        .await?;
        Ok(format!("Deleted eval case {}", id))
    }

    // -------------------------------------------------------------------------------------------
    // Standalone runs
    // -------------------------------------------------------------------------------------------

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
                self.path.rsplit('/').next().unwrap_or(&self.path).to_string()
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
                                "JSON.stringify({{ input: flow_input.user_message, output: {}, expected: flow_input.expected }})",
                                output_expr
                            ),
                        }
                    }
                }),
                ScorerKind::Script | ScorerKind::Flow => serde_json::json!({
                    "type": if scorer.kind == ScorerKind::Script { "script" } else { "flow" },
                    "path": scorer.path,
                    "input_transforms": {
                        "input": { "type": "javascript", "expr": "flow_input.user_message" },
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

        // Resolve the case before anything else: a stored case is the source of truth for what
        // ran, so an inline body must not be able to override one.
        let case = match (&payload.dataset, payload.case_id) {
            (Some(dataset), Some(case_id)) => {
                require_can_read(&authed, dataset)?;
                let client = object_store(&authed, &db, user_db.clone(), &w_id).await?;
                let cases = read_cases(&client, dataset).await?;
                let stored = cases
                    .into_iter()
                    .find(|c| c.id == case_id)
                    .ok_or_else(|| Error::NotFound(format!("Eval case {} not found", case_id)))?;
                NewEvalCase {
                    name: stored.name,
                    input: stored.input,
                    host_flow_path: stored.host_flow_path,
                    tool_inputs: stored.tool_inputs,
                    expected: stored.expected,
                    tags: stored.tags,
                    source: stored.source,
                }
            }
            (None, None) => payload.case.ok_or_else(|| {
                Error::BadRequest(
                    "Either a case or a dataset and case_id must be supplied".to_string(),
                )
            })?,
            _ => {
                return Err(Error::BadRequest(
                    "dataset and case_id must be supplied together".to_string(),
                ))
            }
        };

        require_agent(&authed, &user_db, &w_id, &agent_path).await?;

        // The version the agent is at now. Recorded so the run stays attributable to a prompt
        // state later; it does not pin execution, which stays live.
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
        )
        .await?;
        Ok((axum::http::StatusCode::CREATED, uuid.to_string()))
    }


    // -------------------------------------------------------------------------------------------
    // Experiments
    // -------------------------------------------------------------------------------------------

    const EXPERIMENTS_PREFIX: &str = "wmill_eval_datasets/experiments";

    fn experiment_key(dataset: &str, id: Uuid) -> String {
        format!("{}/{}/{}.json", EXPERIMENTS_PREFIX, dataset, id)
    }

    /// One run of a dataset. The cases are stored by value, not by reference: a dataset keeps
    /// changing, and a result set that cannot say which inputs produced it is not reproducible.
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct EvalExperiment {
        pub id: Uuid,
        pub dataset: String,
        pub subject: EvalSubject,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub scorers: Vec<ScorerRef>,
        /// The exact case set that ran, and the job each one produced.
        pub cases: Vec<ExperimentCase>,
        pub created_at: DateTime<Utc>,
        pub created_by: String,
    }

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
        if authed.is_operator {
            return Err(Error::NotAuthorized(
                "Operators cannot run experiments".to_string(),
            ));
        }
        check_scopes(&authed, || "jobs:run".to_string())?;
        // A write, not a read: it persists an experiment into the dataset's namespace and its
        // shared list.
        require_can_write(&authed, &payload.dataset)?;

        let agent_path = payload.subject.path.clone();
        require_agent(&authed, &user_db, &w_id, &agent_path).await?;
        let version = current_resource_version(&db, &w_id, &agent_path).await?;

        let client = object_store(&authed, &db, user_db.clone(), &w_id).await?;
        // Read the case set under the lock, then release it for the pushes: holding it across the
        // whole launch would 409 every capture and case edit on this dataset until the last job
        // was queued. The write below retakes it.
        let (cases, launched_against) = {
            let mut tx = db.begin().await?;
            lock_dataset(&mut tx, &w_id, &payload.dataset, LOCK_ATTEMPTS).await?;
            let dataset = read_dataset(&client, &payload.dataset).await?;
            let cases = read_cases(&client, &payload.dataset).await?;
            tx.commit().await?;
            (cases, dataset.created_at)
        };
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

        let experiment_id = Uuid::new_v4();
        let mut experiment_cases = Vec::with_capacity(cases.len());
        for case in cases {
            let new_case = NewEvalCase {
                name: case.name.clone(),
                input: case.input.clone(),
                host_flow_path: case.host_flow_path.clone(),
                tool_inputs: case.tool_inputs.clone(),
                expected: case.expected.clone(),
                tags: case.tags.clone(),
                source: case.source.clone(),
            };
            let job_id = push_case_run(
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
            )
            .await?;
            experiment_cases.push(ExperimentCase {
                case_id: case.id,
                name: case.name,
                input: case.input,
                expected: case.expected,
                job_id,
            });
        }

        let experiment = EvalExperiment {
            id: experiment_id,
            dataset: payload.dataset.clone(),
            subject: EvalSubject {
                kind: EvalSubjectKind::Agent,
                path: agent_path,
                version,
            },
            scorers: payload.scorers,
            cases: experiment_cases,
            created_at: Utc::now(),
            created_by: authed.username.clone(),
        };
        // Retaken for the write, and the dataset re-checked under it. Identity, not existence:
        // the path can have been deleted and recreated while the jobs were pushed, and this
        // experiment holds copies of the *old* dataset's cases. A longer lock budget than a case
        // edit gets, because the jobs are already queued and failing here strands them.
        let mut tx = db.begin().await?;
        lock_dataset(&mut tx, &w_id, &payload.dataset, LOCK_ATTEMPTS_LAUNCH)
            .await
            .map_err(|_| {
                Error::internal_err(format!(
                    "Eval dataset {} stayed locked by another writer; this experiment's {} jobs are \
                     running but no experiment was recorded for them. Do not retry — that would run \
                     the dataset again on top of them.",
                    payload.dataset,
                    experiment.cases.len()
                ))
            })?;
        let still_there = match read_dataset(&client, &payload.dataset).await {
            Ok(dataset) => Some(dataset.created_at),
            // Only an absence means deleted. A storage fault must surface as itself rather than as
            // a phantom delete.
            Err(Error::NotFound(_)) => None,
            Err(e) => return Err(e),
        };
        if still_there != Some(launched_against) {
            return Err(Error::BadRequest(format!(
                "Eval dataset {} was deleted or recreated while the experiment was starting; its \
                 {} jobs are running but no experiment was recorded for them",
                payload.dataset,
                experiment.cases.len()
            )));
        }
        put_object(
            &client,
            &experiment_key(&payload.dataset, experiment_id),
            serde_json::to_vec(&experiment)?,
        )
        .await?;
        tx.commit().await?;
        Ok(experiment_id.to_string())
    }

    pub async fn list_experiments(
        authed: ApiAuthed,
        Extension(db): Extension<DB>,
        Extension(user_db): Extension<UserDB>,
        Path((w_id, dataset)): Path<(String, String)>,
    ) -> JsonResult<Vec<EvalExperiment>> {
        use futures::{StreamExt, TryStreamExt};

        require_can_read(&authed, &dataset)?;
        let client = object_store(&authed, &db, user_db, &w_id).await?;
        let prefix = format!("{}/{}", EXPERIMENTS_PREFIX, dataset);
        let keys: Vec<String> = client
            .list(Some(&ObjectPath::from(prefix.as_str())))
            .map_ok(|meta| meta.location.to_string())
            .try_collect()
            .await
            .map_err(object_store_error_to_error)?;

        let mut experiments = futures::stream::iter(keys.into_iter().map(|key| {
            let client = client.clone();
            async move {
                let bytes = get_object(&client, &key).await.ok().flatten()?;
                serde_json::from_slice::<EvalExperiment>(&bytes).ok()
            }
        }))
        .buffer_unordered(16)
        .filter_map(|e| async move { e })
        .collect::<Vec<_>>()
        .await;
        experiments.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(Json(experiments))
    }

    #[derive(Deserialize)]
    pub struct ExperimentRef {
        pub dataset: String,
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
                // An agent scorer answers in `output`, which is itself often a JSON string.
                _ => match map.get("output") {
                    Some(serde_json::Value::Number(n)) => n.as_f64(),
                    Some(serde_json::Value::String(s)) => s
                        .trim()
                        .parse::<f64>()
                        .ok()
                        .or_else(|| serde_json::from_str::<serde_json::Value>(s).ok().and_then(
                            |v| v.get("score").and_then(|s| s.as_f64()),
                        )),
                    _ => None,
                },
            },
            _ => None,
        }
    }

    pub async fn experiment_results(
        authed: ApiAuthed,
        Extension(db): Extension<DB>,
        Extension(user_db): Extension<UserDB>,
        Path(w_id): Path<String>,
        Json(payload): Json<ExperimentRef>,
    ) -> JsonResult<ExperimentResults> {
        require_can_read(&authed, &payload.dataset)?;
        let client = object_store(&authed, &db, user_db, &w_id).await?;
        let bytes = get_object(&client, &experiment_key(&payload.dataset, payload.id))
            .await?
            .ok_or_else(|| Error::NotFound(format!("Experiment {} not found", payload.id)))?;
        let experiment: EvalExperiment = serde_json::from_slice(&bytes)?;

        // The experiment object lives in workspace object storage, which a script can write
        // directly — so its job ids are caller-controlled. Results below are read on the
        // unrestricted pool, so a forged experiment naming someone else's flow job would hand back
        // output the jobs API would refuse. Only jobs this server stamped for *this* experiment
        // are read; anything else is reported as if it had not run.
        let job_ids: Vec<Uuid> = experiment.cases.iter().map(|c| c.job_id).collect();
        let ours = sqlx::query_scalar!(
            "SELECT id FROM v2_job
             WHERE id = ANY($1) AND workspace_id = $2
               AND args->'_eval'->>'experiment_id' = $3",
            &job_ids,
            &w_id,
            experiment.id.to_string()
        )
        .fetch_all(&db)
        .await?
        .into_iter()
        .collect::<std::collections::HashSet<Uuid>>();

        // One query for every case job's own status: reading the agent step's success reported a
        // case as successful even when a scorer step had failed.
        let statuses = sqlx::query!(
            "SELECT id, status::text AS \"status!\" FROM v2_job_completed
             WHERE id = ANY($1) AND workspace_id = $2",
            &job_ids,
            &w_id
        )
        .fetch_all(&db)
        .await?
        .into_iter()
        .filter(|r| ours.contains(&r.id))
        .map(|r| (r.id, r.status))
        .collect::<std::collections::HashMap<_, _>>();

        // Bounded concurrency rather than one await after another: a 100-case experiment with
        // three scorers is 400 lookups, and each helper call is itself several queries.
        use futures::StreamExt;
        let scorer_count = experiment.scorers.len();
        let rows = futures::stream::iter(experiment.cases.clone().into_iter().map(|case| {
            let db = db.clone();
            let w_id = w_id.clone();
            let is_ours = ours.contains(&case.job_id);
            async move {
                if !is_ours {
                    return (case, None, vec![None; scorer_count]);
                }
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

                // Sequential within a case, concurrent across cases: nesting a second bounded
                // stream here would multiply the two bounds into 32 in-flight queries against a
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

    // -------------------------------------------------------------------------------------------
    // Capturing a case from real traffic
    // -------------------------------------------------------------------------------------------

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
                host_flow_path = parent.runnable_path.clone();
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
        let conversation = conversation.ok_or_else(|| {
            Error::NotFound(format!("Conversation {} not found", conversation_id))
        })?;

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
}
