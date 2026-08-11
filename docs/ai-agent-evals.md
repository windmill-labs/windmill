# AI agent evals

A reusable AI agent (`docs/reusable-ai-agents.md`) can be run on its own, against a curated set
of **cases** — the inputs it is expected to keep handling. One drawer does both: the case is
either typed inline or drawn from a **dataset**, and **Save to dataset** is the bridge between
them.

It is reachable from everywhere an agent already is: the AI agent step in the flow editor, the
`ai_agent` row on `/resources`, and an AI agent run's detail page. Each of those knows which
agent it is about, so the drawer opens with the subject already set; the picker in its header
exists for pointing the same dataset at a different agent.

Running the whole dataset is an **experiment**: every case against one subject, scored, in a
table.

## What runs

A case is executed as one job: a one-step flow whose single module is the `aiagent` step linked
to the subject, pushed as a `RawFlow` preview. That is the same vehicle `ModuleTest.svelte` uses
to test an agent step, so a case exercises the production linked-agent branch of
`ai_executor.rs` rather than a parallel one — a playground that behaves differently from
production would be worse than none.

Two case fields exist because a linked agent is not self-contained:

- **Host flow.** A linked agent's tools bind to their host flow through that flow's
  `tool_inputs`. A case defaults to the agent's own authored defaults; naming a host flow
  resolves that flow's overrides at run time instead, for when someone hits the discrepancy.
- **Prior conversation.** Supplying a message list runs the agent over exactly those turns via
  `Memory::Manual` (`windmill-ai/src/types.rs`), which bypasses stored memory. That is what makes
  replaying a recorded conversation deterministic without writing into the memory a production
  conversation is using. A step-supplied `memory` therefore overrides the agent's own, and is
  read from the job's raw args rather than the interpolated ones — a recorded message is
  arbitrary user text, and interpolating it would resolve a `$var:` someone typed into a chat.

## Where results live

Results are jobs. A run's output, logs, trajectory, tool-call child jobs, permissions and
retention are already `v2_job` / `v2_job_completed` and the flow status's `agent_actions` — the
same rows `AIAgentLogViewer` renders. Nothing is stored a second time.

What makes a job findable again is stamped on it at push:

- `runnable_path` is `<agent>/<dataset>/<case>` (just `<agent>` for an unsaved case), so the
  existing `script_path_start` / `script_path_exact` job filters answer "every run of this agent"
  and "every run of this case" with no new state. `v2_job.runnable_path` is `varchar(255)`, so a
  stamp that would overflow degrades to the agent path alone.
- `_eval` in the flow's args records `{subject: {kind, path, version}, dataset, case_id}`, so a
  job opened cold from the runs page explains itself, and an over-long path still carries the
  association. Extra flow inputs are inert — the module reads only
  `user_message`/`user_attachments`.

## Versioning

`subject.version` is the `resource_version` id the agent was at when the run was **enqueued**. It
is recorded, never used to pin execution: a linked agent step resolves its resource when it runs,
and that stays true. Without the record, a run from last week could not be attributed to a prompt
state, which is the reason resource versioning landed before this.

Because resolution is live, the stamp is the version at enqueue rather than the one that
executed: a run that waits in the queue while the agent is edited runs the newer value and is
recorded against the older id. Closing that gap means either pinning execution to a version or
having the executor report the version it resolved, both of which belong with the experiment
work.

A version captures the resource, not its transitive closure. Two byte-identical versions can
behave differently because a `$var:`/`$res:` they reference changed underneath them, so a
recorded version is necessary for attribution but not sufficient.

## Capturing a case from real traffic

Manufactured cases miss the edge cases that actually break agents, so both capture paths build a
draft for review rather than writing anything:

- **From an AI agent run** — the job's `user_message`/`user_attachments`, plus the host flow and
  the `tool_inputs` that run actually used, lifted from the parent flow's step.
