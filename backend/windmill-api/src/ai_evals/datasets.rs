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
    Extension(db): Extension<DB>,
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
    // Every case is checked before the dataset is written, so that the only step that can fail is
    // the first one. `eval_case` grants users no write, so the cases cannot be inserted in the
    // transaction that creates the dataset under the caller's own policies — validating first is
    // what keeps "created holding these cases" from becoming "created, holding some of them".
    if payload.cases.len() as i64 > MAX_CASES_PER_DATASET {
        return Err(Error::BadRequest(format!(
            "An eval dataset holds at most {} cases. Split them into several datasets.",
            MAX_CASES_PER_DATASET
        )));
    }
    for case in &payload.cases {
        check_case_size(&case.input, case.expected.as_ref())?;
    }
    let mut scorers = payload.scorers;
    assign_scorer_ids(&mut scorers)?;
    let scorers = serde_json::to_value(&scorers)?;
    let mut tx = user_db.begin(&authed).await?;
    // A path already taken returns no row; a path the caller may not write raises the policy
    // error `map_write_denied` translates. The two are distinct answers and must stay so.
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
    .map_err(|e| map_write_denied(&authed, &payload.path, e))?;
    tx.commit().await?;
    if created.is_none() {
        return Err(Error::BadRequest(format!(
            "Eval dataset {} already exists",
            payload.path
        )));
    }

    // One transaction for all of them, on the unrestricted pool: the dataset the caller was just
    // allowed to create is the permission these rows hang off, and either they all land or the
    // dataset is left empty rather than holding an arbitrary prefix of what was asked for.
    if !payload.cases.is_empty() {
        let mut tx = db.begin().await?;
        for case in payload.cases {
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

/// Replace a dataset's cases with the list sent, in one transaction.
///
/// The editor holds every case at once and saves them together, so this is one write rather than
/// one per case: a save that wrote a prefix would leave the dataset in a state nobody asked for,
/// and the cases it had already written would arrive again as new ones when the save was retried.
pub async fn save_cases(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, String)>,
    Json(payload): Json<SaveCases>,
) -> JsonResult<Vec<Uuid>> {
    require_dataset_writable(&authed, &user_db, &w_id, &path).await?;
    // The list is what the dataset holds afterwards, so its own length is the count to weigh.
    if payload.cases.len() as i64 > MAX_CASES_PER_DATASET {
        return Err(Error::BadRequest(format!(
            "An eval dataset holds at most {} cases. Split them into several datasets.",
            MAX_CASES_PER_DATASET
        )));
    }
    for case in &payload.cases {
        check_case_size(&case.input, case.expected.as_ref())?;
    }

    let mut tx = db.begin().await?;
    let kept: Vec<Uuid> = payload.cases.iter().filter_map(|c| c.id).collect();
    sqlx::query!(
        "DELETE FROM eval_case
         WHERE workspace_id = $1 AND dataset_path = $2 AND NOT (id = ANY($3))",
        w_id,
        path,
        &kept
    )
    .execute(&mut *tx)
    .await?;

    let mut ids = Vec::with_capacity(payload.cases.len());
    for case in &payload.cases {
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
            .fetch_optional(&mut *tx)
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
            })?,
        };
        ids.push(id);
    }
    tx.commit().await?;
    Ok(Json(ids))
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
