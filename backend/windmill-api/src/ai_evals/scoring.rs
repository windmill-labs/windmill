use super::*;

/// The version of a scorer's runnable that would run now. Part of what a score records, so a
/// script edited between two experiments is visible as a change of scorer rather than of agent.
pub(crate) async fn resolve_definition(db: &DB, w_id: &str, scorer: &Scorer) -> Result<String> {
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

/// Bring a run's record up to date with the flow that is executing it: which iteration answered
/// which case, and what its scorers returned.
///
/// Read-driven because there is nothing to drive it: the flow runs on workers that know nothing
/// about these tables. A run therefore holds its answers and its scores whether or not anyone is
/// watching, and this is what copies them into rows that outlive the jobs' retention.
/// A run's own last step calls this, so a run records itself whether or not anyone watched it.
/// Reading it does too, which is what covers a run whose flow never reached that step.
///
/// `answers` is what separates the two: a listing reports each run's score aggregates and never
/// shows an answer, so harvesting them there reads a column of every case of every listed run to
/// display none of it.
pub(crate) async fn sync_run(
    db: &DB,
    w_id: &str,
    experiment_id: Uuid,
    run_job_id: Uuid,
    answers: bool,
) -> Result<()> {
    backfill_case_jobs(db, w_id, experiment_id, run_job_id).await?;
    if answers {
        record_case_answers(db, w_id, experiment_id).await?;
    }
    harvest_flow_scores(db, w_id, experiment_id).await?;
    Ok(())
}

/// In-flight reads of what a run produced. Each is several queries and a run holds up to
/// `MAX_CASES_PER_RUN` cases, so they go a few at a time rather than one after another.
const HARVEST_CONCURRENCY: usize = 8;

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
            return (row.ordinal, None, None, Some("unavailable".to_string()));
        }
        // The agent step's own result, never the iteration's: the iteration goes on to score the
        // answer, so the answer is settled long before the iteration is.
        let agent = windmill_queue::get_result_and_success_by_id_from_flow(
            db,
            w_id,
            &row.job_id,
            AGENT_NODE_ID,
            None,
        )
        .await
        .ok();
        // An iteration that ended without an answer — skipped, cancelled, or an agent that failed
        // outright — produced none, and saying so is what stops this re-reading it.
        let answered = agent
            .as_ref()
            .map(|(_, success)| *success)
            .or_else(|| row.status.is_some().then_some(false));
        let output = agent.as_ref().and_then(|(result, _)| agent_answer(result));
        (row.ordinal, output, answered, row.status)
    }))
    .buffered(HARVEST_CONCURRENCY)
    .collect::<Vec<_>>()
    .await;

    for (ordinal, output, answered, status) in answers {
        // Nothing to record yet, and the iteration may still produce it.
        if answered.is_none() && status.is_none() {
            continue;
        }
        sqlx::query!(
            "UPDATE eval_experiment_case
             SET output = COALESCE(output, $3), answered = COALESCE(answered, $4),
                 status = COALESCE(status, $5)
             WHERE experiment_id = $1 AND ordinal = $2",
            experiment_id,
            ordinal,
            output,
            answered,
            status,
        )
        .execute(db)
        .await?;
    }
    Ok(())
}

