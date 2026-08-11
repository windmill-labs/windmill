# AI agent evals

A reusable AI agent (`docs/reusable-ai-agents.md`) can be run on its own, against a curated set
of **cases** — the inputs it is expected to keep handling. One drawer does both: the case is
either typed inline or drawn from a **dataset**, and **Save to dataset** is the bridge between
them.

It is reachable from everywhere an agent already is: the AI agent step in the flow editor, the
`ai_agent` row on `/resources`, and an AI agent run's detail page. Each of those knows which
agent it is about, so the drawer opens with the subject already set; the picker in its header
exists for pointing the same dataset at a different agent.

Scoring, experiment runs and comparison are not part of this. A run produces a job you read.

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

`subject.version` is the `resource_version` id the agent resolved to when the run started. It is
recorded, never used to pin execution: a linked agent step resolves its resource live, and that
stays true. Without the record, a run from last week could not be attributed to a prompt state,
which is the reason resource versioning landed before this.

A version captures the resource, not its transitive closure. Two byte-identical versions can
behave differently because a `$var:`/`$res:` they reference changed underneath them, so a
recorded version is necessary for attribution but not sufficient.

## Storage

Datasets live in the workspace object storage, which must be configured before a dataset can be
created. Two objects per dataset:

```
wmill_eval_datasets/meta/<path>.json     the dataset's own metadata
wmill_eval_datasets/cases/<path>.jsonl   one JSON case per line
```

They are split so listing datasets never downloads case bulk.

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

### Permissions

Object storage has no per-object ACL, so a dataset's permissions are the permissions of the
Windmill path it is named by, enforced in the handlers: reading needs the folder (or `u/<self>`,
or admin), writing needs ownership of the path. There is no per-dataset `extra_perms`, and anyone
who can read the workspace bucket directly can read every dataset — as with any other workspace
file.

## Capturing a case from real traffic

Manufactured cases miss the edge cases that actually break agents, so both capture paths build a
draft for review rather than writing anything:

- **From an AI agent run** — the job's `user_message`/`user_attachments`, plus the host flow and
  the `tool_inputs` that run actually used, lifted from the parent flow's step.
- **From a flow conversation** — the case re-asks the conversation's *last user turn*, with
  everything before it replayed as the agent's memory and whatever the agent answered after it
  kept as `expected`. Splitting there rather than at the end is what makes a finished
  conversation — which ends on the assistant — yield a runnable case. Tool messages are left out:
  their content is keyed to call ids this replay will not reissue. `expected` has no consumer
  yet; it is recorded now because it is only available at capture time.
