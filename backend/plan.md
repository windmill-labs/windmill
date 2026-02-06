╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌
 Plan: Extract shared try_schedule_next_job function

 Context

 Commit 718938d88a inlined the schedule push logic into commit_completed_job to reuse the outer transaction (fixing pool connection exhaustion). This created two problems:
 - Con 2: Two diverging code paths — the inlined logic in commit_completed_job and the old handle_maybe_scheduled_job still used by worker_flow.rs — with different retry semantics and connection models
 - Con 3: +108 lines inlined in commit_completed_job, making it harder to read

 Approach

 Extract the inlined schedule push logic into a shared pub async fn try_schedule_next_job() that both call sites use. Delete handle_maybe_scheduled_job.

 Function signature

 pub async fn try_schedule_next_job<'c>(
     db: &Pool<Postgres>,
     mut tx: Transaction<'c, Postgres>,
     completed_job: &MiniCompletedJob,
     schedule: &Schedule,
     script_path: &str,
 ) -> (Transaction<'c, Postgres>, Option<Error>)

 Returns (tx, None) on success (including graceful cases: disabled schedule, path mismatch, schedule disabled after push failure). Returns (tx, Some(QuotaExceeded)) if quota hit. Returns (tx, Some(err)) if both push and schedule-disable failed (caller triggers zombie retry).

 The tx is always returned so the caller can continue using it or drop it (rollback) for the zombie path.

 Logic (moved from current inlined code)

 1. Guard: !schedule.enabled → return (tx, None)
 2. Guard: script_path != schedule.script_path → return (tx, None)
 3. Fetch schedule_authed via fetch_authed_from_permissioned_as_conn(&mut *tx)
 4. Savepoint retry loop (3 attempts, 2s sleep):
   - Create savepoint via tx.begin()
   - Call push_scheduled_job(db, savepoint, ...)
   - Commit savepoint on success
   - Return early on QuotaExceeded
 5. On persistent push failure:
   - Disable schedule on &mut *tx
   - Report via report_error_to_workspace_handler_or_critical_side_channel
   - If disable also fails: return (tx, Some(err))

 Files to modify

 1. windmill-queue/src/jobs.rs

 - Add try_schedule_next_job function (place near removed handle_maybe_scheduled_job, ~line 1879)
 - Simplify commit_completed_job schedule_next_tick block (lines 1102-1227) to:
 if schedule_next_tick {
     let (returned_tx, schedule_push_err) =
         try_schedule_next_job(db, tx, completed_job, &schedule, script_path).await;
     tx = returned_tx;
     if let Some(err) = schedule_push_err {
         if !matches!(err, Error::QuotaExceeded(_)) {
             return Ok((Some(job_id), 0, true));
         }
     }
 }
 - Remove handle_maybe_scheduled_job (lines 1879-1973)

 2. windmill-worker/src/worker_flow.rs

 - Update import (line 70): replace handle_maybe_scheduled_job with try_schedule_next_job
 - Update call site (lines 2179-2195):
 if let Some(schedule) = schedule {
     let tx = db.begin().warn_after_seconds(5).await?;
     let (tx, schedule_err) = try_schedule_next_job(
         db, tx,
         &MiniCompletedJob::from(flow_job.clone()),
         &schedule,
         flow_job.runnable_path.as_ref().unwrap(),
     )
     .warn_after_seconds(5)
     .await;
     match schedule_err {
         Some(Error::QuotaExceeded(_)) => return Err(Error::QuotaExceeded("quota exceeded".to_string()).into()),
         Some(_) => return Ok(()),
         None => { tx.commit().await?; }
     }
 } else { ... }

 3. tests/schedule_push.rs

 - Update import (line 11): replace handle_maybe_scheduled_job with try_schedule_next_job
 - Update 5 test functions that call handle_maybe_scheduled_job to:
   - Create a transaction: let tx = db.begin().await?
   - Call try_schedule_next_job(&db, tx, ...)
   - Destructure (tx, err) and commit tx before asserting DB state
   - Check err instead of result.is_ok()

 Behavioral change for worker_flow.rs

 The flow path switches from pool-based auth + db.begin() (2 extra connections) to tx-based auth + savepoints (0 extra connections). Retry strategy changes from 10x/5s to 3x/2s (outer retry mechanisms compensate). QuotaExceeded no longer disables the schedule (improvement — quota is transient).

 Verification

 1. cargo check — full workspace compiles
 2. cargo test --test schedule_push — all tests pass
 3. cargo check -p windmill-worker — worker crate compiles with updated import
 4. Grep for handle_maybe_scheduled_job — zero results
