# Detecting skipped schedule occurrences

Design plan. Nothing in this document is implemented yet.

## The problem

Windmill's scheduler has no tick grid. It keeps exactly one queued occurrence
per schedule and re-anchors on the wall clock each time:

- `push_scheduled_job` computes `next = find_next(max(db_now, prev_scheduled_for + 1s))`
  (`backend/windmill-queue/src/schedule.rs:169`)
- **scripts** re-arm on completion (`windmill-queue/src/jobs.rs:1411`)
- **flows** re-arm on entry to step 0 (`windmill-worker/src/worker_flow.rs:2870`)

So when a run takes longer than the interval, or waits for a worker, the grid
silently shifts and the intervening occurrences never exist. Nothing records
that they were lost.

Measured on a dev instance, a 25 s script on `*/10 * * * * *`:

```
scheduled_for: 08:20:20 → 08:20:50 → 08:21:20 → 08:21:50
```

Every 30 s instead of every 10 s. Two of every three occurrences gone, with no
log line, no column, and no job row.

The two kinds degrade differently:

| cause | scripts | flows |
|---|---|---|
| run overruns the interval | occurrences lost (serialized by construction) | none lost, occurrences overlap |
| queue wait (workers busy) | occurrences lost | occurrences lost |
| reconciler re-armed a dead schedule | whole gap lost | whole gap lost |
| `paused_until`, `dynamic_skip`, `no_flow_overlap` | intentional | intentional |

## What users get

On the **schedules list**, a warning next to schedules that are losing runs,
alongside the existing paused and error indicators. Illustrative copy:

> 3 of the last 20 runs skipped occurrences

In the **schedule editor**, the numbers behind it:

> Interval 10s. The last run waited 0s for a worker, then ran for 25s,
> skipping 2 occurrences.

Those two numbers separate the two causes, which need opposite fixes: a large
wait means not enough workers, a long run means the job outgrew its interval.
No one-word label, because both usually contribute and any dominance rule would
mislabel the mixed case.

For **operators** (EE), an alert when a scheduled script is running and has
already passed its own next occurrence, so a wedged schedule pages someone
rather than waiting to be noticed.

This is diagnosis only. Nothing starts overlapping, nothing is queued up, and no
missed occurrence is retroactively fired.

## What already exists

Not to be rebuilt:

- **`outstanding_wait_time`** stores `started_at - scheduled_for` per job past
  `OUTSTANDING_WAIT_TIME_THRESHOLD_MS` (default 1000), survives completion, and
  is already on screen via `WaitTimeWarning.svelte`.
- **`jobs_waiting_alerts`** (EE) alerts on queue waiting per tag, configured
  through the `alert_job_queue_waiting` global setting, with `healthchecks`
  rows for healthy/unhealthy transitions and a cooldown. Its predicate carries
  `running = false`, so it is structurally blind to a run that overruns.
- **`find_unarmed_schedules`** plus the reconciler pass catches schedules with
  *no* queued occurrence. A wedged schedule has one, so it is not covered.
- **`schedule.error`** and the red dot on the schedules row are the existing
  per-schedule warning slot.

## The key property

`v2_job.created_at` of occurrence N+1 is bit-identical to `completed_at` of
occurrence N, because the push happens in the completing transaction and both
`now_from_db()` and the `created_at` default resolve to the same transaction
start time. Therefore:

```
scheduled_for = find_next(created_at)
```

holds exactly, not approximately. The entire occurrence sequence is
reconstructible from durable rows that already exist and are already indexed,
with nothing written at push time.

## Design

Three pieces, in three venues.

| piece | runs where | writes | edition |
|---|---|---|---|
| skipped-occurrence counting | read time, schedules API, server side | nothing | CE |
| "late right now" badge | read time, single aggregating join on `v2_job_queue` | nothing | CE |
| overrun alert | monitor pass, ~5 min | `healthchecks` + critical alert | EE |

`push_scheduled_job` is not touched.

### Skipped-occurrence counting