- **From a flow conversation** — the case re-asks the conversation's *last user turn*, with
  everything before it replayed as the agent's memory and whatever the agent answered after it
  kept as `expected`. Splitting there rather than at the end is what makes a finished
  conversation — which ends on the assistant — yield a runnable case. Tool messages are left out:
  their content is keyed to call ids this replay will not reissue. `expected` is what a scorer
  compares a rerun against, and capture time is the only moment it exists.

## Experiments

Running a dataset produces an experiment: every case executed against one subject, with a row per
case. The experiment records the **exact case set it ran**, by value — a dataset keeps changing,
and a result set that cannot say which inputs produced it is not reproducible.

Each case runs as its own job — a small flow of the agent followed by a step per scorer — rather
than the whole dataset running as one loop. That keeps a case's run stamp, history query and
trajectory view identical to a single run, and lets results be read back per step by node id
(`get_result_and_success_by_id_from_flow`) instead of walking a nested loop's status.

An experiment applies one tool binding to every case: `host_flow_path` is set per experiment, and
a case's own `host_flow_path` is honoured only by a single run. Rows of one experiment would not
otherwise be comparable with each other.

### Comparison

One experiment's numbers say little; the delta against the run before a change says whether the
change helped. Picking a baseline experiment adds a per-scorer delta to each row and to the mean,
and a filter down to the rows that regressed.

Rows are joined by case id, so a case added after the baseline ran simply has no delta rather
than counting as a change. Deltas need a number on both sides, and the means skip cases a scorer
produced no number for — counting a missing score as zero would read as a regression.

### Scorers

A scorer is any runnable taking `(input, output, expected)` and returning a number — a bare
number, a boolean, or an object with a `score`. Deterministic built-ins are hub scripts, and
LLM-as-judge is a reusable agent used as a scorer, so scoring needs no engine of its own.

A judge agent is prompted with the case and the answer as one JSON message and its `output` is
parsed for a number; a script or flow receives them as named arguments. Anything a scorer returns
that holds no number is left empty rather than guessed at, and averages skip the empty ones — a
missing score counted as zero would read as a regression.

## Storage

Datasets live in the workspace object storage, which must be configured before a dataset can be
created. Per dataset:

```
wmill_eval_datasets/meta/<path>.json                    the dataset's own metadata
wmill_eval_datasets/cases/<path>.jsonl                  one JSON case per line
wmill_eval_datasets/experiments/<path>/<id>.json        one run of the dataset
```

Metadata is split from the case bulk so listing datasets never downloads cases.

### Why case writes take a lock

Adding, editing or deleting a case is a read-modify-write of the whole JSONL, so it runs inside a
transaction holding
`pg_try_advisory_xact_lock(hashtext('ai_eval_dataset:' || workspace || '/' || path))`. Advisory
locks are database-global, so this serializes writers across every API server. Without it, two
cases captured from production runs at the same moment both read the same file and the second PUT
silently drops the first — which is precisely the case you most wanted to keep.

The lock is `try` with a bounded retry rather than the blocking form, because it is held across
two object-store round-trips: a hung storage call must fail that one dataset's request instead of
parking connections behind it.

Optimistic concurrency (conditional PUT with `If-Match`) would avoid the database entirely, but a
store that ignores the header does not error — it silently drops the guarantee, and workspace
storage can be S3, Azure Blob, GCS or any S3-compatible store.

The lock only covers writers that go through the API. Workspace object storage is directly
reachable from scripts through the S3 helpers, so a script appending to the JSONL itself bypasses
it.

That same reachability is why an experiment's `job_id`s are not trusted. Results are read on the
unrestricted pool, so a forged experiment object naming somebody else's flow job would otherwise
hand back output the jobs API would refuse. Only jobs this server stamped with that experiment's
id (in `_eval`) are read; anything else is reported as though it had not run.

### Permissions

Object storage has no per-object ACL, so a dataset's permissions are the permissions of the
Windmill path it is named by, enforced in the handlers: reading needs read on the folder (or
`u/<self>`, or admin), writing needs write on it, and operators cannot write at all. Recording an
experiment counts as a write — it persists into the dataset's namespace. There is no
per-dataset `extra_perms`, and anyone who can read the workspace bucket directly can read every
dataset — as with any other workspace file.
