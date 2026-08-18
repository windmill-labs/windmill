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
gets a tab rather than a drawer because it is where you sit while changing a prompt.

**Evals belong to a saved agent.** A dataset and its runs hang off an `ai_agent` resource, so they
outlive the step being renamed, copied or deleted, and two runs are comparable because they name
the same thing. A step whose agent is written inline has nothing to hang them on: the tab says so
and points at **Save as reusable agent**, which is one click and the only setup evals ask for.

## What runs

A case is executed as one job: a one-step flow whose single module is an `aiagent` step, pushed as
a `RawFlow` preview. That is the same vehicle `ModuleTest.svelte` uses to test an agent step, so a
case exercises the production branch of `ai_executor.rs` rather than a parallel one — a playground
that behaves differently from production would be worse than none.

The step is built one of two ways, and both name the same agent:

- **The deployed agent** (`subject.kind = "agent"`) is named by path, and the step resolves it
  live, as the linked branch does in production.
- **Its draft** (`subject.kind = "agent_draft"`) is the same agent with the edits waiting on it.
  The draft's value is read server-side and inlined as an unlinked step, because a reference
  resolves live and would otherwise run what the edits replace.

The configuration is never taken from the request. A subject carrying one is refused: it would run
something other than the resource it names, and a run nobody can attribute is worse than no run.

Scoring is not part of this flow. A case job produces an answer and stores it, and scoring reads
that answer afterwards — which is what lets a scorer added next week score a run from today
without calling the agent again.

A draft is the transforms as authored, expressions included. One that reads `results.<step>.x` or
a `flow_input` the case does not supply resolves to nothing here, the same way it would in any run
of that step outside its flow.

One case field exists because a linked agent is not self-contained: **host flow**. A linked
agent's tools bind to their host flow through that flow's `tool_inputs`. A case defaults to the
agent's own authored defaults; naming a host flow resolves that flow's overrides at run time
instead, for when someone hits the discrepancy.

A case carries **no conversation**. It is one question and the answer it should produce, so a run
starts from the agent's own memory configuration and nothing is replayed into it. Replaying a
recorded conversation is a real thing to want — it is how a case captured from production
reproduces the state its answer depended on — but it needs a message list in the case, an override
of the agent's memory in the executor, and an editor for both, and none of that is worth carrying
until someone is curating cases from chat traffic.

## Where results live

Results are jobs. A run's output, logs, trajectory, tool-call child jobs, permissions and
retention are already `v2_job` / `v2_job_completed` and the flow status's `agent_actions`. Nothing
is stored a second time.

The pane shows the **answer** and nothing else of a job. A trajectory is what the run page renders
in full, and a second copy of it in a side panel is a worse version of a page that already exists;
`job_id` is the way there.

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

