use super::*;

/// One run of a dataset: written once when the dataset is run, and only ever read afterwards.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvalExperiment {
    pub id: Uuid,
    pub dataset: String,
    pub subject: EvalSubject,
    /// This subject's nth run of this dataset, allocated once and never reused: "Run 7" survives
    /// history being pruned, which a position computed when the list is read would not.
    pub run_number: i32,
    /// The flow executing the run: one job holding every case and its scores.
    pub run_job_id: Uuid,
    pub case_count: i64,
    /// What the run scored, one entry per scorer that produced a number. Carried on the run so a
    /// list can say what each one scored without reading every cell of every one of them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scores: Vec<ExperimentScore>,
    /// Whether the flow executing this run is still going. What makes a list of runs worth
    /// watching rather than worth reloading.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub running: bool,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

/// One scorer's headline for one run: the two numbers a column reports, over that run's cells.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExperimentScore {
    pub scorer_id: String,
    /// What the column is called in the dataset that ran it, resolved here because a list of runs
    /// spanning datasets cannot hold every dataset's scorers to look it up.
    pub name: String,
    /// `agent` or `script`, for the badge to say which kind of thing produced the number.
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mean: Option<f64>,
    /// The share of scored cells at or above the column's threshold, for a column that has one.
    /// Absent where the column has no threshold and the mean is the whole headline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pass_rate: Option<f64>,
    pub scored: i64,
    /// How many of this run's cells the column failed on. A column that failed on all of them
    /// still ran, which is the difference between a headline of nothing and no headline at all.
    pub failed: i64,
}

#[derive(Deserialize)]
pub struct ListExperimentsQuery {
    /// Restrict to one agent's runs. Both what was deployed and what was drafted are that agent's
    /// history, so this does not discriminate by kind.
    #[serde(default)]
    pub subject_path: Option<String>,
}