Server side, over the last N occurrences fetched per schedule: reconstruct each
`S_i = find_next(created_at_i)`, then count cron occurrences strictly between
consecutive `S_i`. The API returns the resulting counts and decomposition per
schedule, never the raw occurrence rows.

### The one write: a watermark column

A single column on `schedule`, advanced to `GREATEST(now(), paused_until)` at
`create_schedule`, `edit_schedule`, `set_enabled(true)` and `rearm_schedule`.
Gaps are only counted when both endpoints sit at or past it.

Every one of those is a cold path that already writes the schedule row, so the
watermark costs no additional statement, and nothing is written per occurrence.

It closes four holes with one mechanism:

- **pause windows**: `push_scheduled_job` anchors on `paused_until` rather than
  `now`, so the occurrence pushed during a pause has a `created_at` that does
  not predict its `scheduled_for`
- **cron edits**: a gap would otherwise be measured with the new expression
  across a period governed by the old one
- **disable then re-enable**
- **reconciler re-arms**, which are reported through `schedule.error` already

## Correctness hazards

Found while validating the approach, each one handled:

- **`status = 'skipped'` occurrences must not be filtered out.** Any hole in
  the middle of the sequence manufactures phantom skips. `list_schedule_with_jobs`
  (`windmill-api-schedule/src/lib.rs:1013`) carries `AND status <> 'skipped'`;
  measured against a `dynamic_skip` schedule, 4 actual root occurrences were
  visible as 0 through that filter. Per `CONTEXT.md` a polling flow's normal
  steady state is also `skipped`, so this is the common case rather than an
  edge. Reconstruction reads the unfiltered rows; the existing display array is
  left exactly as it is.
- **Order by `created_at`, not `completed_at`.** Flows overlap, so completion
  order is not creation order and consecutive-occurrence pairing would scramble.
- **A flow schedule holds two or more concurrent root rows in `v2_job_queue`**,
  since the successor is pushed at step 0 entry while its predecessor runs.
  Measured. The live join must aggregate per schedule rather than assume one row.
- **The walk is capped at 1000 occurrences**, displayed as `1000+`. Uses the
  existing bounded `ScheduleType::upcoming` primitive (`windmill-common/src/utils.rs:1082`).
- **Clock-skew clamp** (`now_cutoff + 1s`) makes the reconstruction wrong in the
  backwards-clock case. Rare, already logged at ERROR, accepted.

## Scope by runnable kind

The overrun signal (badge and alert) applies to **scripts only**. A flow that
runs longer than its interval has its successor already queued and starting on
time, which is the design intent, so alerting there would fire constantly on
healthy schedules. Occurrences that flows lose to **late starts** are still
counted by reconstruction, and that path is unaffected.

## Surfaces

- Schedules list row: a badge alongside the existing paused and error indicators
- Schedule editor: the decomposition, for example "interval 10s; last run waited
  0s, ran 25s"

No "lateness" versus "skips" label. The two components usually both contribute,
so any dominance rule would mislabel the mixed case; two numbers against the
interval say everything a label would and cannot be wrong.

Vocabulary: **occurrence**, matching `schedule.rs` throughout. Not "tick".

## Explicitly not doing

- No events table, no retention job, no RLS policy
- No change to `push_scheduled_job`
- No per-schedule alert configuration. The predicate compares against each
  schedule's own next occurrence, so it is self-calibrating across a daily and a
  per-minute schedule and needs no thresholds.
- No catch-up or overlap policy. Making the overrun behaviour configurable is a
  much larger change to the execution model, and it needs this diagnosis first
  to know which way people would want it to go.

## Tests

Unit tests on the pure reconstruction function, which takes
`(cron, timezone, Vec<created_at>, watermark)` and returns counts. No database
fixture, no worker, no end-to-end schedule test. Four cases, exactly the holes
above: a pause window, a skipped occurrence mid-sequence, a cron edit
mid-window, and the cap.

## Adjacent finding, out of scope

`outstanding_wait_time` has no foreign key, no cascade and no cleanup anywhere
in the codebase. Rows are inserted for every job that waits past the threshold
and never deleted.
