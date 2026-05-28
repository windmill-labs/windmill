//! # Per-workspace fairness for the shared worker pool (Enterprise feature)
//!
//! On multi-tenant deployments (notably `app.windmill.dev`, and any EE
//! cluster with a single shared worker group) a single workspace flooding
//! the queue with jobs can degrade quality of service for everyone else.
//! This module computes the set of "overloaded" workspaces whose share of
//! the worker pool must be capped, and the dispatch in `jobs.rs` uses a
//! **duration-weighted stochastic admission rule** at pull time to enforce
//! the cap as a *worker-second* share, not a pull-count share.
//!
//! The full algorithm lives in [`crate::workspace_fairness_ee`] behind the
//! `private` feature — this OSS-facing module is the public surface that
//! the pull dispatch and integration tests call. When EE is on, the symbols
//! here transparently re-export the EE implementation. When EE is off, they
//! are no-ops: `maybe_refresh_overloaded` does nothing, `should_admit_capped`
//! always returns `true`, and the pull path is bit-identical to its
//! pre-fairness shape. **Both runtime correctness and the entire reasoning
//! below assume the EE module is compiled in**; the OSS build is a stub.
//!
//! Every numerical default mentioned below (`MAX_PERCENT = 50`,
//! `DURATION_SECS = 10`, `MIN_TOTAL = 4`, `WORKER_PING_LIVE_SECS = 60`,
//! `ADMISSION_EPSILON_PERCENT = 5`) is tunable via global settings or
//! constants; the values here are the as-shipped defaults at the time of
//! writing and what the design discussion below was calibrated against.
//!
//! ## 1. What "overloaded" means — worker-seconds, not jobs
//!
//! A workspace is overloaded when, over a rolling
//! `WORKSPACE_FAIRNESS_DURATION_SECS = 10s` window, it has consumed at least
//! `WORKSPACE_FAIRNESS_MAX_PERCENT = 50%` of cluster worker-time. Activity
//! is measured in **worker-seconds**: each job contributes the wall-clock
//! time it actually held a worker, intersected with the window. A
//! count-based signal — "what fraction of jobs in the window are from this
//! workspace" — gets badly fooled by job-duration heterogeneity: 600
//! short (100ms) jobs and one long (60s) job consume the same worker-time
//! but the count-based form attributes 600× more weight to the spammy
//! workspace. Worker-seconds put both patterns on the same scale.
//!
//! Two sources contribute to a workspace's worker-second total:
//!
//! - **Running** (live, currently-on-a-worker): driven from `v2_job_runtime`
//!   filtered on `ping > now() - WORKER_PING_LIVE_SECS` (60s, ≈ 2× worker
//!   heartbeat interval), then PK-joined to `v2_job` for the `kind` filter
//!   and `v2_job_queue` for `started_at` / `suspend_until`. Contribution is
//!   `clamp(min(now, ping) − max(started_at, window_start), 0, window)`.
//!   End-of-interval is the per-job `ping`, which both (a) implements the
//!   zombie defense — a worker that stopped pinging stops accruing
//!   worker-seconds at its last heartbeat, so a backlog of stuck
//!   `running = true` rows can't dominate the denominator — and (b) matches
//!   the semantic of "worker-seconds the worker has confirmed". `v2_job_runtime`
//!   is small (rows deleted on completion), so driving the scan from there
//!   keeps the per-refresh cost bounded by the *in-flight* count rather
//!   than by the queue size, even when one workspace has thousands of
//!   `running = true` rows.
//!
//! - **Completed** (recently finished): pulled by an index scan over
//!   `v2_job_completed (completed_at)`, then PK-joined to `v2_job`. The
//!   index hit is critical — see "Why no `WITH params AS (...)` CTE" below.
//!   Contribution is `clamp(min(completed_at, now) − max(started_at,
//!   completed_at − duration_ms, window_start), 0, window)`. Clamping
//!   start-of-interval by `completed_at − duration_ms` defends against
//!   zombie rows that `zombie_monitor` force-failed: `started_at` may be
//!   far in the past, but `duration_ms` reflects the actual measured worker
//!   time, so the row only contributes its real runtime, not the idle wait
//!   before force-fail.
//!
//! Both halves exclude **flow-orchestration kinds**
//! (`flow, flowpreview, flownode, singlestepflow`) and **concurrency-
//! suspended rows** (`suspend_until IS NOT NULL`) — these hold
//! `running = true` but consume no worker slot. Same predicate as
//! `handle_zombie_jobs` in `monitor.rs`.
//!
//! `WORKSPACE_FAIRNESS_MIN_TOTAL = 4` is also in worker-seconds (≈ 40 %
//! utilization of one worker over a 10s window) — below the floor, the
//! cluster is too quiet to bother capping anyone.
//!
//! ## 2. The cap is enforced stochastically, weighted by duration
//!
//! The pull dispatch in `jobs.rs` flips a coin on every pull: with
//! probability `p_c` it uses the standard pull query (capped workspaces
//! are admissible — FIFO will pick them if they're at the head), and with
//! probability `1 − p_c` it uses the *fairness pull query* which excludes
//! the overloaded workspaces. Doing it as a probabilistic split rather
//! than a binary cap/uncap gate keeps victim latency flat instead of
//! breathing in/out with each refresh cycle.
//!
//! The key design choice is how `p_c` is set. The natural first try is
//! `p_c = (MAX_PERCENT + ε) / 100` — a constant. That converges the
//! *pull-count* ratio to `MAX_PERCENT`, but only matches the worker-second
//! ratio when capped and uncapped workspaces share the same mean job
//! duration. The steady-state share equation is:
//!
//!   `share = p_c · D_c / (p_c · D_c + (1 − p_c) · D_u)`
//!
//! where `D_c` and `D_u` are the per-job mean durations of capped and
//! uncapped workspaces respectively. With `D_c = 34s` and `D_u = 1s` (the
//! exact numbers observed during the lancom01-prod / jps-internal cloud
//! incident), a constant `p_c = 0.65` (60 % + 5 % ε) yields
//!
//!   `share = 0.65 · 34 / (0.65 · 34 + 0.35 · 1) = 22.1 / 22.45 ≈ 98%`
//!
//! — i.e., the "60 % cap" was in practice giving capped workspaces 98 %
//! of worker-seconds. Victims were observed waiting 15s+ for pickup
//! despite the cap firing on every pull.
//!
//! Inverting the equation for the desired share `t = (MAX_PERCENT + ε) / 100`:
//!
//!   `p_c = t · D_u / ((1 − t) · D_c + t · D_u)`
//!
//! Same numbers, target 0.65: `p_c ≈ 0.054` — about 12× tighter than the
//! count-based form. The refresh computes `p_c` and stores it in
//! [`WORKSPACE_FAIRNESS_ADMISSION_PPM`] (parts-per-10_000, fits in an
//! `AtomicU32`). The pull-time check is one atomic load plus one
//! `rand::rng().random_range(0..10_000)` draw — same hot-path cost as the
//! count-based form.
//!
//! ### `D_c`/`D_u` come from a separate, longer service-time window
//!
//! Crucially, `D_c` and `D_u` must be **true mean service times**, because
//! the share equation above is Little's-law-based
//! (`occupancy = arrival_rate × mean_service_time`). They are **not** taken
//! from the occupancy aggregation: that aggregation clamps each job's
//! contribution to the short occupancy window (`DURATION_SECS`, 10s), so a
//! job longer than the window contributes at most 10s — fine for measuring
//! *share*, but it would truncate `D_c` to ≤ 10s and systematically
//! under-admit the skew exactly when capped jobs are long (the case the cap
//! exists for: e.g. true `D_c = 34s` clamped to 10s gives `p_c ≈ 0.157`, an
//! 86 % effective share instead of 65 %). Instead, the refresh samples true
//! unclamped `duration_ms` of completed jobs over a longer, decoupled
//! service-time window (`DURATION_SAMPLE_SECS`, 60s) — long enough to avoid
//! truncation and to keep the mean stable when few jobs complete within the
//! 10s occupancy window. So the refresh emits two per-workspace signals:
//! windowed occupancy worker-seconds (for classification) and a 60s
//! service-time `(Σ duration_ms, count)` (for admission), merged per
//! workspace.
//!
//! ### Why we kept the fallback when the fairness pull returns empty
//!
//! The 100 − `p_c` % of pulls that try the fairness query (excluding
//! capped workspaces) fall back to the standard query if the fairness
//! query returns no row. The alternative — idle the worker, holding the
//! slot open in case a victim shows up — was considered but rejected for
//! the first iteration: with `p_c` correctly tightened, victims do get the
//! slot they need *when they exist*, and absent victims, falling back to
//! the capped pool is the right behaviour (otherwise the cluster
//! under-utilises itself for no benefit). Adding a reserve-capacity skip
//! is a fine-tuning lever for bursty victim arrival patterns and is left
//! as a follow-up.
//!
//! ### Degenerate cases
//!
//! If either bucket is empty — no capped jobs, no uncapped jobs, or a
//! capped workspace with zero completions in the 60s service-time window
//! (all its jobs still running) — the formula is undefined. The refresh
//! falls back to the count-based `p_c = t` in those cases — it matches
//! the pre-refactor behaviour and is the safest thing to do when there's
//! no service-time signal yet to weight on.
//!
//! ## 3. Coordinated refresh — exactly once per cycle, cluster-wide
//!
//! The aggregation is too expensive to run on every worker process every
//! pull (and would produce no new information on the sub-second
//! timescale). It runs **at most once every `refresh_interval` seconds
//! across the entire fleet**, gated by both a per-process CAS and a
//! DB-side row lock:
//!
//! 1. **Per-process gate** — `maybe_refresh_overloaded` (called from the
//!    pull path) does `LAST_REFRESH_MICROS.compare_exchange` to ensure at
//!    most one in-flight refresh per process per interval. If the CAS
//!    fails or the interval hasn't elapsed yet, the call is a no-op.
//!    Cost on the hot path: one atomic load, optionally one CAS.
//!
//! 2. **DB-side claim** — `refresh_overloaded` first does a cheap upsert
//!    (`INSERT ... ON CONFLICT ON background_task_state ... WHERE
//!    updated_at < NOW() − refresh_interval RETURNING true`). The `VALUES`
//!    clause is all constants, so Postgres has no expensive work to do
//!    even for losers. Only the unique winner per cycle gets `Some(true)`;
//!    losers get `None` and skip the aggregation entirely.
//!
//! 3. **Winner-only aggregation** — the winner runs the
//!    `v2_job_runtime ∪ v2_job_completed` worker-second aggregation
//!    returning per-workspace `(workspace_id, worker_seconds, jobs)`,
//!    classifies into overloaded/uncapped, computes `p_c`, and writes the
//!    new payload `{"overloaded": [...], "admission_ppm": N}` back to
//!    `background_task_state.workspace_fairness`.
//!
//! 4. **Everyone reads** — winner and losers alike then `SELECT` the
//!    current value, parse it, and update their in-process
//!    `WORKSPACE_FAIRNESS_OVERLOADED` and `WORKSPACE_FAIRNESS_ADMISSION_PPM`
//!    atomics. This is what makes losers eventually see the winner's
//!    decision; they just don't pay the aggregation cost.
//!
//! The refresh interval is `ACTIVE_REFRESH_SECS = 2s` when the cluster
//! currently has a capped workspace (faster — we want the cap to lift
//! promptly once load drops) and `IDLE_REFRESH_SECS = 5s` otherwise
//! (slower — minimise DB load during normal operation). The DB-side guard
//! always uses the tighter `ACTIVE_REFRESH_SECS` to bound the race
//! window; the per-process gate enforces the idle cadence.
//!
//! If a refresh fails (DB error, timeout > 5s), `LAST_REFRESH_MICROS` is
//! left set to the attempt's timestamp so the next attempt has to wait a
//! full interval — exactly the same cooldown as a successful refresh.
//! Resetting to `0` on failure would remove the rate limit entirely
//! precisely when DB load is highest, which is the wrong direction.
//!
//! ## 4. Audit logging
//!
//! Workspaces entering or leaving the capped set produce
//! `workspace_fairness.capped` / `workspace_fairness.uncapped` audit
//! entries scoped to the `admins` workspace, with the affected workspace
//! as the `resource` field. Emitted by the refresh winner only, so a
//! transition produces exactly one audit row regardless of fleet size.
//! The "previous list" diffed against is the DB value (not the per-process
//! cache) so a freshly-restarted worker that happens to win the first
//! claim doesn't emit spurious "newly capped" entries for workspaces that
//! were already capped before it started.
//!
//! ## 5. Notable SQL performance constraints
//!
//! - **No `WITH params AS (...)` CTE for `window_start`.** A natural
//!   refactor would be to compute `NOW() - make_interval(secs => N)` once
//!   in a CTE and reference it in both halves of the UNION. But Postgres
//!   *materialises* the CTE and the optimiser can no longer push the
//!   `completed_at > window_start` predicate down to the
//!   `ix_job_completed_completed_at` index. On the production cloud DB
//!   (~12M `v2_job_completed` rows), that turns a 10 ms index scan into a
//!   ~47s full table scan. The query intentionally inlines `NOW()` and
//!   `NOW() - make_interval(...)` at every callsite.
//!
//! - **Drive running side from `v2_job_runtime`, not `v2_job_queue`.**
//!   Naive ordering ("scan v2_job_queue for `running = true`, join v2_job
//!   for the kind filter") does a Seq Scan over ~thousands of running-or-
//!   bookkeeping rows and does a PK lookup into `v2_job` for every one of
//!   them — ~10 ms in prod, but worse: bounded by *queue size*. Pivoting
//!   to drive the scan from `v2_job_runtime` filtered on
//!   `ping > NOW() - 60s` narrows to the in-flight set (small, deletes-
//!   on-completion) *before* any PK lookups: 1.3 ms, 9× less I/O,
//!   bounded by *live worker count*.
//!
//! ## 6. Enterprise gating
//!
//! The cap is an Enterprise feature. `windmill-api-settings` rejects
//! `workspace_fairness_enabled = true` writes from non-EE builds, and on a
//! single-tenant self-hosted deployment the default
//! `workspace_fairness_enabled = false` keeps the pull path identical to
//! the pre-fairness baseline. At runtime the dispatch checks the atomic
//! only — when fairness is off, `maybe_refresh_overloaded` drains the
//! cached state in one pull cycle (resetting `WORKSPACE_FAIRNESS_OVERLOADED`
//! to empty and `WORKSPACE_FAIRNESS_ADMISSION_PPM` to 10_000 = "admit all"),
//! so toggling the feature off without restarting workers is safe.

#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::workspace_fairness_ee::*;

#[cfg(not(feature = "private"))]
mod oss_stubs {
    use sqlx::{Pool, Postgres};

    /// No-op on OSS — workspace fairness is an Enterprise feature.
    #[inline]
    pub fn maybe_refresh_overloaded(_db: &Pool<Postgres>) {}

    /// No-op on OSS — always returns `true` so the dispatch never reaches
    /// the fairness pull query (which is empty anyway, since
    /// `store_pull_query` only materialises it when fairness is enabled).
    #[inline]
    pub fn should_admit_capped() -> bool {
        true
    }
}

#[cfg(not(feature = "private"))]
pub use oss_stubs::*;
