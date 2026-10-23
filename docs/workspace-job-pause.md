# Workspace job pause

A workspace admin can pause new jobs of a workspace until a deadline (`workspace_job_pause`),
typically around a deploy: `wmill sync push --pause-jobs`, `wmill workspace pause-jobs`, the
General tab of workspace settings, or `pauseWorkspaceJobs` / `pause_workspace_jobs` from a flow.

## What a pause does to a pushed job

`push_inner` asks `windmill_common::job_pause::check_push` about every job pushed with no parent:

- **Runs**: dependency, flow/app dependency, git sync (`deploymentcallback`) jobs; runs started
  directly by an admin (no trigger path and a `webhook` or empty trigger kind, see
  `pause_decision`); anything pushed under `PausePolicy::Exempt` (datatable migrations, raw app
  bundles). A job with a parent always runs: the flow or job it belongs to already started.
  "Admin" is the caller (`push`'s `authed`), never the identity the job runs as: a member running
  an on-behalf-of-admin script is held. An on-behalf-of run reaches `push` with no `authed`, so
  the run endpoints exempt an admin's such run themselves (`exempt_if`, `sync_run_policy`).
- **Held**: script, flow and single-step-flow jobs are queued as `unassigned_*`, 30 days out, the
  same way a suspended trigger holds its jobs, plus a `workspace_held_job` row carrying when the
  job was due.
- **Refused** (`Error::JobsPaused`, 503 + `Retry-After`): any other kind (previews, app inline
  scripts), and anything pushed under `PausePolicy::Refuse`.

A caller that waits for the result must push under `with_policy(PausePolicy::Refuse, ..)`, or a
held job leaves it waiting until it times out. That covers `run_wait_result*`, `run_and_stream`,
`run_flow_and_wait_result`, dynamic selects, app components and the trigger helpers that wait for
a result (sync and SSE HTTP routes, websocket return messages, initial messages and URLs). A new
synchronous entry point needs the same wrapper. The policy is a task-local: it does not cross
`tokio::spawn`, so the push must run on the task that entered the scope.

## Invariants

- `workspace_held_job` is what tells a pause-held job from one a suspended trigger holds; both are
  `unassigned_*` with the same trigger columns. Trigger resume/cancel exclude the held rows, and
  pause resume releases only them.
- Pausing also holds the workspace's queued jobs that have not started (`hold_queued_jobs`): a
  schedule's next run is queued ahead of time and would otherwise run mid-deploy. Release sets
  `scheduled_for = GREATEST(held due time, now())`, so that run keeps its slot.
- The pause is runtime state: it is not in the settings document (a settings push must not clear
  it, a fork must not start paused) and is read through a 10s cache evicted by `notify_event`.
  For up to that long after a pause starts, another server or worker can still queue or start a
  job; the monitor re-sweeps paused workspaces every pass, and `sync push --pause-jobs` waits out
  the TTL before deploying so a job that started in that window runs on the pre-deploy state.
  The sweep only judges jobs created before `paused_at` (the start of the pause, kept when it is
  extended) plus that window: later ones were judged by `check_push`, which knows who started
  them, and the sweep must not hold a job `check_push` let through.
- Pausing, resuming and the monitor's release take a per-workspace advisory lock
  (`lock_workspace`), so a release never acts on a "not paused" read a new pause made stale.
  Every pause request renews `pause_id`; `sync push --pause-jobs` resumes with its own id and gets
  a 409 instead if another deploy paused since.
- A held job is `unassigned_*` and due 30 days out; "Run now" refuses it, since a worker fails an
  `unassigned_*` job it reaches.
- A job a waiting caller (`run_wait_result`, a sync route) pushed before the pause and that has not
  started yet is held by the sweep like any queued job: that caller times out rather than getting
  a 503, and the job runs on resume. Telling those jobs apart would need per-job state.
- A pause past its deadline reads as over immediately; `monitor_db` deletes it and releases its
  jobs, and also releases held rows left without a pause (a push that read a stale cache).
- `MAX_PAUSE_DAYS` (7) must stay well under the 30-day horizon after which a worker fails an
  `unassigned_*` job it reaches. A pause can be renewed indefinitely, so every renewal also moves
  the jobs it already holds 30 days out again.
- Held jobs run the version they were pushed with; resume does not re-resolve them to the latest
  deployed version.
