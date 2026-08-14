# AI agent evals

A reusable AI agent (`docs/reusable-ai-agents.md`) can be run on its own, against a curated set
of **cases** — the inputs it is expected to keep handling.

The surface is one table. A row is a case, a column is a scorer, and the cell is that scorer's
verdict on the newest run of that case. Selecting a row opens its detail beside the table; there
is no third pane.

Three words, and no fourth:

- a **case** is one input the agent should handle, held in a **dataset**;
- a **run** is one case executed once, which is a job;
- an **experiment** is the set of runs over a dataset.

It is reachable from everywhere an agent already is: the **Evals tab** of the AI agent step in the
flow editor, the `ai_agent` row on `/resources`, and an AI agent run's detail page. The flow editor
gets a tab rather than a drawer because it is where you sit while changing a prompt — and because
a tab can run the step **as authored**, before it has been saved as an agent at all.

## What runs

A case is executed as one job: a one-step flow whose single module is an `aiagent` step, pushed as
a `RawFlow` preview. That is the same vehicle `ModuleTest.svelte` uses to test an agent step, so a
case exercises the production branch of `ai_executor.rs` rather than a parallel one — a playground
that behaves differently from production would be worse than none.

The step is built one of two ways, and this is the whole of what makes a draft testable:

- **A saved agent** (`subject.kind = "agent"`) is named by path, and the step resolves it live, as
  the linked branch does in production.
- **A draft** (`subject.kind = "draft"`) carries the step's own `input_transforms` and `tools` in
  the request, and runs the unlinked branch with them. `subject.path` then names the step being
  edited (`<flow path>/<module id>`) so the run can be found again; nothing resolves it.

Scoring is not part of this flow. A case job produces an answer and stores it, and scoring reads
that answer afterwards — which is what lets a scorer added next week score a run from today
without calling the agent again.

A draft carries the transforms as authored, expressions included. One that reads
`results.<step>.x` or a `flow_input` the case does not supply resolves to nothing here, the same
way it would in any run of that step outside its flow.

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

Each case runs as its own job rather than the whole dataset running as one loop. That keeps a
case's run stamp, history query and trajectory view identical to a single run, and lets an answer
be read back by node id (`get_result_and_success_by_id_from_flow`) instead of walking a nested
loop's status.

An experiment applies one tool binding to every case: `host_flow_path` is set per experiment, and
a case's own `host_flow_path` is honoured only by a single run. Rows of one experiment would not
otherwise be comparable with each other.

### What a run is called

An experiment is `Run N`: `run_number` is allocated per `(dataset, subject kind, subject path)`
when the run is opened, once, and never reused. Stored rather than counted at read time, so a run
keeps the name it was given as history is pruned around it. That is the whole of naming a run —
there is no user-given label, and `eval_experiment.label` is the column a future one would fill.

### A run is permanent, and one case is not a run

Every run is written once and then only ever read. Nothing is ever written over, and there is no
writable experiment.

**Running one case records nothing.** It is a job: you run it to see what the agent does, and
looking at that is not a claim that it belongs in the dataset's history. Its result is held in the
pane and shown over the row it belongs to, and the scorers run on it there — a rerun is worth
looking at only with its numbers beside it, and waiting for a save to see them would mean
deciding blind. Those numbers are starred until they are saved. Saving it makes a run of it — seeded from the run on screen, so the cases that were not rerun are carried into it
(`eval_experiment_case.carried_from`, `eval_experiment.seeded_from`) and the run is whole, with
the cells that were actually rerun starred. Leaving the pane loses the results, not the jobs:
those are jobs like any other.

Nothing about a saved cell is taken from the client. Each case run carries an `_eval` argument
written when it was queued — subject, version or draft hash, dataset, case — and saving reads the
cells back out of the jobs, so a run cannot be filed under an agent or a version it did not run.
The scoring job is carried in the same way rather than run again: a judge asked twice does not
answer twice the same, and the number that is saved has to be the number that was looked at.

