use super::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvalDataset {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<Box<RawValue>>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
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
    pub expected: Option<Box<RawValue>>,
}

#[derive(Deserialize)]
pub struct CreateDataset {
    pub path: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub scorers: Vec<Scorer>,
    /// The cases to create it holding. A case is a row of the dataset, so unlike a scorer it
    /// cannot be written before there is a dataset for it to be a row of; sending them with the
    /// dataset is what lets one be assembled in a single act rather than created empty and filled
    /// in afterwards.
    #[serde(default)]
    pub cases: Vec<NewEvalCase>,
}

#[derive(Deserialize)]
pub struct EditDataset {
    /// Renames the dataset. Its cases and experiments follow through the foreign keys, so a
    /// rename is a rename rather than a copy that leaves history behind.
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    /// Left out to keep the dataset's columns as they are; sent to replace them wholesale.
    #[serde(default)]
    pub scorers: Option<Vec<Scorer>>,
    /// The cases as they should stand afterwards, as `save_cases` takes them. Sent with the rest
    /// of an edit so that a rename the dataset refuses refuses the case edits with it, rather
    /// than leaving them written under a dataset that kept its name.
    #[serde(default)]
    pub cases: Option<Vec<SaveCase>>,
}

#[derive(Deserialize)]
pub struct CaseId {
    pub id: Uuid,
}

/// A dataset's cases as the editor holds them, which is how the editor writes them: all of them,
/// at once. A case carrying an `id` is one the dataset already has; one without is new, and the
/// id it is given comes back so a save that is retried updates it rather than adding it twice.
#[derive(Deserialize)]
pub struct SaveCases {
    pub cases: Vec<SaveCase>,
}

#[derive(Deserialize)]
pub struct SaveCase {
    #[serde(default)]
    pub id: Option<Uuid>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub input: EvalCaseInput,
    #[serde(default)]
    pub expected: Option<Box<RawValue>>,
}

#[derive(Deserialize)]
/// The edit fields are spelled out rather than `#[serde(flatten)]`-ing `NewEvalCase`: flatten
/// deserializes through a buffered representation, which silently yields `None` for the
/// `Box<RawValue>` fields — an edited case would lose its attachments.
pub struct UpdateCase {
    pub id: Uuid,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub input: EvalCaseInput,
    #[serde(default)]
    pub expected: Option<Box<RawValue>>,
}

#[derive(Serialize)]
pub struct ListCasesResponse {
    pub cases: Vec<EvalCase>,
    pub total: usize,
}
// -----------------------------------------------------------------------------------------------
// Paths and permissions
// -----------------------------------------------------------------------------------------------

pub async fn list_datasets(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<EvalDataset>> {
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query!(
        "SELECT path, summary, scorers, created_at, created_by,
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
    check_summary(payload.summary.as_deref())?;
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot create eval datasets".to_string(),
        ));
    }
    // The dataset and its cases are written in one `user_db` transaction: the row's insert policy
    // gates the dataset, the cases' insert policy gates each case, and the two land together or
    // not at all. Nothing decides access a second time in Rust.
    if payload.cases.len() as i64 > MAX_CASES_PER_DATASET {
        return Err(Error::BadRequest(format!(
            "An eval dataset holds at most {} cases. Split them into several datasets.",
            MAX_CASES_PER_DATASET
        )));
    }
    let mut total = 0usize;
    for case in &payload.cases {
        check_case(case.name.as_deref(), &case.input, case.expected.as_ref())?;
        total += case_bytes(&case.input, case.expected.as_ref())?;
    }
    check_dataset_bytes(total)?;
    let mut scorers = payload.scorers;
    // A dataset being created has no columns yet, so every id is minted.
    assign_scorer_ids(&mut scorers, &std::collections::HashSet::new())?;
    let scorers = serde_json::to_value(&scorers)?;
    let mut tx = user_db.begin(&authed).await?;
    // A path already taken returns no row; a path the caller may not create raises the insert
    // policy, which `map_rls_denied` turns into an access error.
    let created = sqlx::query_scalar!(
        "INSERT INTO eval_dataset
            (workspace_id, path, summary, scorers, created_by, edited_by)
         VALUES ($1, $2, $3, $4, $5, $5)
         ON CONFLICT (workspace_id, path) DO NOTHING
         RETURNING path",
        w_id,
        payload.path,
        payload.summary,
        scorers,
        authed.username,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| map_rls_denied(&payload.path, "create", e))?;
    if created.is_none() {
        return Err(Error::BadRequest(format!(
            "Eval dataset {} already exists",
            payload.path
        )));
    }
    for case in &payload.cases {
        sqlx::query!(
            "INSERT INTO eval_case
                    (workspace_id, dataset_path, name, input, expected, created_by)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            w_id,
            payload.path,
            case.name,
            serde_json::to_value(&case.input)?,
            opt_from_raw(case.expected.as_ref())?,
            authed.username,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(format!("Created eval dataset {}", payload.path))
}

pub async fn get_dataset(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, String)>,
) -> JsonResult<EvalDataset> {
    Ok(Json(read_dataset(&authed, &user_db, &w_id, &path).await?))
}