Manufactured cases miss the edge cases that actually break agents, so capture builds a draft for
review rather than writing anything. **From an AI agent run**: the job's
`user_message`/`user_attachments`, the host flow and the `tool_inputs` that run actually used
(lifted from the parent flow's step), and what the run answered as `expected`. That is the only
capture path: a test of the step is not one, because a step test is a thing you do while writing the
step rather than a run worth keeping.

`expected` is what a scorer compares a rerun against. Capture is the moment it exists for free,
but it is an ordinary field: it can be typed as plain text, or as JSON when the answer has
structure.

## Experiments

Running a dataset produces an experiment: every case executed against one subject, with a row per
case. The experiment records the **exact case set it ran**, by value — a dataset keeps changing,
and a result set that cannot say which inputs produced it is not reproducible.

Each case runs as its own job rather than the whole dataset running as one loop. That keeps a
case's run stamp and history query identical to a single run, and lets an answer be read back by
node id (`get_result_and_success_by_id_from_flow`) instead of walking a nested
loop's status.

An experiment applies one tool binding to every case: `host_flow_path` is set per experiment, and
a case's own `host_flow_path` is honoured only by a single run. Rows of one experiment would not
otherwise be comparable with each other.

### What a run is called

An experiment is `Run N`: `run_number` is allocated per `(dataset, agent path)` when the run is
opened, once, and never reused. Stored rather than counted at read time, so a run keeps the name it
was given as history is pruned around it. That is the whole of naming a run — there is no
user-given label, and `eval_experiment.label` is the column a future one would fill.

Numbering is per agent and not per subject kind, so runs of what is deployed and runs of the edits
on top of it share one sequence. They are the same agent's history, and "Run 7" should mean one
thing; which of the two ran it is what the run says beside its number.

### A run is permanent, and one case is a trial

Every run is written once and then only ever read. Nothing is written over: there is no writable
experiment, no partial rerun, and no cell that can be edited after the fact.

**Running one case records nothing.** It is a trial: you run it to see what the agent does with
that case, and looking at that is not a claim that it belongs in the dataset's history. Its answer
and its scores show in the case panel, beside the recorded result they leave alone — the table goes
on showing exactly what the selected run recorded, which is the point of a run being permanent.
Running the dataset is what produces numbers worth comparing, and a run in which some cells came
from one version and some from another would not be one of them.

A trial is scored where it stands. A run whose numbers you cannot see is a run you have to eyeball,
which is the thing scorers exist to replace, and the trial's scoring reads the answer it already
stored — no second agent call. Leaving the pane loses the trial, not the job: that is a job like
any other, with its own retention and its own `_eval` stamp saying which agent, version and case it
ran.

**Running the dataset** is the only way a run appears, and every cell in it is its own.

- **One experiment holds one subject, and one agent keeps one history.** The experiment list is
  filtered to the agent the pane was opened on, across both kinds, so a dataset shared by two
  agents never shows one agent's runs when the other is opened, and comparing a draft run against
  the deployed run before it is the comparison the pane is for.
- **A version is per cell** (`eval_experiment_case.subject_version`), not per experiment, because
  the agent resolves live and a dataset that takes minutes to run can span an edit. What is
  *recorded* is uniform, though: the subject is resolved once when the run is opened and every cell
  is stamped from it.

The table watches the agent while it is open — a small `subject_state` read every few seconds,
paused while the tab is hidden and repeated when it regains focus. The results endpoint reports the
same version and hash, but it harvests scores and reads every job to do it, so it is not something
to poll; without the watch, an agent edited while the table is open goes on looking current until
the pane is reopened, which is exactly when it is most misleading.

- **A draft is dated by its hash**, not by a version, because editing a draft moves nothing a
  version could record. Each cell carries `subject_draft_hash`, the hash of the configuration it
  ran (canonicalised: key order is not meaningful and `serde_json` preserves insertion order).

**One case is called stale**, and it is the one a label cannot express: the run reads `v23 + edits`,
the agent is *still* on v23 with edits waiting, and they are not the same edits. All four conditions
hold together — the run executed a draft, that draft is not what is deployed now, its version is the
version the agent is still on, and a draft exists now holding something else.

Everything else is legible without a warning. A run of an older version is history and says so
(`Run 14 · v23` beside an agent on v24), and flagging it would flag every past run the moment
anything is deployed. A run whose edits were later deployed is a run of that version, and the
results endpoint recognises and restamps it. Only two runs both reading `v23 + edits` can silently
be two different things.

The table says it once, above itself, in one line: *this run executed an earlier state of the draft
on v24*. No dimming, because every cell of a run is stamped from one resolution of the subject and so
the statement is about the run rather than its rows; and no rerun button, because Run all is one row
up.
- **An agent's draft is its own subject.** Its runs are keyed under `agent_draft`, so a number
  produced by unsaved edits is never quietly read as the deployed agent's. An agent with edits
  waiting is tested on the edits, and the toolbar says which of its two states is under test.
  There is no toggle: editing an agent and opening evals means testing the edits, and the deployed
  value's numbers are already in the history from before the editing started.

In the flow editor, editing a linked agent forks the configuration into the step and clears the
link. The edits are mirrored into that agent's own resource draft as they are made, so evals still
name the agent: the step is where you type, and the draft is what runs. Which agent a step is,
whether it is being edited, and whether those edits are saved is a strip **above the step's tabs**,
because it is true of every tab. Inside the step inputs it read as being about the inputs, which is
why the evals table used to repeat it in an alert of its own. While an agent is being edited the
strip says on the line — not under an icon — that saving updates every flow using the agent, and
carries an unsaved-changes badge once there is something to save.

The strip also names the version, because that is what a run is recorded against: `v24` beside the
agent, and `v24` beside an unsaved-changes badge for edits sitting on top of it, which is what a run
labelled `v24 + edits` executed. It comes from the resource's newest history entry, since the
resource itself does not carry its version.

Nothing is saved by hand. The expensive artifact is the job, which `v2_job` already stores,
permissions and retains; what a run adds is rows of scores pointing at jobs.

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

### Measuring a run again

A run is permanent, so a scorer edited or added after it cannot be applied to it: there is no cell
to overwrite. **Run scorers only** opens a run of its own that reuses that run's answers, and scores
them with the scorers as they are now. It calls the agent for nothing.

What makes the result readable is that the answers and their provenance are copied **whole** rather
than mixed: every cell keeps the parent's `job_id`, `subject_version` and draft hash, so the new run
is attributed to the version that produced those answers, goes stale against the current agent
exactly as its parent does, and tells you nothing about the agent as it is now. `scored_from` records
where the answers came from, and the run picker says so, because a run that reads like any other
would be read as the agent having answered again.

It is deliberately all of the dataset's scorers rather than the one you edited. A run holding one
column could not be compared with any other run on the rest of them, and re-running a deterministic
scorer only reproduces its number.

There is no other way to score an existing run, on purpose. Rescoring a run in place would make a
permanent run editable, and an action in a column's menu that scored *other* runs read as acting on
the one on screen. Trying an edited scorer on a single case is a click away in the panel, which runs
that case against the current agent and scores the answer.

**Not built, and worth building:** a cache. If a cell's agent configuration, case input and scorer
definition are all unchanged, its number could be reused instead of recomputed, which would make
running a dataset cheap and make measuring-again unnecessary. It is deliberately absent rather than
deferred by omission: reusing a cell asserts the agent is deterministic, and it is not, so it has to
be an explicit choice with its own answer to what a run means when half of it was computed last
week.

### A score is a number, and optionally a line through it

Every scorer returns a number. Pass or fail is not a second kind of score: a column carries an
optional `pass_if`, and a case scoring at or above it counts as a pass. A boolean scorer is one
that returns 0 or 1 with the line at 0.5.

One primitive, two readings. A column with a threshold reports a **pass rate** beside its mean and
marks each cell, because "how many cases are good enough" is the question most datasets are
actually asking; the mean stays, because "by how much" is the one it does not answer. A column
without a threshold is a plain number and is not dressed up as a verdict.

The line is deliberately outside the score's **definition** hash: where it sits is an
interpretation of a score rather than part of producing it. Moving it re-reads every run already
recorded — every pass rate in the history changes at once, with nothing re-run and no model call.
It can be set when the column is added and changed later from the column header.

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
editor, since writing the assertions is the work. Both are named by a summary of what they score,
which becomes the column header and, prefixed with the dataset, the path; both take a path you can
change. That is what
keeps the choice between them about how you want to score rather than about setup cost.

Editing a column is editing the runnable it points at, so the column menu opens it in place: a
script in the script editor drawer, a judge in the resource editor.

A `reason` is worth returning: it is what the cell shows on hover, together with the per-assertion
`checks`, so a number that looks wrong can be read rather than re-derived from the trajectory.

Rescoring is offered at two grains, and both of them are repair rather than editing. A column
rescores whole from its header, which is what a judge edit calls for; a cell that **failed** to
score offers to try again from its own hover, because a judge that returned nothing readable or a
scorer that threw should not cost a rerun of the dataset. A cell that scored is not re-scorable on
its own: a run whose numbers can be replaced one at a time until they read well is not evidence.

A scorer may return a bare number, a boolean, or `{score, reason, checks}`; a judge's answer arrives
under `output`, sometimes as a string holding one of those, and often as a markdown code fence
around it — a model told to reply with JSON only does that often enough that refusing to read it
would turn good verdicts into missing ones. Anything with no number in it is left
empty rather than guessed at, and means skip the empty ones — a missing score counted as zero would
read as a regression.

### What a run says it ran

A run is `v15` when it ran the deployed agent and `v15 + edits` when it ran that version with
undeployed changes on top. Spelled out rather than marked with a sigil: a star beside a version is
a legend nobody has, and the two are different enough to be worth two words.

An `agent_draft` run records the version it is an edit of, because "the draft" is not attributable
without saying which deployed state it is a draft of. It also stops being a draft by itself: the
agent is hashed as deployed, in the same shape a draft is hashed in, so a run whose configuration
was later saved is recognised as the version it became. Edit, run, deploy, and the run you made
reads as `v16` rather than staying an edit of `v15` forever.

That recognition is **written, not derived**. When the hashes match, the run's subject is rewritten
to `agent` at that version, once, keeping the hash it is founded on. Deriving it on every read
would make the answer expire: it would only ever mean "this ran what is deployed right now", so the
next deployment would send a run that already read `v16` back to `v15 + edits`, and a label that
moves backwards is worse than one that never moved. The write goes to the unrestricted pool
alongside the scores harvested in the same read, and nothing in it comes from the caller — the hash
is the proof, and a run of a configuration that was never deployed simply stays an edit.

It follows that the resolution needs someone to look: a run is stamped by the first results read
after its configuration is deployed. A run whose configuration was deployed and then replaced
without anyone opening the table keeps saying `+ edits`, which is the honest answer when the only
evidence is a hash that matches nothing deployed.

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

A scorer is stored on the dataset as `{id, name?, pass_if?, kind, path}`, with the `id` assigned
once and never reused. That id is what
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

Opening evals selects the dataset this agent was last worked in, remembered per agent in
`localStorage` and only restored while it still exists and is still readable. Nothing else is
chosen for you: opening on whichever dataset happens to sort first reads as this agent's history
when it is not. The picker lists this agent's own datasets first and everyone else's below, sorted
rather than filtered — running one dataset against a second agent is the comparison the picker
exists for, and hiding the others is how nobody finds it.

A dataset is created before it holds anything, and named the way a script is: you write a
**summary** of what the cases are for, and the path follows from it, prefixed with the agent so it
sorts with the agent's own. Both stay editable, and with no summary the fallback is
`<agent>_dataset1`, taking the next free number. One path segment rather than a folder under the
agent, because a Windmill path is `<kind>/<owner>/<name>` and the picker that edits it cannot
express a deeper one.

Creating one and editing one are the same two fields, so they are the same drawer, reached from the
dataset picker. A drawer rather than a form in the table: inline, creating pushed the table down and
editing replaced it, and neither reads as the small edit it is. Renaming moves the dataset, and its
cases and its runs follow through the foreign keys.

A case has no save button, and nothing reports that it saved: the panel edits the row, edits are
written on a short debounce, and the row following what you type is the confirmation. A write that
fails says so in a toast. Running it stays a separate decision, taken with the case open, which is
why the row carries no Run button either — only Delete, which asks first. **Add a case** adds an empty row and opens it, and nothing is added for you: a
dataset whose first row is a case nobody wrote starts with noise in it.

A case is its message, what it expects, and nothing else. It carries no name: the message is what
identifies it in a table read left to right.

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
| `eval_experiment_case` | the case set it executed, the job each case became, and the version or draft hash each ran against |
| `eval_score` | one scorer's verdict on one run, with the definition that produced it |

An experiment records its cases by value instead of pointing at `eval_case`, because a dataset
keeps changing and a result set that cannot say which inputs produced it is not reproducible. For
the same reason `case_id` is a plain column rather than a foreign key: deleting a case must not
rewrite the history of the runs that used it.

Deleting a dataset takes its cases, its experiments, their recorded case sets and every score with
it through the foreign keys. The jobs those experiments produced are left alone — they are jobs, with their
own retention, and a run that happened is not undone by curating the dataset away.

A case is text: a message and an expected answer. Attachments are S3 references rather than inline
bytes, so nothing in a case is meant to be large, and two
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