/// Every run of this agent, across every dataset it has been measured on.
///
/// Filtered by `user_db`: a run is visible exactly when the dataset it belongs to is.
pub async fn list_all_experiments(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<ListExperimentsQuery>,
) -> JsonResult<Vec<EvalExperiment>> {
    let mut tx = user_db.clone().begin(&authed).await?;
    let rows = sqlx::query!(
        "SELECT e.id, e.dataset_path, e.subject, e.run_number, e.run_job_id, e.created_at,
                e.created_by,
                (SELECT count(*) FROM eval_experiment_case c WHERE c.experiment_id = e.id)
                    AS \"case_count!\"
         FROM eval_experiment e
         JOIN eval_dataset d ON d.workspace_id = e.workspace_id AND d.path = e.dataset_path
         WHERE e.workspace_id = $1
               AND ($3::text IS NULL OR e.subject ->> 'path' = $3)
         ORDER BY e.created_at DESC
         LIMIT $2",
        w_id,
        MAX_EXPERIMENTS_LISTED,
        query.subject_path,
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let mut experiments = rows
        .into_iter()
        .map(|row| {
            experiment_from_row(
                row.id,
                row.dataset_path,
                row.subject,
                row.run_number,
                row.run_job_id,
                row.case_count,
                row.created_at,
                row.created_by,
            )
        })
        .collect::<Result<Vec<_>>>()?;

    resolve_listed_drafts(&authed, &db, &user_db, &w_id, &mut experiments).await?;
    let scorers_by_dataset = scorers_of_listed(&authed, &user_db, &w_id, &experiments).await?;
    mark_running(&db, &w_id, &mut experiments).await?;
    sync_listed_runs(&db, &w_id, &experiments).await?;
    let mut scores = experiment_scores(&db, &experiments, &scorers_by_dataset).await?;
    for experiment in experiments.iter_mut() {
        experiment.scores = scores.remove(&experiment.id).unwrap_or_default();
    }
    Ok(Json(experiments))
}

/// Which listed runs are still going, read from the flows executing them. A run whose flow is no
/// longer there at all is over: jobs have their own retention, and reading a missing one as
/// unfinished would leave every run older than it spinning.
async fn mark_running(db: &DB, w_id: &str, experiments: &mut [EvalExperiment]) -> Result<()> {
    let job_ids: Vec<Uuid> = experiments.iter().map(|e| e.run_job_id).collect();
    if job_ids.is_empty() {
        return Ok(());
    }
    let unfinished: std::collections::HashSet<Uuid> = sqlx::query_scalar!(
        "SELECT j.id AS \"id!\" FROM v2_job j
         LEFT JOIN v2_job_completed c ON c.id = j.id AND c.workspace_id = $2
         WHERE j.id = ANY($1) AND j.workspace_id = $2 AND c.id IS NULL",
        &job_ids,
        w_id
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .collect();
    for experiment in experiments.iter_mut() {
        experiment.running = unfinished.contains(&experiment.run_job_id);
    }
    Ok(())
}

/// A run of a draft whose edits have since been deployed is a run of that version. Resolved once
/// per subject rather than once per run, because a listing is usually one agent's history.
async fn resolve_listed_drafts(
    authed: &ApiAuthed,
    db: &DB,
    user_db: &UserDB,
    w_id: &str,
    experiments: &mut [EvalExperiment],
) -> Result<()> {
    let drafted: std::collections::HashSet<String> = experiments
        .iter()
        .filter(|e| e.subject.kind == EvalSubjectKind::AgentDraft)
        .map(|e| e.subject.path.clone())
        .collect();
    if drafted.is_empty() {
        return Ok(());
    }
    // Read each subject as the caller (see experiment_results): an agent the caller cannot read
    // yields no hash or version, so its config fingerprint never leaks through the list either.
    let mut deployed = std::collections::HashMap::new();
    for path in drafted {
        let (hash, version) = match readable_agent_state(authed, user_db, w_id, &path).await? {
            Some((config, version)) => (Some(draft_hash(&config)), Some(version)),
            None => (None, None),
        };
        deployed.insert(path.clone(), (hash, version));
    }
    for experiment in experiments.iter_mut() {
        let Some((hash, version)) = deployed.get(&experiment.subject.path) else {
            continue;
        };
        // Each run's own dataset: the list may span them, and the update is keyed on both.
        let dataset = experiment.dataset.clone();
        resolve_deployed_draft(db, w_id, &dataset, experiment, hash.as_deref(), *version).await?;
    }
    Ok(())
}

/// How many listed runs one list call reads out of their flows. A run's scores live in its flow
/// until something reads them into `eval_score`, so an unopened run has nothing to report; the cap
/// keeps a long history from turning one list call into a hundred flow reads.
const MAX_RUNS_SYNCED_PER_LIST: usize = 10;

/// Read the flows of listed runs that still have scores to collect. Runs already collected are
/// skipped, so the steady-state cost of listing is one query rather than one read per run.
async fn sync_listed_runs(db: &DB, w_id: &str, experiments: &[EvalExperiment]) -> Result<()> {
    if experiments.is_empty() {
        return Ok(());
    }
    let ids: Vec<Uuid> = experiments.iter().map(|e| e.id).collect();
    let unread = sqlx::query_scalar!(
        "SELECT DISTINCT experiment_id FROM eval_score
         WHERE experiment_id = ANY($1) AND score IS NULL AND error IS NULL
               AND NOT not_applicable",
        &ids
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .collect::<std::collections::HashSet<_>>();
    for experiment in experiments
        .iter()
        .filter(|e| unread.contains(&e.id))
        .take(MAX_RUNS_SYNCED_PER_LIST)
    {
        // Best-effort, for the same reason reading one run is: this is the home screen, and one
        // run with an unreadable cell must not cost the list of every other run.
        if let Err(e) = sync_run(db, w_id, experiment.id, experiment.run_job_id, false).await {
            tracing::warn!("could not collect eval run {}: {e:#}", experiment.id);
        }
    }
    Ok(())
}

/// Every listed run's per-scorer headline, in one grouped query.
///
/// Thresholds come from each run's own dataset as its scorers are *now*, joined per (run, scorer)
/// rather than per scorer: a list spanning datasets is a list of runs whose columns are not the
/// same columns.
async fn experiment_scores(
    db: &DB,
    experiments: &[EvalExperiment],
    scorers_by_dataset: &std::collections::HashMap<String, Vec<Scorer>>,
) -> Result<std::collections::HashMap<Uuid, Vec<ExperimentScore>>> {
    let mut by_experiment: std::collections::HashMap<Uuid, Vec<ExperimentScore>> =
        Default::default();
    // One entry per (run, column) it could have scored, which is what carries the threshold and
    // the column's order into the query.
    let mut ids: Vec<Uuid> = vec![];
    let mut scorer_ids: Vec<String> = vec![];
    let mut thresholds: Vec<Option<f64>> = vec![];
    for experiment in experiments {
        for scorer in scorers_by_dataset
            .get(&experiment.dataset)
            .map(|s| s.as_slice())
            .unwrap_or(&[])
        {
            ids.push(experiment.id);
            scorer_ids.push(scorer.id.clone());
            thresholds.push(scorer.pass_if);
        }
    }
    if ids.is_empty() {
        return Ok(by_experiment);
    }
    let rows = sqlx::query!(
        "SELECT s.experiment_id AS \"experiment_id!\", s.scorer_id AS \"scorer_id!\",
                avg(s.score) AS mean,
                count(s.score) AS \"scored!\",
                count(*) FILTER (WHERE s.error IS NOT NULL) AS \"failed!\",
                count(*) FILTER (WHERE t.pass_if IS NOT NULL AND s.score >= t.pass_if)
                    AS \"passed!\",
                bool_or(t.pass_if IS NOT NULL) AS \"has_threshold!\"
         FROM eval_score s
         JOIN unnest($1::uuid[], $2::text[], $3::float8[])
                AS t(experiment_id, scorer_id, pass_if)
              ON t.experiment_id = s.experiment_id AND t.scorer_id = s.scorer_id
         GROUP BY s.experiment_id, s.scorer_id",
        &ids,
        &scorer_ids,
        &thresholds as &[Option<f64>],
    )
    .fetch_all(db)
    .await?;
    let mut headline: std::collections::HashMap<
        (Uuid, String),
        (Option<f64>, i64, i64, i64, bool),
    > = Default::default();
    for row in rows {
        headline.insert(
            (row.experiment_id, row.scorer_id),
            (
                row.mean,
                row.scored,
                row.failed,
                row.passed,
                row.has_threshold,
            ),
        );
    }
    // Emitted in the dataset's column order rather than the query's, so the badges on a row read
    // left to right the way that dataset's table does.
    for experiment in experiments {
        for scorer in scorers_by_dataset
            .get(&experiment.dataset)
            .map(|s| s.as_slice())
            .unwrap_or(&[])
        {
            // A column with no cells at all on this run is one added after it. A column that has
            // cells is reported even where none produced a number, which is what a column that
            // failed throughout looks like.
            let Some((mean, scored, failed, passed, has_threshold)) =
                headline.get(&(experiment.id, scorer.id.clone()))
            else {
                continue;
            };
            by_experiment
                .entry(experiment.id)
                .or_default()
                .push(ExperimentScore {
                    scorer_id: scorer.id.clone(),
                    name: scorer_name(scorer),
                    kind: scorer.def.kind_str().to_string(),
                    mean: *mean,
                    pass_rate: (*has_threshold && *scored > 0)
                        .then(|| *passed as f64 / *scored as f64),
                    scored: *scored,
                    failed: *failed,
                });
        }
    }
    Ok(by_experiment)
}

/// The scorers of every dataset named by a listed run, read through `user_db` so a run of a
/// dataset the caller cannot read contributes nothing.
async fn scorers_of_listed(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    experiments: &[EvalExperiment],
) -> Result<std::collections::HashMap<String, Vec<Scorer>>> {
    let paths: Vec<String> = experiments
        .iter()
        .map(|e| e.dataset.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    if paths.is_empty() {
        return Ok(Default::default());
    }
    let mut tx = user_db.clone().begin(authed).await?;
    let rows = sqlx::query!(
        "SELECT path, scorers FROM eval_dataset WHERE workspace_id = $1 AND path = ANY($2)",
        w_id,
        &paths
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    rows.into_iter()
        .map(|row| Ok((row.path, parse_scorers(row.scorers)?)))
        .collect()
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
    /// The scorer read this case and had nothing to measure on it. Left out of the column's mean
    /// and pass rate rather than counted as a zero.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub not_applicable: bool,
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
    pub input: EvalCaseInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<Box<RawValue>>,
    /// The iteration that ran this case. Absent between a run being recorded and its flow
    /// reaching this case, which reads as a case still to run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<Uuid>,
    /// What happened to the answer: the iteration's own `success`/`failure`/`canceled`/`skipped`
    /// once it has finished, and until then the agent step's, since the answer is written before
    /// the scorers that keep the iteration running have read it. `unavailable` for a case whose
    /// job was retained away before anything read what it produced.
    pub status: String,
    /// The agent's answer, which is what a table cell shows. The whole trajectory stays
    /// reachable through `job_id`, so the row carries the text rather than the result object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// The agent version this cell ran against. Cells of one experiment can differ, which is what
    /// the table says instead of averaging two versions silently.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_version: Option<i64>,
    /// For a run of unsaved edits, the hash of the configuration this cell ran: edits move without
    /// a version changing, and `resolve_deployed_draft` matches this against what is deployed.
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
    /// mean rather than instead of it: neither number answers the other's question.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pass_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_pass_rate: Option<f64>,
    pub scored: usize,
    /// Cells the baseline has no score for, reported so a column the baseline never ran shows as
    /// unscored rather than as a spurious difference.
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
    /// agent that no longer exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_current_version: Option<i64>,
    /// What the agent hashes to as deployed. A run of unsaved edits carrying this hash ran exactly
    /// what is deployed now — the edits were saved — so it is a run of that version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_deployed_hash: Option<String>,
}

/// The agent's own result is `{output, messages}`; the answer is its `output`.
pub(crate) fn agent_answer(result: &RawValue) -> Option<String> {
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
    not_applicable: bool,
    definition: String,
}

/// Every score of one experiment, keyed by the cell and the scorer that produced it.
async fn load_scores(
    db: &DB,
    experiment_id: Uuid,
) -> Result<std::collections::HashMap<(i32, String), ScoreRow>> {
    Ok(sqlx::query!(
        "SELECT ordinal, scorer_id, score, reason, checks, error, not_applicable, definition
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
                not_applicable: r.not_applicable,
                definition: r.definition,
            },
        )
    })
    .collect())
}

async fn read_experiment(db: &DB, w_id: &str, dataset: &str, id: Uuid) -> Result<EvalExperiment> {
    let row = sqlx::query!(
        "SELECT e.subject, e.run_number, e.run_job_id, e.created_at,
                e.created_by,
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
        row.run_job_id,
        row.case_count,
        row.created_at,
        row.created_by,
    )
}

/// Recognise a draft run that has since been deployed, and record it as the version it became.
///
/// Written once rather than derived per read: derived against what is deployed *now*, the next
/// deployment would send a run that already read `v21` back to `v18 + edits`.
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
    // Both writes in one transaction: a failure between them would leave the experiment promoted
    // to a version while its cells stayed a draft's, a split no later read repairs since the
    // experiment is no longer a draft.
    let mut tx = db.begin().await?;
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
    .execute(&mut *tx)
    .await?;
    // The cells that ran that configuration are dated by the version too; leaving their hash would
    // make the run go on reading as a draft's after the next deployment.
    sqlx::query!(
        "UPDATE eval_experiment_case
         SET subject_version = $3, subject_draft_hash = NULL
         WHERE experiment_id = $1 AND subject_draft_hash = $2",
        experiment.id,
        hash,
        version,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Record what a run produced, from inside the run: the last step of a run's own flow calls this.
///
/// Gated on reading the run rather than on writing its dataset, unlike everything else here: it is
/// the same harvest `experiment_results` performs behind the same check, over the run's own cells,
/// and it reports a count rather than any of what it read.
pub async fn collect_experiment(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<ExperimentId>,
) -> JsonResult<usize> {
    // Through `user_db`, so the run is one the caller can see. The row carries the job to read it
    // out of, so nothing that is read afterwards is caller-supplied.
    let mut tx = user_db.begin(&authed).await?;
    let experiment = sqlx::query!(
        "SELECT id, run_job_id FROM eval_experiment WHERE workspace_id = $1 AND id = $2",
        w_id,
        query.id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let experiment =
        experiment.ok_or_else(|| Error::NotFound(format!("Eval run {} not found", query.id)))?;
    sync_run(&db, &w_id, experiment.id, experiment.run_job_id, true).await?;
    let recorded = sqlx::query_scalar!(
        "SELECT count(*) AS \"count!\" FROM eval_experiment_case
         WHERE experiment_id = $1 AND status IS NOT NULL",
        experiment.id
    )
    .fetch_one(&db)
    .await?;
    Ok(Json(recorded as usize))
}

#[derive(Deserialize)]
pub struct ExperimentId {
    pub id: Uuid,
}

/// Collect a run for a reader, without letting the collection decide whether the read succeeds.
/// `collect_experiment` propagates instead: it is the run reporting on itself, and a failure there
/// is worth surfacing to the step that called it.
async fn collect_quietly(db: &DB, w_id: &str, experiment_id: Uuid, run_job_id: Uuid) {
    if let Err(e) = sync_run(db, w_id, experiment_id, run_job_id, true).await {
        tracing::warn!("could not collect eval run {}: {e:#}", experiment_id);
    }
}

/// The rows a results table is built from. The job ids come out of `eval_experiment_case`, which
/// only this module writes, so they can be read on the unrestricted pool once the dataset read
/// below has established the caller's access.
pub async fn experiment_results(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, dataset)): Path<(String, String)>,
    Query(query): Query<ExperimentRef>,
) -> JsonResult<ExperimentResults> {
    // The rows carry what the run's jobs produced, which `jobs:read` gates. `UserDB` settles who
    // may see the dataset; a token's scopes are a separate question.
    check_scopes(&authed, || "jobs:read".to_string())?;
    let dataset_row = read_dataset(&authed, &user_db, &w_id, &dataset).await?;
    let scorers = dataset_row.scorers;

    let mut experiment = read_experiment(&db, &w_id, &dataset, query.id).await?;
    // Best-effort: collecting is what the run's own step is for, and a cell that could not be read
    // — a job retained away between the iteration and its children — must not take the whole table
    // down with it. The rows already recorded are still the run.
    collect_quietly(&db, &w_id, query.id, experiment.run_job_id).await;
    let scores = load_scores(&db, query.id).await?;

    let baseline = match query.baseline.filter(|id| *id != query.id) {
        Some(id) => {
            let baseline = read_experiment(&db, &w_id, &dataset, id).await?;
            collect_quietly(&db, &w_id, id, baseline.run_job_id).await;
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
        "SELECT ordinal, case_id, input, expected, job_id, subject_version,
                subject_draft_hash, output, answered, status
         FROM eval_experiment_case
         WHERE experiment_id = $1 ORDER BY ordinal",
        query.id
    )
    .fetch_all(&db)
    .await?;

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
                not_applicable: current.map(|c| c.not_applicable).unwrap_or(false),
                // A row exists because the run was launched with this scorer, so an empty one is a
                // score still to come, unless the scorer has already said this case is not one it
                // measures.
                pending: current
                    .map(|c| c.score.is_none() && c.error.is_none() && !c.not_applicable)
                    .unwrap_or(false),
                passed: scorer.passed(current.and_then(|c| c.score)),
                baseline: baseline_score.and_then(|b| b.score),
                definition_changed: changed,
            });
        }
        rows.push(ExperimentRow {
            case_id: case.case_id,
            input: serde_json::from_value(case.input)?,
            expected: opt_to_raw(case.expected)?,
            // The iteration's verdict once it has one. While it is still running, the agent step's:
            // the answer is written before the scorers read it, and a spinner beside an answer
            // already there reads as an answer still being written.
            status: case
                .status
                .or_else(|| {
                    case.answered
                        .map(|ok| if ok { "success" } else { "failure" }.to_string())
                })
                .unwrap_or_else(|| "running".to_string()),
            output: case.output,
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

    // Read as the caller, so a viewer who can see the dataset but not the agent gets neither: the
    // agent's version and configuration fingerprint must not leak past its own read permission.
    let (subject_deployed_hash, subject_current_version) =
        match readable_agent_state(&authed, &user_db, &w_id, &experiment.subject.path).await? {
            Some((config, version)) => (Some(draft_hash(&config)), Some(version)),
            None => (None, None),
        };

    // A run of unsaved edits whose configuration has since been deployed is a run of that version.
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
        subject_deployed_hash,
    }))
}
