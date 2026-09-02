# Detecting skipped schedule occurrences

The design behind the schedule occurrence counting, and the reasoning that
settled each choice.

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

The two kinds degrade differently. "Serialized" is a plain script schedule;
"overlapping" is a flow, or a script carrying `retry` or `dynamic_skip` (see
"Scope by runnable kind"):

| cause | serialized | overlapping |
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

For **operators** (EE), an alert when a scheduled run is still going and has
already passed its own next occurrence, so a wedged schedule pages someone
rather than waiting to be noticed.

This is diagnosis only. Nothing starts overlapping, nothing is queued up, and no
missed occurrence is retroactively fired.

## Which cause is caught, and when

Two things lose occurrences, and they are caught differently.

| | overrun (ran longer than the interval) | late start (waited for a worker) |
|---|---|---|
| plain script schedule | counted | counted |
| script with `retry` or `dynamic_skip` | nothing to catch | counted |
| flow schedule | nothing to catch | counted |

Where occurrences are **serialized**, the successor is pushed when the current
run finishes, so the gap it opens is governed by wait and duration together.
Reconstruction sees the gap whichever one caused it, and the editor splits it
back into the two numbers.

Where occurrences **overlap**, the successor is pushed when the current run
*starts*, so only the wait can move the grid forward. A run longer than the
interval loses nothing there, which is the design intent, so there is nothing to
report.

All of this is retrospective. A gap becomes visible only once the *next*
occurrence exists, which needs the current one to finish (serialized) or start
(overlapping). A schedule that is wedged right now — three hours into a run on a
ten-minute cron, or still queued behind busy workers — shows nothing until it
moves. That blind spot is what the live badge and the operator alert are for.

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
`S_i = find_next(created_at_i)`, then compare `find_next(S_i)` against
`S_{i+1}`. The API returns the resulting counts and decomposition per schedule,
never the raw occurrence rows.

### Detection is cheap, counting is not

Measured on croner 2.2.0, release build: parsing an expression costs ~0.9 µs,
`find_next` 165–330 ns for ordinary expressions and 3.3 µs for the worst one
tried (`0 0 3 29 2 *`), and walking 1000 consecutive occurrences 150–650 µs
(5.7 ms for that same worst case).

Deciding *whether* a run skipped anything is one `find_next` per gap. Across a
full page — `list_with_jobs` returns up to 1000 schedules with 20 occurrences
each — that is roughly 39 000 calls, about 10 ms, hard bounded and independent
of how badly the schedules are behaving.

Counting *how many* were skipped needs a walk, and 19 000 gaps of up to 1000
steps each runs into seconds. So the two surfaces ask different questions. The
list counts runs that skipped something (a boolean per gap). The editor counts
occurrences, for one schedule at a time (at most 20 walks, ~6 ms worst case).
Neither needs a step budget or a degradation heuristic.

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
  This changes the existing `list_schedule_with_jobs` ordering, and with it the
  order of the bars on the schedules row for overlapping schedules. The index it
  needs, `ix_job_root_job_index_by_path_2`, already exists.
- **Deliberate non-runs do not manufacture phantom skips.** `no_flow_overlap`
  returns `PushNextFlowJob::Done` with `stop_early_override: Some(true)`
  (`windmill-worker/src/worker_flow.rs:3311`), so the occurrence's job row exists
  and completes normally; `dynamic_skip` completes its occurrence as
  `status = 'skipped'`. Reconstruction reads both like any other row.
- **A flow schedule holds two or more concurrent root rows in `v2_job_queue`**,
  since the successor is pushed at step 0 entry while its predecessor runs.
  Measured. The live join must aggregate per schedule rather than assume one row.
- **The counting walk is capped at 1000 occurrences per gap**, displayed as
  `1000+`, and only ever runs in the editor. Uses the existing bounded
  `ScheduleType::upcoming` primitive (`windmill-common/src/utils.rs:1082`).
- **Clock-skew clamp** (`now_cutoff + 1s`) makes the reconstruction wrong in the
  backwards-clock case. Rare, already logged at ERROR, accepted.

## Scope by runnable kind

The overrun signal (badge and alert) applies to schedules whose occurrences are
**serialized**, which is `NOT is_flow AND retry IS NULL AND dynamic_skip IS NULL`
rather than simply "scripts".

A script schedule carrying `retry` or `dynamic_skip` is pushed as
`JobPayload::SingleStepFlow` (`windmill-queue/src/schedule.rs:369` and `:245`),
and `JobKind::SingleStepFlow.is_flow()` is true
(`windmill-types/src/jobs.rs:209`). The completion-time re-arm at
`windmill-queue/src/jobs.rs:1379` is therefore skipped for it and it re-arms at
step 0 entry like a flow, so its occurrences overlap.

For every overlapping kind, a run longer than the interval has its successor
already queued and starting on time, so alerting there would fire constantly on
healthy schedules. Occurrences those schedules lose to **late starts** are still
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
- No alert configuration, per-schedule or instance-wide. The predicate compares
  against each schedule's own next occurrence, so it is self-calibrating across a
  daily and a per-minute schedule and needs no thresholds; the only knob a
  setting would add is on/off, which the critical alert channel already has.
- No catch-up or overlap policy. Making the overrun behaviour configurable is a
  much larger change to the execution model, and it needs this diagnosis first
  to know which way people would want it to go.

## Delivery

All three pieces ship together. The first two share the same API response and the
same indicator on the schedules row, so splitting them would mean writing that
code once and rewriting it straight after.

The Enterprise half cannot join them: `*_ee.rs` files are symlinks into
`windmill-ee-private`, so the alert needs a companion PR there and an
`ee-repo-ref.txt` bump, with only the monitor wiring on this side. That is a
repository boundary, not a staging decision.

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
