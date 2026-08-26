use super::*;

/// What a scorer resolves to, alongside the definition to record: a script by its pinned hash, or
/// a judge by the configuration to inline.
pub(crate) enum ResolvedScorer {
    Script { hash: i64 },
    Judge { config: AgentDraft },
}

/// The runnable a scorer names, resolved through the caller's *own* database so a run can only
/// execute code the caller may read: a scorer is added with a bare path and nothing checks read
/// access there.
///
/// Returns the definition to record and what to run: a script by its deployed hash to pin, or a
/// judge by the configuration to inline, so a redeploy midway through a run cannot swap the code
/// out from under a score labelled with the old version.
pub(crate) async fn resolve_scorer(
    user_db: &UserDB,
    authed: &ApiAuthed,
    w_id: &str,
    scorer: &Scorer,
) -> Result<(String, ResolvedScorer)> {
    match &scorer.def {
        ScorerDef::Script { path } => {
            // The latest *deployed* hash (no draft, no failed deploy), through the canonical helper
            // so the version a scorer pins is the one everything else runs.
            let mut tx = user_db.clone().begin(authed).await?;
            let hash = windmill_common::get_latest_script_hash(&mut *tx, path, w_id).await?;
            tx.commit().await?;
            let Some(hash) = hash else {
                return Err(Error::BadRequest(format!(
                    "Scorer script {} is not deployed or not readable",
                    path
                )));
            };
            Ok((
                scorer.definition(Some(&hash.to_string())),
                ResolvedScorer::Script { hash },
            ))
        }
        ScorerDef::Agent { path } => {
            let Some((config, version)) = readable_agent_state(authed, user_db, w_id, path).await?
            else {
                return Err(Error::BadRequest(format!(
                    "Judge scorer {} is not a readable ai_agent resource",
                    path
                )));
            };
            Ok((
                scorer.definition(Some(&version.to_string())),
                ResolvedScorer::Judge { config },
            ))
        }
    }
}

/// Bring a run's record up to date with the flow that executed it: which iteration answered which
/// case, what the agent answered, and what its scorers returned.
///
/// `answers` is what separates the two callers: a listing reports each run's score aggregates and
/// never shows an answer, so harvesting them there reads a column of every case of every listed
/// run to display none of it.
pub(crate) async fn sync_run(
    db: &DB,
    w_id: &str,
    experiment_id: Uuid,
    run_job_id: Uuid,
    answers: bool,
) -> Result<()> {
    backfill_case_jobs(db, w_id, experiment_id, run_job_id).await?;
    settle_unspawned_cases(db, w_id, experiment_id, run_job_id).await?;
    if answers {
        record_case_answers(db, w_id, experiment_id).await?;
    }
    harvest_flow_scores(db, w_id, experiment_id).await?;
    Ok(())
}

/// Give a terminal status to cases the run never spawned an iteration for: with no `job_id` there
/// is nothing to read an answer or a score out of, so they would report "running" indefinitely.
async fn settle_unspawned_cases(
    db: &DB,
    w_id: &str,
    experiment_id: Uuid,
    run_job_id: Uuid,
) -> Result<()> {
    // Only a run that has reached `v2_job_completed` is settled from here. A job absent from the
    // tables is as likely mid-launch — the experiment is committed before its job is pushed — as
    // aged out, and settling then would cancel the cases of a run about to start. A cancelled run
    // lands in `v2_job_completed`, so a cancel before an iteration spawned is still covered.
    let Some(terminal_status) = sqlx::query_scalar!(
        "SELECT status::text AS \"status!\" FROM v2_job_completed WHERE id = $1 AND workspace_id = $2",
        run_job_id,
        w_id
    )
    .fetch_optional(db)
    .await?
    else {
        return Ok(());
    };
    let settled = sqlx::query_scalar!(
        "UPDATE eval_experiment_case SET status = $2, answered = false
         WHERE experiment_id = $1 AND job_id IS NULL AND status IS NULL
         RETURNING ordinal",
        experiment_id,
        terminal_status
    )
    .fetch_all(db)
    .await?;
    // The score cells of a case that never ran have no job to read a verdict out of either.
    if !settled.is_empty() {
        sqlx::query!(
            "UPDATE eval_score SET error = 'The case did not run'
             WHERE experiment_id = $1 AND ordinal = ANY($2)
                   AND score IS NULL AND error IS NULL AND NOT not_applicable",
            experiment_id,
            &settled
        )
        .execute(db)
        .await?;
    }
    Ok(())
}