/// An edit is one transaction: the rename, the summary, the columns and the cases land together
/// or not at all, so a case the list no longer holds, or a name already taken, refuses the whole
/// save rather than half of it.
pub async fn update_dataset(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, String)>,
    Json(payload): Json<EditDataset>,
) -> Result<String> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot modify eval datasets".to_string(),
        ));
    }
    check_summary(payload.summary.as_deref())?;
    let new_path = match payload.path.filter(|p| *p != path) {
        Some(new_path) => {
            check_path(&new_path)?;
            Some(new_path)
        }
        None => None,
    };
    if let Some(cases) = &payload.cases {
        check_cases(cases)?;
    }
    // The whole edit is one `user_db` transaction, governed by the row-level policies throughout:
    // the row is read `FOR UPDATE` (its UPDATE policy decides who may), the columns it holds are
    // read under that lock so a concurrent edit cannot restore a removed scorer's id, and the
    // cases move under the (possibly new) name through the case write policies. The rename's
    // destination is checked too — the dataset UPDATE policies carry no explicit `WITH CHECK`, so
    // Postgres reuses their `USING` as one, and a rename into a path the caller cannot write is
    // refused (into their own namespace is allowed, as for any resource). Nothing is decided in Rust.
    let mut tx = user_db.clone().begin(&authed).await?;
    // The case advisory lock first, then the row `FOR UPDATE`: `add_case`/`save_cases` take the
    // advisory lock before their `eval_case` writes touch the dataset row's foreign key, so every
    // path that holds both must take them in this order or a concurrent edit and case-add deadlock.
    lock_dataset_cases(&mut tx, &w_id, &path).await?;
    let current = sqlx::query_scalar!(
        "SELECT scorers FROM eval_dataset WHERE workspace_id = $1 AND path = $2 FOR UPDATE",
        w_id,
        path
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(current) = current else {
        drop(tx);
        return Err(write_refused(&authed, &user_db, &w_id, &path).await);
    };
    let existing: std::collections::HashSet<String> =
        serde_json::from_value::<Vec<Scorer>>(current)
            .map(|scorers| scorers.into_iter().map(|s| s.id).collect())
            .unwrap_or_default();
    let scorers = match payload.scorers {
        Some(mut scorers) => {
            assign_scorer_ids(&mut scorers, &existing)?;
            Some(serde_json::to_value(&scorers)?)
        }
        None => None,
    };

    let updated = sqlx::query_scalar!(
        "UPDATE eval_dataset
         SET path = COALESCE($6, path), summary = $3,
             scorers = COALESCE($4, scorers), edited_at = now(), edited_by = $5
         WHERE workspace_id = $1 AND path = $2
         RETURNING path",
        w_id,
        path,
        payload.summary,
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
            map_rls_denied(new_path.as_deref().unwrap_or(&path), "rename", e)
        }
    })?;
    // No row updated: the caller cannot write this dataset (its UPDATE policy denied the row) or it
    // is gone. A refused rename destination raises 42501 instead, handled just above.
    let Some(updated) = updated else {
        drop(tx);
        return Err(write_refused(&authed, &user_db, &w_id, &path).await);
    };
    // Under the name the dataset now has: the cases followed the rename through the foreign key.
    if let Some(cases) = &payload.cases {
        write_cases(&mut tx, &w_id, &updated, cases, &authed.username).await?;
    }
    tx.commit().await?;
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