/// Match each case to the iteration that ran it. The flow engine mints those job ids, so the case
/// they belong to is read back from the iteration's own arguments — the case is what the loop
/// iterates over, so it is there by construction, and it survives iterations finishing in any
/// order.
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
                (j.id IS NOT NULL) AS \"job_exists!\"
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

    use futures::StreamExt;
    let verdicts = futures::stream::iter(pending.into_iter().map(|row| async move {
        // Nothing left to read the verdict out of. Settled here, since a cell left pending is one
        // every later listing would go back to this same absent job for.
        if !row.job_exists {
            return (
                row.ordinal,
                row.scorer_id,
                Some((
                    Verdict::default(),
                    Some("The run that produced this score is no longer available".to_string()),
                )),
            );
        }
        let verdict = read_verdict(
            db,
            w_id,
            row.job_id,
            &row.scorer_id,
            row.status.as_deref(),
            "The case produced no answer to score",
        )
        .await;
        (row.ordinal, row.scorer_id, verdict)
    }))
    .buffered(HARVEST_CONCURRENCY)
    .collect::<Vec<_>>()
    .await;

    for (ordinal, scorer_id, read) in verdicts {
        // Still to come: a scorer whose own step has not run yet.
        let Some((verdict, error)) = read else {
            continue;
        };
        sqlx::query!(
            "UPDATE eval_score
             SET score = $4, reason = $5, checks = $6, error = $7, not_applicable = $8
             WHERE experiment_id = $1 AND ordinal = $2 AND scorer_id = $3",
            experiment_id,
            ordinal,
            scorer_id,
            verdict.score,
            verdict.reason,
            verdict.checks,
            error,
            verdict.not_applicable,
        )
        .execute(db)
        .await?;
    }
    Ok(())
}

/// One scorer's verdict, read out of the job that produced it, which may still be running: a
/// scorer's own step can be done while the iteration around it is not. `None` while the result is
/// not readable yet, which is a state to wait through rather than to record as a failure.
async fn read_verdict(
    db: &DB,
    w_id: &str,
    scoring_job: Uuid,
    scorer_id: &str,
    job_status: Option<&str>,
    // What to record when the job is over and this scorer produced nothing. A different statement
    // depending on where the scorer ran: its own job failed, or the case it was to score never
    // produced an answer.
    missing_error: &str,
) -> Option<(Verdict, Option<String>)> {
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
            let verdict = extract_verdict(&value);
            match verdict {
                // A number, or the scorer saying this case is not one it measures. Both are
                // answers, so both are recorded and neither is an error.
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
                _ => (verdict, Some(missing_error.to_string())),
            }
        }
        None if job_status == Some("success") => return None,
        // The job holding this scorer has not finished, so a module with nothing in it yet is a
        // step that has not run rather than one that produced nothing.
        None if job_status.is_none() => return None,
        None => (Verdict::default(), Some(missing_error.to_string())),
    })
}

/// The score and reason read straight out of text that failed to parse as JSON. Deliberately not a
/// second JSON parser: it looks for the two keys and takes what follows, which is what survives a
/// model writing an unescaped quote in the middle of a sentence.
fn salvage_verdict(text: &str) -> (Option<f64>, Option<String>, Option<serde_json::Value>) {
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

    (score, reason, None)
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
/// case rather than failing to: an explicit `{"score": null}`, which is what Braintrust's scorers
/// return for the same thing. A bare `null` stays an error, since a scorer that forgot to return
/// is indistinguishable from one that returned nothing on purpose.
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
        // when told to reply with JSON only. Both are the model doing what it was asked; refusing
        // to read them is what turns a good verdict into "no number to plot".
        if let serde_json::Value::String(text) = &parsed {
            let text = unfence(text);
            if let Ok(inner) = serde_json::from_str::<serde_json::Value>(text) {
                if let Ok(raw) = serde_json::value::to_raw_value(&inner) {
                    return extract_verdict(&raw);
                }
            }
            // Nearly JSON: a judge that quotes the agent inside its own reason writes those quotes
            // unescaped, which is invalid and is also the most ordinary thing for it to say. The
            // number is what the column plots, so it is read out of the text rather than lost with
            // the object around it.
            let (score, reason, checks) = salvage_verdict(text);
            return Verdict { score, reason, checks, not_applicable: false };
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
    /// unrecognised is a silently empty cell, not an error.
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

        // A judge quoting the agent inside its own reason, which is invalid JSON and the most
        // ordinary sentence for it to write. The number is what the column plots, so it survives.
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

    /// A scorer saying it has nothing to measure on a case, which is a verdict rather than a
    /// failure: the cell is left out of the mean instead of counted as a zero. Spelled out, so a
    /// scorer that returns nothing at all is still an error rather than silently excused.
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