/// In-flight reads of what a case's agent step produced. Each is several queries and a run holds
/// up to `MAX_CASES_PER_DATASET` cases, so they go a few at a time.
const HARVEST_CONCURRENCY: usize = 8;

/// Cases whose scorer results are read in one query: every scorer of every case in the batch, so
/// the batch bounds how much of a run's worth of judge conversations is held at once.
const HARVEST_BATCH_CASES: usize = 100;

/// Copy what each iteration produced into its row: the agent's answer, whether producing it
/// succeeded, and how the iteration ended.
///
/// Written once, when it becomes readable, rather than read back out of the jobs whenever the
/// table is displayed — jobs have their own retention, and a run whose rows are kept has to still
/// read as the run it was after they have aged out.
async fn record_case_answers(db: &DB, w_id: &str, experiment_id: Uuid) -> Result<()> {
    let unrecorded = sqlx::query!(
        "SELECT c.ordinal, c.job_id AS \"job_id!\", d.status::text AS status,
                (j.id IS NOT NULL) AS \"job_exists!\"
         FROM eval_experiment_case c
         LEFT JOIN v2_job j ON j.id = c.job_id AND j.workspace_id = $2
         LEFT JOIN v2_job_completed d ON d.id = c.job_id AND d.workspace_id = $2
         WHERE c.experiment_id = $1 AND c.job_id IS NOT NULL AND c.status IS NULL",
        experiment_id,
        w_id
    )
    .fetch_all(db)
    .await?;

    use futures::StreamExt;
    let answers = futures::stream::iter(unrecorded.into_iter().map(|row| async move {
        // The job was retained away before anything read it: nothing to read, and nothing more
        // will ever be there to read.
        if !row.job_exists {
            return Ok((row.ordinal, None, None, Some("unavailable".to_string())));
        }
        // The agent step's own result, never the iteration's: the iteration goes on to score the
        // answer, so the answer is settled long before the iteration is.
        let agent = agent_result(db, w_id, row.job_id).await?;
        // An iteration that ended without an answer — skipped, cancelled, or an agent that failed
        // outright — produced none, and saying so is what stops this re-reading it.
        let answered = agent
            .as_ref()
            .map(|(_, success)| *success)
            .or_else(|| row.status.is_some().then_some(false));
        let output = agent.as_ref().and_then(|(result, _)| agent_answer(result));
        Ok::<_, Error>((row.ordinal, output, answered, row.status))
    }))
    .buffered(HARVEST_CONCURRENCY)
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .collect::<Result<Vec<_>>>()?;

    // One statement for the whole run: the run's own collect step reaches every case at once, and
    // a thousand of them one at a time is a thousand round trips.
    let mut ordinals = vec![];
    let mut outputs = vec![];
    let mut answered = vec![];
    let mut statuses = vec![];
    for (ordinal, output, was_answered, status) in answers {
        // Nothing to record yet, and the iteration may still produce it.
        if was_answered.is_none() && status.is_none() {
            continue;
        }
        ordinals.push(ordinal);
        outputs.push(output);
        answered.push(was_answered);
        statuses.push(status);
    }
    if ordinals.is_empty() {
        return Ok(());
    }
    sqlx::query!(
        "UPDATE eval_experiment_case c
         SET output = COALESCE(c.output, t.output), answered = COALESCE(c.answered, t.answered),
             status = COALESCE(c.status, t.status)
         FROM UNNEST($2::int[], $3::text[], $4::bool[], $5::text[])
              AS t(ordinal, output, answered, status)
         WHERE c.experiment_id = $1 AND c.ordinal = t.ordinal",
        experiment_id,
        &ordinals,
        &outputs as &[Option<String>],
        &answered as &[Option<bool>],
        &statuses as &[Option<String>],
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The agent step's result, with "there is none" kept apart from "it could not be read": a lookup
/// that failed for any other reason must not be recorded as a case that produced no answer,
/// because nothing reads that row again.
pub(crate) async fn agent_result(
    db: &DB,
    w_id: &str,
    job_id: Uuid,
) -> Result<Option<(Box<RawValue>, bool)>> {
    match windmill_queue::get_result_and_success_by_id_from_flow(
        db,
        w_id,
        &job_id,
        AGENT_NODE_ID,
        None,
    )
    .await
    {
        Ok(found) => Ok(Some(found)),
        Err(Error::NotFound(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Match each case to the iteration that ran it. The flow engine mints those job ids, so the case
/// they belong to is read back from the iteration's own arguments, which survives iterations
/// finishing in any order.
async fn backfill_case_jobs(
    db: &DB,
    w_id: &str,
    experiment_id: Uuid,
    run_job_id: Uuid,
) -> Result<()> {
    sqlx::query!(
        "UPDATE eval_experiment_case c SET job_id = j.id
         FROM v2_job j
         WHERE j.parent_job = $3 AND j.workspace_id = $2
               AND (j.args -> 'iter' -> 'value' ->> 'case_id')::uuid = c.case_id
               AND c.experiment_id = $1 AND c.job_id IS NULL",
        experiment_id,
        w_id,
        run_job_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Read the scores a run's own flow produced into `eval_score`, so a score outlives the flow
/// that produced it and the retention on its jobs.
async fn harvest_flow_scores(db: &DB, w_id: &str, experiment_id: Uuid) -> Result<()> {
    let pending = sqlx::query!(
        // Left-joined, so an iteration still running is read too: a scorer runs after the agent
        // within that iteration, so its verdict is there to be read as soon as its own step is
        // done, and waiting for the iteration to end would hold every column of a case back until
        // the last of them finished.
        "SELECT s.ordinal, s.scorer_id, c.job_id AS \"job_id!\", d.status::text AS status,
                c.answered, (j.id IS NOT NULL) AS \"job_exists!\"
         FROM eval_score s
         JOIN eval_experiment_case c
              ON c.experiment_id = s.experiment_id AND c.ordinal = s.ordinal
         LEFT JOIN v2_job j ON j.id = c.job_id AND j.workspace_id = $2
         LEFT JOIN v2_job_completed d ON d.id = c.job_id AND d.workspace_id = $2
         WHERE s.experiment_id = $1 AND s.score IS NULL AND s.error IS NULL
               AND NOT s.not_applicable AND c.job_id IS NOT NULL",
        experiment_id,
        w_id
    )
    .fetch_all(db)
    .await?;
    if pending.is_empty() {
        return Ok(());
    }

    // The job tree is walked in SQL rather than once per cell: a live run is read every couple of
    // seconds and a full one is up to MAX_CASES_PER_DATASET × MAX_SCORERS_PER_DATASET cells. The
    // shape is `build_run_flow`'s: a scorer is the one module of its own branch of the scoring
    // step, so its job's parent is that branch and the branch's parent is the case.
    let mut case_jobs: Vec<Uuid> = pending.iter().map(|row| row.job_id).collect();
    case_jobs.sort();
    case_jobs.dedup();
    let mut modules: Vec<String> = pending
        .iter()
        .map(|row| scorer_module_id(&row.scorer_id))
        .collect();
    modules.sort();
    modules.dedup();
    let mut verdicts: Vec<(i32, String, Option<(Verdict, Option<String>)>)> =
        Vec::with_capacity(pending.len());
    for batch in case_jobs.chunks(HARVEST_BATCH_CASES) {
        let results: std::collections::HashMap<(Uuid, String), Box<RawValue>> = sqlx::query!(
            "SELECT branch.parent_job AS \"case_job!\", scorer.flow_step_id AS \"module!\",
                    done.result AS \"result: sqlx::types::Json<Box<RawValue>>\"
             FROM v2_job branch
             JOIN v2_job scorer ON scorer.parent_job = branch.id
             JOIN v2_job_completed done ON done.id = scorer.id
             WHERE branch.parent_job = ANY($1) AND branch.workspace_id = $2
                   AND scorer.flow_step_id = ANY($3)",
            batch,
            w_id,
            &modules
        )
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|row| {
            let result = row
                .result
                .map(|json| json.0)
                .unwrap_or_else(|| RawValue::from_string("null".to_string()).expect("a literal"));
            ((row.case_job, row.module), result)
        })
        .collect();
        let in_batch: std::collections::HashSet<Uuid> = batch.iter().copied().collect();
        for row in pending.iter().filter(|row| in_batch.contains(&row.job_id)) {
            // Nothing left to read the verdict out of. Settled here, since a cell left pending is
            // one every later listing would go back to this same absent job for.
            if !row.job_exists {
                verdicts.push((
                    row.ordinal,
                    row.scorer_id.clone(),
                    Some((
                        Verdict::default(),
                        Some("The run that produced this score is no longer available".to_string()),
                    )),
                ));
                continue;
            }
            // What to say when the job is over and this scorer left nothing. Only
            // `record_case_answers` tells the two states apart and a listing syncs without it, so
            // `None` withholds the sentence — not the harvest: a scorer that returned a number is
            // read and recorded either way.
            let missing = row.answered.map(|answered| {
                if answered {
                    "This scorer did not run for the case"
                } else {
                    "The case produced no answer to score"
                }
            });
            let result = results
                .get(&(row.job_id, scorer_module_id(&row.scorer_id)))
                .map(|r| r.as_ref());
            let verdict = settle_verdict(result, row.status.as_deref(), missing);
            verdicts.push((row.ordinal, row.scorer_id.clone(), verdict));
        }
    }

    // One statement for every cell read, for the same reason the answers are written that way.
    let mut ordinals = vec![];
    let mut scorer_ids = vec![];
    let mut scores = vec![];
    let mut reasons = vec![];
    let mut checks = vec![];
    let mut errors = vec![];
    let mut not_applicable = vec![];
    for (ordinal, scorer_id, read) in verdicts {
        // Still to come: a scorer whose own step has not run yet.
        let Some((verdict, error)) = read else {
            continue;
        };
        ordinals.push(ordinal);
        scorer_ids.push(scorer_id);
        scores.push(verdict.score);
        reasons.push(verdict.reason);
        checks.push(verdict.checks);
        errors.push(error);
        not_applicable.push(verdict.not_applicable);
    }
    if ordinals.is_empty() {
        return Ok(());
    }
    sqlx::query!(
        "UPDATE eval_score s
         SET score = t.score, reason = t.reason, checks = t.checks, error = t.error,
             not_applicable = t.not_applicable
         FROM UNNEST($2::int[], $3::text[], $4::double precision[], $5::text[], $6::jsonb[],
                     $7::text[], $8::bool[])
              AS t(ordinal, scorer_id, score, reason, checks, error, not_applicable)
         WHERE s.experiment_id = $1 AND s.ordinal = t.ordinal AND s.scorer_id = t.scorer_id",
        experiment_id,
        &ordinals,
        &scorer_ids,
        &scores as &[Option<f64>],
        &reasons as &[Option<String>],
        &checks as &[Option<serde_json::Value>],
        &errors as &[Option<String>],
        &not_applicable,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// One scorer's verdict, from the result of the step that produced it, inside a job that may
/// still be running: a scorer's own step can be done while the iteration around it is not. `None`
/// while the result is not readable yet, which is a state to wait through rather than to record
/// as a failure; `Some` with an error is a scorer that produced nothing, worded by where it ran.
fn settle_verdict(
    result: Option<&RawValue>,
    job_status: Option<&str>,
    // What to record when the job is over and this scorer produced nothing. A different statement
    // depending on where the scorer ran: its own job failed, or the case it was to score never
    // produced an answer. `None` when the caller cannot yet tell those apart, which leaves the
    // cell pending for a read that can, rather than settling it on the wrong one of the two.
    missing_error: Option<&str>,
) -> Option<(Verdict, Option<String>)> {
    Some(match result {
        Some(value) => {
            let verdict = extract_verdict(value);
            match verdict {
                // A score is a fraction: the mean and the pass rate read it as one, so a number
                // outside that range is recorded as an error rather than a value that would
                // quietly skew the column.
                Verdict { score: Some(score), .. } if !(0.0..=1.0).contains(&score) => (
                    Verdict::default(),
                    Some(format!(
                        "The scorer returned {}, outside the 0 to 1 range a score must be in",
                        score
                    )),
                ),
                // A number in range, or the scorer saying this case is not one it measures. Both
                // are answers, so neither is an error.
                Verdict { score: Some(_), .. } | Verdict { not_applicable: true, .. } => {
                    (verdict, None)
                }
                // The job around this scorer is still going, so a module with no number in it is
                // one that has not run yet. Recording a failure here would make it permanent.
                _ if job_status.is_none() => return None,
                _ if job_status == Some("success") => (
                    verdict,
                    Some("The scorer returned no number to plot".to_string()),
                ),
                _ => match missing_error {
                    Some(missing) => (verdict, Some(missing.to_string())),
                    None => return None,
                },
            }
        }
        // The iteration is over, so a scorer step with no readable result produced nothing and
        // never will; left pending it would be re-read on every listing.
        None if job_status == Some("success") => (
            Verdict::default(),
            Some("The scorer step produced no result".to_string()),
        ),
        // The job holding this scorer has not finished, so a module with nothing in it yet is a
        // step that has not run rather than one that produced nothing.
        None if job_status.is_none() => return None,
        None => match missing_error {
            Some(missing) => (Verdict::default(), Some(missing.to_string())),
            None => return None,
        },
    })
}

/// The score and reason read straight out of text that failed to parse as JSON. Deliberately not a
/// second JSON parser: it looks for the two keys and takes what follows, which is what survives a
/// model writing an unescaped quote in the middle of a sentence.
fn salvage_verdict(text: &str) -> (Option<f64>, Option<String>) {
    fn after_key<'a>(text: &'a str, key: &str) -> Option<&'a str> {
        let start = text.find(key)? + key.len();
        Some(text[start..].trim_start().strip_prefix(':')?.trim_start())
    }

    let score = after_key(text, "\"score\"").and_then(|rest| {
        if rest.starts_with("true") {
            return Some(1.0);
        }
        if rest.starts_with("false") {
            return Some(0.0);
        }
        let end = rest
            .find(|c: char| !matches!(c, '0'..='9' | '.' | '-' | '+' | 'e' | 'E'))
            .unwrap_or(rest.len());
        rest[..end].parse::<f64>().ok()
    });

    // To the last quote of the object, so an unescaped one inside the sentence stays part of it.
    let reason = after_key(text, "\"reason\"")
        .and_then(|rest| rest.strip_prefix('"'))
        .and_then(|rest| {
            let body = match rest.rfind('}') {
                Some(brace) => &rest[..brace],
                None => rest,
            };
            let end = body.rfind('"')?;
            Some(body[..end].to_string())
        })
        .filter(|reason| !reason.is_empty());

    (score, reason)
}

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

/// What a scorer said about one run. `not_applicable` is the scorer declining to measure this
/// case: an explicit `{"score": null}`. A bare `null` stays an error, since a scorer that forgot
/// to return is indistinguishable from one that returned nothing on purpose.
#[derive(Default)]
struct Verdict {
    score: Option<f64>,
    reason: Option<String>,
    checks: Option<serde_json::Value>,
    not_applicable: bool,
}

impl Verdict {
    fn scored(score: f64) -> Self {
        Verdict { score: Some(score), ..Default::default() }
    }
}

/// A scorer may return a bare number, a boolean, or `{score, reason, checks}`; an agent wraps its
/// answer in `output`, sometimes as a string holding any of those. Anything with no number in it
/// is left empty rather than guessed at.
fn extract_verdict(value: &RawValue) -> Verdict {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(value.get()) else {
        return Verdict::default();
    };
    fn as_number(value: &serde_json::Value) -> Option<f64> {
        match value {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }
    if let Some(number) = as_number(&parsed) {
        return Verdict::scored(number);
    }
    let serde_json::Value::Object(map) = &parsed else {
        // A judge often answers with JSON inside a string, and often fences it as markdown even
        // when told to reply with JSON only.
        if let serde_json::Value::String(text) = &parsed {
            let text = unfence(text);
            if let Ok(inner) = serde_json::from_str::<serde_json::Value>(text) {
                if let Ok(raw) = serde_json::value::to_raw_value(&inner) {
                    return extract_verdict(&raw);
                }
            }
            // Nearly JSON: a judge that quotes the agent inside its own reason writes those quotes
            // unescaped, which is invalid and also the most ordinary thing for it to say. The
            // number is what the column plots, so it is read out of the text rather than lost with
            // the object around it.
            let (score, reason) = salvage_verdict(text);
            return Verdict { score, reason, checks: None, not_applicable: false };
        }
        return Verdict::default();
    };
    let reason = || {
        map.get("reason")
            .or_else(|| map.get("comment"))
            .and_then(|r| r.as_str())
            .map(|r| r.to_string())
    };
    if let Some(score) = map.get("score").and_then(as_number) {
        return Verdict {
            score: Some(score),
            reason: reason(),
            checks: map.get("checks").cloned(),
            not_applicable: false,
        };
    }
    // Written out rather than merely absent, which is what separates it from a scorer that
    // returned an object with no verdict in it at all.
    if map.get("score").is_some_and(|s| s.is_null()) {
        return Verdict {
            score: None,
            reason: reason(),
            checks: map.get("checks").cloned(),
            not_applicable: true,
        };
    }
    match map.get("output") {
        Some(output) => match serde_json::value::to_raw_value(output) {
            Ok(raw) => extract_verdict(&raw),
            Err(_) => Verdict::default(),
        },
        None => Verdict::default(),
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
    /// unrecognised is a silently empty cell rather than an error.
    #[test]
    fn extract_verdict_reads_every_documented_scorer_shape() {
        let score = |json: &str| extract_verdict(&raw(json)).score;
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

        // A judge quoting the agent inside its own reason, which is invalid JSON.
        let quoted = extract_verdict(&raw(
            r#"{"output": "{\"score\": 0.8, \"reason\": \"invented context (\"stop asking me\", never said) here\"}"}"#,
        ));
        assert_eq!(quoted.score, Some(0.8));
        assert_eq!(
            quoted.reason.as_deref(),
            Some(r#"invented context ("stop asking me", never said) here"#)
        );

        // nothing numeric to plot: left empty rather than guessed at
        assert_eq!(score(r#"{"output": "not a score"}"#), None);
        assert_eq!(score(r#"{"verdict": "good"}"#), None);

        let full = extract_verdict(&raw(
            r#"{"score": 0.5, "reason": "half", "checks": [{"name": "a"}]}"#,
        ));
        assert_eq!(
            (full.score, full.reason),
            (Some(0.5), Some("half".to_string()))
        );
        assert!(full.checks.is_some());
        assert!(!full.not_applicable);

        // `comment` as the rationale, which is what a scorer written for LangSmith or Langfuse
        // returns. Read rather than dropped, since the number arrives either way.
        assert_eq!(
            extract_verdict(&raw(r#"{"score": 1, "comment": "fine"}"#))
                .reason
                .as_deref(),
            Some("fine")
        );
    }

    /// A score is a fraction: anything outside 0..=1 (a scorer that returned a count, say) is
    /// recorded as an error naming the value rather than plotted as a bogus point.
    #[test]
    fn an_out_of_range_score_is_recorded_as_an_error_not_a_value() {
        // In range: recorded as the score it is.
        let (v, e) = settle_verdict(Some(&raw("0.5")), Some("success"), None).unwrap();
        assert_eq!(v.score, Some(0.5));
        assert!(e.is_none());
        // Out of range (a scorer returning a count, say): no score, an error naming the value.
        let (v, e) = settle_verdict(Some(&raw("100")), Some("success"), None).unwrap();
        assert_eq!(v.score, None);
        assert!(e.unwrap().contains("100"));
        let (v, _) = settle_verdict(Some(&raw("-5")), Some("success"), None).unwrap();
        assert_eq!(v.score, None);
        // No result at all once the iteration is over: an error, not a cell pending forever.
        let (v, e) = settle_verdict(None, Some("success"), None).unwrap();
        assert_eq!(v.score, None);
        assert!(e.is_some());
        // Still running: nothing to settle yet.
        assert!(settle_verdict(None, None, None).is_none());
    }

    /// A scorer saying it has nothing to measure on a case is a verdict rather than a failure: the
    /// cell is left out of the mean instead of counted as a zero. Spelled out, so a scorer that
    /// returns nothing at all is still an error rather than silently excused.
    #[test]
    fn an_explicit_null_score_is_not_applicable_rather_than_missing() {
        let na = extract_verdict(&raw(r#"{"score": null, "reason": "no sources to cite"}"#));
        assert!(na.not_applicable);
        assert_eq!(na.score, None);
        assert_eq!(na.reason.as_deref(), Some("no sources to cite"));

        // Through a judge's wrapper, as any other verdict is.
        assert!(extract_verdict(&raw(r#"{"output": {"score": null}}"#)).not_applicable);
        assert!(extract_verdict(&raw(r#"{"output": "{\"score\": null}"}"#)).not_applicable);

        // Not the same as a scorer that returned nothing, or an object with no verdict in it.
        assert!(!extract_verdict(&raw("null")).not_applicable);
        assert!(!extract_verdict(&raw(r#"{"verdict": "good"}"#)).not_applicable);
    }
}