pub(crate) async fn read_cases(
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
        "SELECT id, name, input, expected, created_at, created_by
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
                expected: opt_to_raw(row.expected)?,
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

/// Serializes everything that writes a dataset's case set, held for the rest of the transaction.
///
/// Two whole-set replacements would otherwise each delete what its own list does not hold before
/// either inserts, leaving the union of both rather than either. An addition takes the same lock so
/// that counting the set and adding to it is one step: two additions at the cap would otherwise
/// both read one under it and both insert.
async fn lock_dataset_cases(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    w_id: &str,
    path: &str,
) -> Result<()> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtext('ai_eval_cases:' || $1 || '/' || $2))",
        w_id,
        path
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn add_case(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, String)>,
    Json(payload): Json<NewEvalCase>,
) -> Result<String> {
    require_dataset_writable(&authed, &user_db, &w_id, &path).await?;
    check_case(
        payload.name.as_deref(),
        &payload.input,
        payload.expected.as_ref(),
    )?;

    let mut tx = user_db.begin(&authed).await?;
    lock_dataset_cases(&mut tx, &w_id, &path).await?;
    let count = sqlx::query_scalar!(
        "SELECT count(*) AS \"count!\" FROM eval_case WHERE workspace_id = $1 AND dataset_path = $2",
        w_id,
        path
    )
    .fetch_one(&mut *tx)
    .await?;
    if count >= MAX_CASES_PER_DATASET {
        return Err(Error::BadRequest(format!(
            "Eval dataset {} already holds {} cases, the maximum. Split it into several datasets.",
            path, MAX_CASES_PER_DATASET
        )));
    }
    // The dataset's own bytes plus this case's, so incremental adds cannot walk a dataset past the
    // aggregate cap one case at a time. Read under the same lock as the count.
    let existing_bytes = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(octet_length(input::text) + COALESCE(octet_length(expected::text), 0)), 0) AS \"bytes!\"
         FROM eval_case WHERE workspace_id = $1 AND dataset_path = $2",
        w_id,
        path
    )
    .fetch_one(&mut *tx)
    .await?;
    check_dataset_bytes(
        existing_bytes as usize + case_bytes(&payload.input, payload.expected.as_ref())?,
    )?;

    let id = sqlx::query_scalar!(
        "INSERT INTO eval_case
            (workspace_id, dataset_path, name, input, expected, created_by)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id",
        w_id,
        path,
        payload.name,
        serde_json::to_value(&payload.input)?,
        opt_from_raw(payload.expected.as_ref())?,
        authed.username,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if is_missing_dataset(&e) {
            Error::NotFound(format!("Eval dataset {} not found", path))
        } else {
            e.into()
        }
    })?;
    tx.commit().await?;
    Ok(id.to_string())
}

/// Replace a dataset's cases with the list sent, in one transaction.
///
/// The editor holds every case at once and saves them together, so this is one write rather than
/// one per case: a save that wrote a prefix would leave the dataset in a state nobody asked for,
/// and the cases it had already written would arrive again as new ones when the save was retried.
pub async fn save_cases(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, String)>,
    Json(payload): Json<SaveCases>,
) -> JsonResult<Vec<Uuid>> {
    require_dataset_writable(&authed, &user_db, &w_id, &path).await?;
    check_cases(&payload.cases)?;
    let mut tx = user_db.begin(&authed).await?;
    let ids = write_cases(&mut tx, &w_id, &path, &payload.cases, &authed.username).await?;
    tx.commit().await?;
    Ok(Json(ids))
}

/// What a whole list of cases can be refused for, before any of it is written.
fn check_cases(cases: &[SaveCase]) -> Result<()> {
    // The list is what the dataset holds afterwards, so its own length is the count to weigh.
    if cases.len() as i64 > MAX_CASES_PER_DATASET {
        return Err(Error::BadRequest(format!(
            "An eval dataset holds at most {} cases. Split them into several datasets.",
            MAX_CASES_PER_DATASET
        )));
    }
    let mut total = 0usize;
    for case in cases {
        check_case(case.name.as_deref(), &case.input, case.expected.as_ref())?;
        total += case_bytes(&case.input, case.expected.as_ref())?;
    }
    check_dataset_bytes(total)?;
    // One row per id: the same id twice would write one row twice and return a list longer than
    // the dataset it describes, and the save would read as having kept a case it dropped.
    let mut ids: Vec<Uuid> = cases.iter().filter_map(|c| c.id).collect();
    ids.sort();
    let submitted = ids.len();
    ids.dedup();
    if ids.len() != submitted {
        return Err(Error::BadRequest(
            "A case id appears more than once in the dataset".to_string(),
        ));
    }
    Ok(())
}