**Running the dataset** is the other way a run appears, and it is one immediately: every cell is
its own.

- **One experiment holds one subject, and every subject keeps its own.** A run is seeded from the
  last run of `(dataset, subject kind, subject path)`, and the experiment list is filtered to the
  subject the pane was opened on. A dataset shared by two agents would otherwise show one agent's
  runs when the other is opened, and switching between them would throw away where you were. An
  unsaved step is a subject in its own right, keyed by `<flow path>/<module id>`, so iterating on a
  draft never lands in the saved agent's history.
- **A version is per cell** (`eval_experiment_case.subject_version`), not per experiment: a run
  seeded from one made before an edit holds two versions. The table says so instead of averaging
  them silently — a cell that ran against a version the agent has since moved off is dimmed, and
  the table offers to rerun.
The table watches the agent while it is open — a small `subject_state` read every few seconds,
paused while the tab is hidden and repeated when it regains focus. The results endpoint reports
the same version and hash, but it harvests scores and reads every job to do it, so it is not
something to poll; without the watch, an agent edited while the table is open goes on looking
current until the pane is reopened, which is exactly when it is most misleading.

- **A draft is dated by its hash**, not by a version, because editing a draft moves nothing a
  version could record. Each cell carries `subject_draft_hash`, the hash of the configuration it
  ran (canonicalised: key order is not meaningful and `serde_json` preserves insertion order), and
  a cell whose hash is not what the draft hashes to now is stale in exactly the same way.
- **An agent's draft can be run, and is its own subject.** A reference resolves live, so a linked
  step always executes the deployed value: an eval of an edited agent would test what the edits
  replace. The `agent_draft` subject reads the draft server-side and inlines it as an unlinked
  step — the brain becomes the module's input transforms, the tools its tools — which is the same
  branch a step with no linked agent already takes. Its runs are keyed under `agent_draft`, so
  they never mix with the deployed agent's history, and they carry no version, because a draft is
  the state that has not become one. The table says when an agent has undeployed changes and
  offers to run them.

Nothing is saved by hand. The expensive artifact is the job, which `v2_job` already stores,
permissions and retains; what an experiment adds is rows of scores pointing at jobs. A save button
would ask the user to decide, before seeing the numbers, whether the numbers matter — the one
exception is a partial rerun, which is provisional by construction and offers to be kept.

## Scoring

Scoring is separate from running, and only one of the two needs a button. **Run** produces an
answer and then scores it, because the score is the point of pressing it. **Rescore** re-runs the
scorers over answers already recorded and never touches the agent.

That split is not a convenience. A judge is edited far more often than an agent, and re-running the
agent to test a judge change mixes two variables and pays the model twice for an answer that is
already stored. It is also what makes an honest comparison possible: a baseline that predates a
scorer can be scored *from its stored answers*, so the column stops reading "not scored" without
anyone re-running last week's agent.

`POST /ai_evals/score` is one route at four grains — one run, one column of one experiment, a whole
experiment, or a column across history — and none of them runs on its own. Scoring history is an
action a user takes, never a background job spending a provider budget while a table is open.

### What a scorer receives

An agent is judged on its behaviour, so the final answer is the smaller half of the evidence. Every
scorer — a judge prompt, a script, a flow — is handed the same `EvalRun`, built from the job the
run already stored:

| field | from |
|---|---|
| `input`, `expected` | the case as the experiment recorded it |
| `output` | the agent step's own result |
| `tool_calls` | every message carrying an `agent_action`, in order, with the arguments, result, error and duration of the job that call ran |
| `tools` | the tools that were called, with the schema of the script version that ran |
| `metrics` | `steps`, `duration_ms`, and the provider's `usage` when it reported any |

Tool results are truncated at 4 KiB with `truncated: true`, so a large one cannot swamp a judge's
context, and a check that reads a truncated result can say so rather than failing on the missing
tail. A tool whose schema could not be resolved carries `null`, and a scorer validating arguments
must treat that as unchecked rather than as a failure. There is no cost field: Windmill keeps no
provider price table, and a number that guessed at dollars would be worse than none — the script
template takes a rate as an argument instead.

