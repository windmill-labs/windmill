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

- **From an AI agent run** — the job's `user_message`/`user_attachments`, the host flow and the
  `tool_inputs` that run actually used (lifted from the parent flow's step), and what the run
  answered as `expected`.
- **From a flow conversation** — the case re-asks the conversation's *last user turn*, with
  everything before it replayed as the agent's memory and whatever the agent answered after it
  kept as `expected`. Splitting there rather than at the end is what makes a finished
  conversation — which ends on the assistant — yield a runnable case. Tool messages are left out:
  their content is keyed to call ids this replay will not reissue.

`expected` is what a scorer compares a rerun against. Capture is the moment it exists for free,
but it is an ordinary field: it can be typed as plain text, or as JSON when the answer has
structure.

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

Datasets, cases and experiments are rows:

| table | holds |
|---|---|
| `eval_dataset` | one dataset, addressed by a workspace path |
| `eval_case` | one case: its inputs, the answer it was expected to produce, where it was captured from |
| `eval_experiment` | one run of a dataset, against one subject with one set of scorers |
| `eval_experiment_case` | the case set that run executed, and the job each case became |

An experiment records its cases by value instead of pointing at `eval_case`, because a dataset
keeps changing and a result set that cannot say which inputs produced it is not reproducible. For
the same reason `case_id` is a plain column rather than a foreign key: deleting a case must not
rewrite the history of the runs that used it.

Deleting a dataset takes its cases, its experiments and their recorded case sets with it through
the foreign keys. The jobs those experiments produced are left alone — they are jobs, with their
own retention, and a run that happened is not undone by curating the dataset away.

A case is text: a message, an expected answer, at most a short replayed conversation. Attachments
are S3 references rather than inline bytes, so nothing in a case is meant to be large, and two
caps keep it that way — 256 KiB per case and 10 000 cases per dataset, both refused at the API
rather than truncated.

### Permissions

A dataset is permissioned like any other path-addressed object: row-level security on
`eval_dataset` decides who may see it (readers of its folder, `u/<self>`, a group, or an
`extra_perms` grant) and who may change it. Operators cannot write at all. Recording an experiment
counts as a write, since it persists into the dataset.

Cases and experiments are the contents of a dataset rather than objects in their own right. They
carry a read policy derived from their dataset and no write policy at all: the API writes them on
the unrestricted pool, after asking the dataset row itself whether this caller may write it with
`SELECT … FOR UPDATE`, which applies the dataset's UPDATE policies as well as its SELECT policies.
The rule therefore lives in one place instead of being mirrored in Rust, where it could drift. A
stray write to those tables through `user_db` fails rather than silently succeeding.

### Why an experiment is recorded before it is launched

Launching picks every job's id up front, writes the experiment and its case set in one
transaction, and only then pushes the jobs. The order is the point. Pushing first and recording
afterwards leaves a window in which jobs are running that no experiment accounts for, that nothing
will collect and that a retry would silently duplicate. In this order, a launch that dies partway
leaves a recorded case whose job is missing — which the results table shows — and if a push fails
midway the cases that never reached the queue are removed again, so an experiment holds exactly
what ran.

The dataset's foreign key is what makes a concurrent delete safe: the transaction fails, and at
that point no job has been queued.