/// Replace a dataset's cases with `cases`, in the caller's transaction: rows not in the list go,
/// rows carrying an id are updated, the rest are added. Returns one id per case, in order.
async fn write_cases(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    w_id: &str,
    path: &str,
    cases: &[SaveCase],
    username: &str,
) -> Result<Vec<Uuid>> {
    lock_dataset_cases(tx, w_id, path).await?;
    let kept: Vec<Uuid> = cases.iter().filter_map(|c| c.id).collect();
    sqlx::query!(
        "DELETE FROM eval_case
         WHERE workspace_id = $1 AND dataset_path = $2 AND NOT (id = ANY($3))",
        w_id,
        path,
        &kept
    )
    .execute(&mut **tx)
    .await?;

    let mut ids = Vec::with_capacity(cases.len());
    for case in cases {
        let input = serde_json::to_value(&case.input)?;
        let expected = opt_from_raw(case.expected.as_ref())?;
        let id = match case.id {
            Some(id) => sqlx::query_scalar!(
                "UPDATE eval_case SET name = $4, input = $5, expected = $6
                 WHERE workspace_id = $1 AND dataset_path = $2 AND id = $3
                 RETURNING id",
                w_id,
                path,
                id,
                case.name,
                input,
                expected,
            )
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Eval case {} not found in {}", id, path)))?,
            None => sqlx::query_scalar!(
                "INSERT INTO eval_case
                    (workspace_id, dataset_path, name, input, expected, created_by)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 RETURNING id",
                w_id,
                path,
                case.name,
                input,
                expected,
                username,
            )
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| {
                if is_missing_dataset(&e) {
                    Error::NotFound(format!("Eval dataset {} not found", path))
                } else {
                    e.into()
                }
            })?,
        };
        ids.push(id);
    }
    Ok(ids)
}

pub async fn update_case(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, String)>,
    Json(payload): Json<UpdateCase>,
) -> Result<String> {
    require_dataset_writable(&authed, &user_db, &w_id, &path).await?;
    check_case(
        payload.name.as_deref(),
        &payload.input,
        payload.expected.as_ref(),
    )?;
    let mut tx = user_db.begin(&authed).await?;
    lock_dataset_cases(&mut tx, &w_id, &path).await?;
    // The dataset's other cases plus this one's new bytes: replacing a case must not carry the
    // dataset over the aggregate cap either.
    let others_bytes = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(octet_length(input::text) + COALESCE(octet_length(expected::text), 0)), 0) AS \"bytes!\"
         FROM eval_case WHERE workspace_id = $1 AND dataset_path = $2 AND id <> $3",
        w_id,
        path,
        payload.id
    )
    .fetch_one(&mut *tx)
    .await?;
    check_dataset_bytes(
        others_bytes as usize + case_bytes(&payload.input, payload.expected.as_ref())?,
    )?;
    let updated = sqlx::query_scalar!(
        "UPDATE eval_case
         SET name = $4, input = $5, expected = $6
         WHERE workspace_id = $1 AND dataset_path = $2 AND id = $3
         RETURNING id",
        w_id,
        path,
        payload.id,
        payload.name,
        serde_json::to_value(&payload.input)?,
        opt_from_raw(payload.expected.as_ref())?,
    )
    .fetch_optional(&mut *tx)
    .await?;
    if updated.is_none() {
        return Err(Error::NotFound(format!(
            "Eval case {} not found in {}",
            payload.id, path
        )));
    }
    tx.commit().await?;
    Ok(format!("Updated eval case {}", payload.id))
}

pub async fn delete_case(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, String)>,
    Json(payload): Json<CaseId>,
) -> Result<String> {
    require_dataset_writable(&authed, &user_db, &w_id, &path).await?;
    let mut tx = user_db.begin(&authed).await?;
    let deleted = sqlx::query_scalar!(
        "DELETE FROM eval_case
         WHERE workspace_id = $1 AND dataset_path = $2 AND id = $3
         RETURNING id",
        w_id,
        path,
        payload.id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
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