### The kinds

Two, and both are runnables:

| kind | is | receives |
|---|---|---|
| `agent` | an `ai_agent` resource used as a judge | the run, rendered as a message |
| `script` | a workspace script | `run`, with `input`, `output` and `expected` also spelled out |

Keeping every scorer a runnable is what makes columns comparable: each has a path, a version, and
code you can open. There is no third kind stored as configuration on the dataset — a judge's model
and grading prompt live on the agent resource, so editing a judge is editing that agent, and the
column is not something you edit at all.

Which kind you are adding is chosen before the form opens, because the two share almost nothing: a
judge is created next to the dataset from the model you pick and a grading prompt that starts at the
default and is editable there, and a script is created from the template below and opened in the
editor, since writing the assertions is the work. Both take a path you can change. That is what
keeps the choice between them about how you want to score rather than about setup cost.

Editing a column is editing the runnable it points at, so the column menu opens it in place: a
script in the script editor drawer, a judge in the resource editor.

A `reason` is worth returning: it is what the cell shows on hover, together with the per-assertion
`checks`, so a number that looks wrong can be read rather than re-derived from the trajectory. The
same hover carries **Score again** for that one cell, which costs a scoring call and no agent call
— a judge is not deterministic, and a scorer gets edited.

A scorer may return a bare number, a boolean, or `{score, reason, checks}`; a judge's answer arrives
under `output`, sometimes as a string holding one of those, and often as a markdown code fence
around it — a model told to reply with JSON only does that often enough that refusing to read it
would turn good verdicts into missing ones. Anything with no number in it is left
empty rather than guessed at, and means skip the empty ones — a missing score counted as zero would
read as a regression.

### A step's history survives its agent being saved

A step with no agent of its own is the subject `<flow>/<step>`, and it is evaluated like anything
else — evals do not wait for an agent to have a name. Saving it as an `ai_agent` moves that
history onto the agent: the runs are relabelled `agent_draft` at the new path, which is what they
were, runs of a configuration of this agent that was not deployed at the time. Only the name
changes; each run keeps the hash of what it ran, so the one that ran exactly what was saved reads
as that version and the others stay starred.

An agent's table then shows both kinds of run — the deployed ones and the undeployed ones — because
they are the history of one agent and keeping them apart would only hide the comparison. The
experiment list takes a second subject (`also_subject_*`) for that.

### What a version star means

A run's subject is `v15` when it ran the deployed agent, `v15*` when it ran that version plus
undeployed edits, and `draft*` when there is no version to name it by. A step forked from an agent
for editing carries `origin_path`, so its runs say `v15*` rather than passing for anonymous — in
the flow editor, editing a linked agent clears the link and copies the configuration into the
step, which makes the run genuinely a run of the step and its provenance easy to lose.

The star comes off by itself. Results report what the agent hashes to as deployed, in the same
shape a draft is hashed in, so a run whose configuration was later saved is recognised as that
version: edit, run, save, and the run you made reads as `v16` rather than staying a draft. An `agent_draft` run
records the version it is an edit of for exactly this reason: "the draft" is not attributable
without saying which deployed state it is a draft of.

### Reusing a scorer

A new dataset starts with no columns, and the judge you want is usually one you already wrote.
The add form lists the scorers this workspace already uses, most recently edited dataset first,
and adding one is a click. Nothing new is stored to make that list: it is read out of the
datasets' own `scorers`.

It is filtered twice, both times by what the caller can read. The datasets are read through
`user_db`, so a scorer only appears if the dataset carrying it does; then the runnables
themselves are checked the same way, so a script or agent the caller cannot open never appears
as a suggestion they could add and never be able to score with.

### A scorer is a column

