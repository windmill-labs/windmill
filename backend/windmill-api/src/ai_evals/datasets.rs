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
    /// The cases to create it holding. A case cannot be written before there is a dataset for it
    /// to be a row of, so they are sent with it rather than added afterwards.
    #[serde(default)]
    pub cases: Vec<NewEvalCase>,
}

#[derive(Deserialize)]
pub struct EditDataset {
    /// Renames the dataset. Its cases and experiments follow through the foreign keys.
    #[serde(default)]
    pub path: Option<String>,
    /// Left out to keep the stored summary; sent as `""` to clear it.
    #[serde(default)]
    pub summary: Option<String>,
    /// Left out to keep the dataset's columns as they are; sent to replace them wholesale.
    #[serde(default)]
    pub scorers: Option<Vec<Scorer>>,
    /// The cases as they should stand afterwards: all of them, each carrying its `id` if the
    /// dataset already has it. Sent with the rest of an edit so a rename the dataset refuses
    /// refuses the case edits with it, rather than leaving them written under the old name.
    #[serde(default)]
    pub cases: Option<Vec<SaveCase>>,
}

#[derive(Deserialize)]
pub struct SaveCase {
    #[serde(default)]
    pub id: Option<Uuid>,
    #[serde(default)]
    pub input: EvalCaseInput,
    #[serde(default)]
    pub expected: Option<Box<RawValue>>,
}

#[derive(Serialize)]
pub struct ListCasesResponse {
    pub cases: Vec<EvalCase>,
}

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
            .map(|row| {
                dataset_from_row(
                    row.path,
                    row.summary,
                    row.scorers,
                    row.created_at,
                    row.created_by,
                    row.edited_at,
                    row.edited_by,
                )
            })
            .collect::<Result<Vec<_>>>()?,
    ))
}

pub async fn create_dataset(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(payload): Json<CreateDataset>,
) -> Result<String> {
    check_proper_path(&payload.path)?;
    check_summary(payload.summary.as_deref())?;
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot create eval datasets".to_string(),
        ));
    }
    check_case_set(
        payload
            .cases
            .iter()
            .map(|case| (&case.input, case.expected.as_ref())),
    )?;
    let mut scorers = payload.scorers;
    // A dataset being created has no columns yet, so every id is minted.
    assign_scorer_ids(&mut scorers, &std::collections::HashSet::new())?;
    let scorers = serde_json::to_value(&scorers)?;
    // One `user_db` transaction: the row's insert policy gates the dataset, the cases' insert
    // policy gates each case, and the two land together or not at all.
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
            // clock_timestamp() (not the now() default, which is transaction-stable) so cases
            // saved together get strictly increasing created_at and reload in insertion order;
            // ORDER BY created_at, id would otherwise tie-break a same-transaction batch on the
            // random uuid id.
            "INSERT INTO eval_case
                    (workspace_id, dataset_path, input, expected, created_by, created_at)
                 VALUES ($1, $2, $3, $4, $5, clock_timestamp())",
            w_id,
            payload.path,
            serde_json::to_value(&case.input)?,
            opt_from_raw(case.expected.as_ref())?,
            authed.username,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    windmill_common::feature_usage::log_feature_usage("ai_agent_eval", "dataset_created", "");

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
/// or not at all.
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
            check_proper_path(&new_path)?;
            // A rename is owner-only, as for every other renamable object. RLS write access is not
            // enough: the UPDATE policies carry no explicit WITH CHECK, so Postgres reuses their
            // USING, and the row's own extra_perms travels with the rename and would satisfy it
            // for any destination.
            windmill_api_auth::require_owner_of_path(&authed, &path)?;
            Some(new_path)
        }
        None => None,
    };
    if let Some(cases) = &payload.cases {
        check_cases(cases)?;
    }
    // One `user_db` transaction, governed by the row-level policies throughout. The row is read
    // `FOR UPDATE` — its UPDATE policy decides who may — which also pins its cases, so a
    // concurrent edit cannot restore a removed scorer's id or interleave with the case write.
    let mut tx = user_db.clone().begin(&authed).await?;
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
        parse_scorers(current)?.into_iter().map(|s| s.id).collect();
    let scorers = match payload.scorers {
        Some(mut scorers) => {
            assign_scorer_ids(&mut scorers, &existing)?;
            Some(serde_json::to_value(&scorers)?)
        }
        None => None,
    };

    let updated = sqlx::query_scalar!(
        "UPDATE eval_dataset
         SET path = COALESCE($6, path), summary = COALESCE($3, summary),
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
/// own retention.
pub async fn delete_dataset(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, String)>,
) -> Result<String> {
    check_proper_path(&path)?;
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

async fn read_cases(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    w_id: &str,
    dataset: &str,
    per_page: usize,
    offset: usize,
) -> Result<Vec<EvalCase>> {
    let rows = sqlx::query!(
        "SELECT id, input, expected, created_at, created_by
         FROM eval_case
         WHERE workspace_id = $1 AND dataset_path = $2
         ORDER BY created_at, id
         LIMIT $3 OFFSET $4",
        w_id,
        dataset,
        per_page as i64,
        offset as i64
    )
    .fetch_all(&mut **tx)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(EvalCase {
                id: row.id,
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
    check_proper_path(&path)?;
    let (per_page, offset) = paginate(pagination);
    let mut tx = user_db.begin(&authed).await?;
    // The dataset first, so an unknown or unreadable one is a 404 rather than an empty dataset:
    // the case rows are invisible in both cases.
    let dataset = sqlx::query_scalar!(
        "SELECT path FROM eval_dataset WHERE workspace_id = $1 AND path = $2",
        w_id,
        path
    )
    .fetch_optional(&mut *tx)
    .await?;
    if dataset.is_none() {
        return Err(Error::NotFound(format!("Eval dataset {} not found", path)));
    }
    let cases = read_cases(&mut tx, &w_id, &path, per_page, offset).await?;
    tx.commit().await?;
    Ok(Json(ListCasesResponse { cases }))
}

/// What a whole list of cases can be refused for, before any of it is written.
fn check_cases(cases: &[SaveCase]) -> Result<()> {
    check_case_set(
        cases
            .iter()
            .map(|case| (&case.input, case.expected.as_ref())),
    )?;
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
                "UPDATE eval_case SET input = $4, expected = $5
                 WHERE workspace_id = $1 AND dataset_path = $2 AND id = $3
                 RETURNING id",
                w_id,
                path,
                id,
                input,
                expected,
            )
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Eval case {} not found in {}", id, path)))?,
            None => sqlx::query_scalar!(
                // clock_timestamp() keeps a same-transaction batch in insertion order on reload.
                "INSERT INTO eval_case
                    (workspace_id, dataset_path, input, expected, created_by, created_at)
                 VALUES ($1, $2, $3, $4, $5, clock_timestamp())
                 RETURNING id",
                w_id,
                path,
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