A scorer is stored on the dataset as `{id, name?, kind, path}`, with the `id` assigned once and
never reused. That id is what
makes a column the same column across experiments when the scorer is renamed or its definition
edited, and a delta is only ever computed between two scores carrying the same id. Two scorers
pointing at the same script are two columns, which is what someone comparing thresholds wants.

A score is keyed `(experiment_id, ordinal, scorer_id)`, not baked into the experiment, so a frozen
experiment can gain a score without becoming mutable in any way that matters: what is frozen is
which runs are in it.

Each score also records the **definition** that produced it — the kind, the path, and the script
hash or resource version that actually ran, so a path alone cannot hide an edit. When
two scores of one column carry different definitions the delta is still shown, marked: hiding the
number would force model calls just to see anything, and showing it unmarked would let a change of
judge read as a change of agent.

Scoring jobs are harvested when results are read: a finished job's module result is written into
its score row, so a score outlives the job that produced it and that job's retention.

### No dataset is chosen for you

Opening evals selects the dataset this subject was last worked in, remembered per subject in
`localStorage` and only restored while it still exists and is still readable. Nothing else is
chosen for you: opening on whichever dataset happens to sort first reads as this agent's history
when it is not.

A case has no save button: the panel edits the row, and edits are written on a short debounce, so
a case is what the table shows rather than what someone remembered to save. Running it stays a
separate decision.

There is no create-a-dataset step either: the empty table offers **Add a case**, and the first one
creates the dataset under what is being tested — `<agent>/dataset1`, or `<flow>/dataset1` for a
step with no agent of its own, taking the next free number. The row says which path it is about to
start, because a name chosen for you should not be a surprise. Naming is worth doing once there is
something in it, which is what the rename in the toolbar is for; renaming moves the dataset and its
cases and experiments follow through the foreign keys. Picking an existing dataset from the
toolbar stays the way to work in someone else's.

### What the table lists

The rows are the dataset's cases, in dataset order, each carrying its result in the selected
experiment when it has one. Adding a case writes the row and opens it for editing: a case is a row
of the dataset, and running it is a separate decision that a case does not have to be worth yet. A table built from the experiment alone would leave a dataset that has
never been run looking empty, which is exactly the state in which someone wants to press Run. A case
the experiment ran but the dataset no longer holds keeps its row at the end: the run happened, and
deleting the case does not unmake it.

Each column's mean sits under the column header it is a mean of, with its delta beside it when a
baseline is selected. There is no separate recap: a number away from its column is a number whose
meaning has to be guessed.

### Comparison

One experiment's numbers say little; the delta against the run before a change says whether the
change helped. Picking a baseline adds a per-scorer delta to every cell and to each column's mean,
and counts the cells that regressed.

Every delta names its scorer. Comparison is a mode of the table, not a column of it, and there is
no single number for a dataset: averaging a judge with an exact match would invent one. A scorer
that wants one number can compute it — the script template scores by counting passed checks.

Rows are joined by case id, so a case added after the baseline ran has no delta rather than
counting as a change. A column the baseline was never scored with reports that, and offers to score
it, rather than showing a difference that does not exist.

## Storage

Datasets, cases and experiments are rows:

| table | holds |
|---|---|
| `eval_dataset` | one dataset, addressed by a workspace path, and the scorers that are its columns |
| `eval_case` | one case: its inputs, the answer it was expected to produce, where it was captured from |
| `eval_experiment` | one run over a dataset, against one subject; written once, then only read |
| `eval_experiment_case` | the case set it executed, the job each case became, the version or draft hash each ran against, and which run it was carried from |
| `eval_score` | one scorer's verdict on one run, with the definition that produced it |

An experiment records its cases by value instead of pointing at `eval_case`, because a dataset
keeps changing and a result set that cannot say which inputs produced it is not reproducible. For
the same reason `case_id` is a plain column rather than a foreign key: deleting a case must not
rewrite the history of the runs that used it.

Deleting a dataset takes its cases, its experiments, their recorded case sets and every score with
it through the foreign keys. The jobs those experiments produced are left alone — they are jobs, with their
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
