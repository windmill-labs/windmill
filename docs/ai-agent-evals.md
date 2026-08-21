# AI agent evals

A reusable AI agent (`docs/reusable-ai-agents.md`) can be run on its own, against a curated set
of **cases** — the inputs it is expected to keep handling.

The surface is two screens deep. It opens on **this agent's runs**, across every dataset it has
been measured on: one row per run, carrying what it scored as a badge per column, because the
question you arrive with is whether the agent got better and that is a comparison down a column.
Opening a run is the second screen — **one table**, a row per case, a column per scorer, the cell
being that scorer's verdict; selecting a row opens its detail beside the table, and there is no
third pane. Editing the dataset a run was of is a drawer over both, not a mode of them.

It is a **dialog**, not a side drawer: what it holds is a screen of its own, read across its full
width, rather than a panel beside the thing you came from.

Three words, and no fourth:

- a **case** is one input the agent should handle, held in a **dataset**;
- a **run** is one case executed once, which is a job;
- an **experiment** is the set of runs over a dataset.

It is reachable from where an agent already is, and always over what you were doing rather
than instead of it: the agent card at the top of an AI agent step's inputs in the flow editor, and
the `ai_agent` row on `/resources`. In the flow editor it is a button on the card rather than a tab
of the step, because what it opens is the agent's history rather than a setting of the step, and
the card is the one thing on screen that is about the agent.

**Evals belong to a saved agent.** A dataset and its runs hang off an `ai_agent` resource, so they
outlive the step being renamed, copied or deleted, and two runs are comparable because they name
the same thing. A step whose agent is written inline has nothing to hang them on, and no card
either: what stands in its place is **Save as reusable agent**, which is one click and the only
setup evals ask for.

## What runs

**A run is one flow**: a loop over the dataset's cases, each iteration answering its case and then
scoring the answer. Pushed as a `RawFlow`, so the agent step is the same vehicle
`ModuleTest.svelte` uses to test an agent step and a case exercises the production branch of
`ai_executor.rs` rather than a parallel one — a playground that behaves differently from production
would be worse than none.

One flow rather than one job per case, because **a run outlives the tab that started it**. A
dataset of two hundred cases against a slow provider takes long enough that nobody watches it, and
only a worker can notice that the last case finished; anything the client would otherwise have had
to do at that point — scoring above all — is a step instead. It also makes the run one thing to
watch, to cancel, and to point a schedule at.

The loop is `parallel` with a bounded `parallelism` and `skip_failures`: a dataset is a burst of
calls to one provider, so answering every case at once is a run that spends its time being
rate-limited, and one case failing is one cell of the run rather than the end of it.

The cases are the loop's **static iterator**, so they live in the flow's value, which is stored
once. Passing them as an argument would put a copy of the whole dataset in every iteration's
arguments.

Whichever state of the agent is chosen — what is deployed, the edits in progress, or a past version
— its configuration is fixed once, when the run is opened, and inlined into the step every case runs. A
linked step would resolve the resource when each case reaches it, so a deploy part-way through a
run would be executed by the cases after it while every row still named the version the run started
against. One run measures one configuration; the cost is that a run does not exercise the linked
branch the production step takes.

The edits and a past version could not be run any other way in the first place: a reference
resolves to what is deployed, which is exactly what neither of them is.

A saved agent's and a past version's configuration are never taken from the request: both are read
from the workspace by the path they name, and a subject carrying one is refused, since it would run
something other than the resource it names. The edits in progress are the one kind the request has
to carry — they exist only in the editor — and the run records what it was handed, inlined into its
flow and hashed, so it is reproducible and attributable to "this version plus these edits"; what the
server cannot assert about them is that they derive from that version.

Each iteration is an agent step, then a **payload step**, then one step per scorer. The payload
step exists because the flow cannot see what it needs to: the agent's own result carries the
answer and every message, but each tool call's arguments, result, status, duration and schema
belong to the job that ran it. The step reads them back through `GET /ai_evals/run_payload`, whose
one argument is the iteration's own job id, and hands the scorers exactly what a scorer receives
anywhere else. A scorer therefore measures the agent's latency and not its own: the payload
reports the *agent step's* duration, never the iteration's.

The edits are the transforms as authored, expressions included. One that reads `results.<step>.x` or
a `flow_input` the case does not supply resolves to nothing here, the same way it would in any run
of that step outside its flow.

A linked agent is not fully self-contained: a host flow can override its tools' inputs through
the step's `tool_inputs`. A run does not reproduce that wiring — the agent runs with its own
authored defaults, the same configuration every flow starts from. An agent whose behaviour depends
on one flow's overrides is measured here without them, which is worth knowing when a score and a
production incident disagree.

A case carries **no conversation**. It is one question and the answer it should produce, so a run
starts from the agent's own memory configuration and nothing is replayed into it. Replaying a
recorded conversation is a real thing to want — it is how a case taken from production would
reproduce the state its answer depended on — but it needs a message list in the case, an override
of the agent's memory in the executor, and an editor for both, and none of that is worth carrying
until someone is curating cases from chat traffic.

## Where results live

Results are jobs. A run's logs, trajectory, tool-call child jobs, permissions and retention are
already `v2_job` / `v2_job_completed` and the flow status's `agent_actions`, and none of that is
stored a second time.

What the table itself is made of is the exception: each cell's answer, its outcome and every
scorer's verdict are copied into the run's own rows the first time they can be read. Jobs have
their own retention, and a recorded run is meant to still read as the run it was long after the
jobs that produced it are gone.

The pane shows the **answer** and nothing else of a job. A trajectory is what the run page renders
in full, and a second copy of it in a side panel is a worse version of a page that already exists;
`job_id` is the way there.

For a recorded row the answer is read off the row rather than out of the job, because `job_id` is
the whole iteration — the agent and then the scorers that measured it — and its result is the last
scorer's verdict, not the answer. The link is still the iteration, which is the run of that case.

What makes a job findable again is stamped on it at push:

- `runnable_path` is the agent's own path, so the existing `script_path_start` job filter
  answers "every run of this agent" with no new state. Which dataset and which experiment are in
  the stamp below rather than the path.
- `_eval` in the flow's args records `{subject: {kind, path, version}, dataset, experiment_id}`,
  and every iteration inherits it, so a job opened cold from the runs page explains itself. Which
  case an iteration ran is in its own `iter.value`, which is also how a cell finds its job again.
  Extra flow inputs are inert — the agent step reads only `user_message`/`user_attachments`.

## Versioning

`subject.version` is the agent's version number: how many times it has been saved. It is counted
per resource rather than read off `resource_version.id`, which is one identity sequence for the
whole table — an agent saved nine times reads v4 … v24 under it, and the gaps count writes in
workspaces the reader cannot see. The id stays how a version is addressed, by the history routes
and by restore; the number is what a version is called, and what runs are named and compared by.

The number is stored on the row rather than counted when read, because both ways of deleting
versions take the oldest: the monitor's trim past `MAX_RESOURCE_VERSIONS`, and clearing a history
down to its current value. Counting the survivors would renumber under either, so a run recorded
against v3 would later name a different version.

For an `agent` run the version names the configuration the run read when it opened, which is the
one every case executes. Without the record, a run from last week could not be attributed to a
prompt state, which is the reason resource versioning landed before this. Pinning an *older*
version is a subject kind of its own — an `agent_version` run says which version to read, where an
`agent` run reads whatever is deployed at the moment it starts.

A version captures the resource, not its transitive closure. Two byte-identical versions can
behave differently because a `$var:`/`$res:` they reference changed underneath them, so a
recorded version is necessary for attribution but not sufficient.

## Experiments

Running a dataset produces an experiment: every case executed against one subject, with a row per
case. The experiment records the **exact case set it ran**, by value — a dataset keeps changing,
and a result set that cannot say which inputs produced it is not reproducible.

The whole dataset runs as **one flow**: a parallel `forloopflow` over the cases, each iteration
answering its case and then scoring the answer, with a final step that records what the run
produced. One job rather than one per case, because a run outlives the tab that started it and only
a worker can notice that the last case finished. Each iteration is still read back by node id
(`get_result_and_success_by_id_from_flow`), so an answer is fetched without walking the loop's
status.

### What a run is called

An experiment is `Run N`: `run_number` is allocated per `(dataset, agent path)` when the run is
opened, once, and never reused. Stored rather than counted at read time, so a run keeps the name it
was given as history is pruned around it. That is the whole of naming a run: there is no
user-given label.

Numbering is per agent and not per subject kind, so runs of what is deployed and runs of the edits
on top of it share one sequence. They are the same agent's history, and "Run 7" should mean one
thing; which of the two ran it is what the run says beside its number.

### A run is permanent

Every run is written once and then only ever read. Nothing is written over: there is no writable
experiment, no partial rerun, and no cell that can be edited after the fact. Running the dataset
is what produces numbers worth comparing — a run in which some cells came from one version and
some from another would not be one of them — and it is the only way a run appears; every cell in
it is its own.

- **One experiment holds one subject, and one agent keeps one history.** The experiment list is
  filtered to the agent the pane was opened on, across both kinds, so a dataset shared by two
  agents never shows one agent's runs when the other is opened, and comparing a draft run against
  the deployed run before it is the comparison the pane is for.
- **A version is per cell** (`eval_experiment_case.subject_version`), not per experiment. The
  subject is resolved once when the run is opened and every cell is stamped from it, so the column
  is uniform today; it is per cell so that a run which one day executes cell by cell can say so
  rather than averaging two versions silently.

The table asks what version the agent is on when it opens and whenever the tab regains focus — a
small `subject_state` read, which is when a save made elsewhere is most likely waiting. It is not
polled: the results endpoint reports the same version, but it is the expensive read — it collects
the run as it goes — so it is polled only while a run is actually in flight, and one pass at a time;
and the edits a run is offered on live in the editor that opened the pane, so there is nothing on
the server to watch for them. An agent saved in another tab while this one stays focused is noticed
on the next focus or the next run, not before.

- **Edits are dated by their hash**, not by a version, because editing moves nothing a version
  could record. Each cell carries `subject_draft_hash`, the hash of the configuration it ran
  (canonicalised: key order is not meaningful and `serde_json` preserves insertion order).

Everything is legible without a warning. A run of an older version is history and says so
(`Run 14 · v23` beside an agent on v24), and flagging it would flag every past run the moment
anything is deployed. A run whose edits were later deployed is a run of that version, and the
results endpoint recognises and restamps it. Two runs both reading `v23 + edits` can be two
different things, and on screen they read the same: the hash each carries is recorded, not shown,
and is what lets a run be recognised as the version it later became. Since the edits live only in
the editor that ran them, there is no "current edits" for the table to compare either against.
- **An agent's unsaved edits are their own subject.** Their runs are keyed under `agent_draft`, so
  a number produced by edits is never quietly read as the deployed agent's. The run dialog offers
  them only when it was opened from the editing card, preselected there because testing the edits
  is why you came; the deployed value and past versions stay one click away on the same toggle,
  and from anywhere else the agent is what is deployed.

In the flow editor, editing a linked agent forks the configuration into the step and clears the
link. The step is the only copy of the edits: nothing is saved anywhere until you decide. Which
agent a step is, whether it is being edited, and whether those edits are saved is a **card at the
top of the step's inputs**, inside the same scroll region as the fields it is about, rather than a
second bar with a scrollbar of its own. Evals open from that card in both of its states: from the
editing card they run the edits as the step holds them when Run is pressed, since the question they
answer is whether the edit is an improvement; from the linked card they run the deployed agent, the
same reading a linked step makes at run time. While an agent is being edited the card says on the
line — not under an icon — that saving updates every flow using the agent, and carries an
unsaved-changes badge once there is something to save; the badge is also the way to the diff.

Three ways out, none of them a draft: **Save changes** writes the edits to the agent and re-links
the step; **Cancel** drops them and re-links, asking first when there is something to drop; the diff
drawer's **Discard changes** is Cancel without the question, from a view of exactly what goes. The
agent's own resource draft — the one the resource editor writes — is untouched by any of this, and
evals never read it. What a fork is an edit *of* is held only by `agentEditStore`, in memory: the
fork itself survives a reload in the flow's own unsaved state, the session does not, so a reloaded
step comes back as a standalone agent with no path to save back to. (Editing a linked agent in its
own editor, rather than in the step, is a follow-up, and persisting that relationship is part of
it; this is the shape until then.)

The editor can unmount mid-session — another node selected, the step's tab switched — so what it
needs to judge the edits travels with the session in `agentEditStore`, not with the component: the
agent as deployed, what was forked into the step, and whether the step has read that fork back.
Without a baseline an editor cannot tell an edit from the deployed value, which is the difference
between Cancel asking and Cancel dropping work unannounced.

The card also names the version, because that is what a run is recorded against: `v24` beside the
agent, and `v24` beside an unsaved-changes badge for edits sitting on top of it, which is what a run
labelled `v24 + edits` executed. It comes from the resource's newest history entry, since the
resource itself does not carry its version.

Nothing is saved by hand. The expensive artifact is the job, which `v2_job` already stores,
permissions and retains; what a run adds is its own rows — one per case, carrying the answer, the
outcome and each scorer's verdict, so the table survives the jobs' retention.

## Scoring

Scoring is not a second act with a button of its own: **Run** produces an answer and then scores
it, because the score is the point of pressing it. Each iteration of the run's flow scores its own
answer as a step, and the numbers are harvested into rows when results are read.

A run's cells are therefore measured by the scorers as they stood when it ran, and never again.
A scorer edited or added afterwards has no cell in the runs that predate it: rescoring a run in
place would make a permanent run editable, and cells replaced one at a time until they read well
are not evidence. The one thing read through the present is the pass line — `pass_if` is applied
when a score is read, so moving it re-reads every run with no model call.

The columns themselves are the dataset's current scorers, so the table stays comparable across the
runs it lists rather than growing a column per run. Removing a scorer therefore takes its column
off the runs already recorded as well: the rows it produced are not deleted, but nothing renders
them, and adding the scorer back mints a new column that fills from the next run on. The removal
asks first, and says that.

**Not built, and worth building:**

- **Rescoring stored answers.** A judge is edited far more often than an agent, and re-running the
  agent to test a judge change mixes two variables and pays the model twice for answers that are
  already stored. The honest shape is a run of its own that reuses a parent run's answers and
  scores them with the columns as they are now — attributed to the version that produced the
  answers, and saying where they came from, so it never reads as the agent having answered again.
- **A cache.** If a cell's agent configuration, case input and scorer definition are all unchanged,
  its number could be reused instead of recomputed, which would make running a dataset cheap. It is
  deliberately absent rather than deferred by omission: reusing a cell asserts the agent is
  deterministic, and it is not, so it has to be an explicit choice with its own answer to what a
  run means when half of it was computed last week.

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
It can be set when the column is added and changed later under **Scorer settings** beside the
column in the dataset drawer, which is also where the column is named.

The name is this dataset's own name for the scorer, seeded from the summary given when it was
added. It is a copy, not a link: the script or judge agent keeps whatever it is called, so renaming
a column here does not rename anything a second dataset shows. Reading it live from the runnable
instead would cost a fetch per column and leave a column blank for anyone who cannot read what it
points at.

### What a scorer receives

An agent is judged on its behaviour, so the final answer is the smaller half of the evidence. Every
scorer — a judge prompt or a script — is handed the same `EvalRun`, built from the job the run
already stored:

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
editor, since writing the assertions is the work. Both are named by a summary of what they score, which becomes the column header and, prefixed with
the dataset, the path; both take a path you can change, and both open with that summary already
filled in — it names the column *and* derives the path, so an empty one is two decisions before the
first score. That is what
keeps the choice between them about how you want to score rather than about setup cost.

Editing a column is editing the runnable it points at, so the dataset drawer opens it in place: a
script in the script editor, a judge in the resource editor.

A `reason` is worth returning: it is what the cell shows on hover, together with the per-assertion
`checks`, so a number that looks wrong can be read rather than re-derived from the trajectory.

A scorer may return a bare number, a boolean, or `{score, reason, checks}`; a judge's answer arrives
under `output`, sometimes as a string holding one of those, and often as a markdown code fence
around it — a model told to reply with JSON only does that often enough that refusing to read it
would turn good verdicts into missing ones. `comment` is read as `reason`, so a scorer written for
another platform keeps its rationale. Anything with no number in it is left
empty rather than guessed at, and means skip the empty ones — a missing score counted as zero would
read as a regression.

`{score: null}` is the one exception, and it means the scorer read the case and had nothing to
measure on it: a column asking whether sources were cited has no verdict on a case with nothing to
cite. The cell shows `n/a` and is left out of the column's mean and pass rate, which is not the
same as the scorer failing — that is an error, and the column reports it as one. Written out
rather than merely absent, since a scorer that returns nothing at all is a scorer that is broken.

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
once and never reused: on a write, an incoming id is kept only when it names a column the dataset
already holds, and anything else is minted, so a column that was removed cannot come back under
its old id and inherit the scores recorded against it. That id is what
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
when it is not, and no run is opened either — which run you want is what the list is there to
answer. The picker lists this agent's own datasets first and everyone else's below, sorted
rather than filtered — running one dataset against a second agent is the comparison the picker
exists for, and hiding the others is how nobody finds it.

A dataset is created before it holds anything, and named the way a script is: you write a
**summary** of what the cases are for, and the path follows from it, prefixed with the agent so it
sorts with the agent's own. Both stay editable, and with no summary the fallback is
`<agent>_dataset1`, taking the next free number. One path segment rather than a folder under the
agent, because a Windmill path is `<kind>/<owner>/<name>` and the picker that edits it cannot
express a deeper one.

### Editing the dataset is a drawer on top of the runs

The table answers what a run measured. What the next run will measure is the dataset, and it is
edited in a drawer over the table: the summary and the path, then the **scorers**, then the cases as
a list beside the editor for the one selected. Columns before cases, because a column applies to
every case and a case is read against every column.

**Every way of managing a scorer is in that drawer** — adding, renaming, moving its pass line,
opening the agent or script behind it, removing it. The column header over a run's table reports;
it does not edit. A run is permanent, so a control on it that changed what the columns are would be
editing the past from the one place that must not.

Adding one offers four entries rather than two: **new** or **existing**, for an **AI judge** or a
**code scorer**. Writing a scorer and picking one that already exists need different fields, and
which of the two you are doing is the first thing the form needs to know. A new one of either kind
opens with its summary already filled in — the summary names the column *and* derives the path, so
an empty one is two decisions before the first score. Creating a dataset is the same drawer with the same two fields and no
cases yet, so there is one place a dataset is written and one shape to learn. It is reached from
the dataset named on a row of the runs list, and from the run dialog — the pencil on the dataset
you are about to run, and **New dataset** beside it — which are the two places a dataset is what
you are looking at. Renaming moves the dataset, and its cases and its runs follow through the
foreign keys. The drawer saves in one request — the rename, the summary and the cases together —
so a rename the server refuses leaves the cases as they were rather than written under the old
name.

Curating and reading are kept apart because they answer different questions and a table that did
both invited editing a case while looking at what an earlier version of it scored. So the row keeps
no Delete — deleting is in the drawer's case list, and it asks first — and a row's panel is
read-only, showing the case *as the run executed it* rather than as the dataset holds it now.

The drawer edits a working copy and writes it when **Save** is pressed: added, changed and
dropped cases land together, so a half-finished edit is never what the next run executes. **Add a
case** puts an empty row in the list and opens it, and nothing is filled in for you: a dataset
whose first case is one nobody wrote starts with noise in it.

A case is its message, what it expects, and nothing else. It carries no name: the message is what
identifies it in a table read left to right. `expected` is what a scorer compares an answer
against, and it is an ordinary field: plain text, or JSON when the answer has structure.

### What the list of runs shows

One row per run of this agent, newest first, whichever dataset it was of: the run's number and what
executed it (`v24`, `v24 + edits`, or a pinned `v18`), how many cases it holds, what it scored, the
dataset, and when.

One list across datasets rather than one per dataset, because "has this agent got better" is not a
question about a dataset. Each row names the dataset it is of — summary first, path under it — and
that name is the way into its cases and columns.

The scores are one badge per scorer rather than one number per run. A dataset with three columns is
three different questions, and a single figure would be their average rather than an answer to any
of them — the same reason the table under a run reports each column separately. Each chip is the
headline that column reports: a pass rate where the column has a line to pass, the mean where it
does not, read through the thresholds as they are **now**, so moving a pass line re-reads every run
already recorded. A column that never scored a run reads `—`; a run still going spins.

Each badge is named and resolved server-side. A list spanning datasets cannot hold every dataset's
scorers to look a column's name up, so the name and the kind ride along with the number, and the
thresholds are joined in per (run, column) — one grouped query over `eval_score` rather than a read
of each run's cells, so a hundred runs is one aggregate. A run whose scores are still in its flow is
read out of it by the list itself — capped per call, newest first, and skipped entirely for runs
already collected, so the steady state is one query and not one flow read per row.

### Running asks what to run

**Run** opens a dialog with two questions, because both answers cost a provider bill and neither
follows from where you happen to be standing:

- **which state of the agent.** `v24 (latest deployed)` is the agent as it is saved when you press
  Run. A past version is what it was then, and `v24 + edits (current)` runs the step's edits as
  they are when you press Run — offered, and preselected, only when the dialog was opened from the
  editing card, because that is why you came. All three are fixed once and run by every case.
- **which dataset**, as a resource picker does it: named by its summary with the path under it, an
  edit button on the row, and a way to start a new one without leaving.

A pinned version reproduces the configuration, not the world around it: `$var:` and `$res:`
references inside it still resolve at run time. Nothing about the agent as it is now can make a run
of `v18` stale, which is the reason to pin one.

The run that was just started opens straight away: it is the one thing on the screen still
changing, and watching it is why you pressed Run.

### What the table lists

The rows are the dataset's cases, in dataset order, each carrying its result in the selected
experiment when it has one. A table built from the experiment alone would leave a dataset that has
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
| `eval_case` | one case: its inputs and the answer it was expected to produce |
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
caps keep it that way — 256 KiB per case and 1 000 cases per dataset, both refused at the API
rather than truncated. A run scores every case by every scorer, so a dataset also holds at most
20 scorers, refused the same way.

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

Launching picks the run job's id up front, writes the experiment, its case set and a pending score
per cell in one transaction, and only then queues the flow. The order is the point. Queueing first
and recording afterwards leaves a window in which a flow is running that no experiment accounts
for, that nothing will collect and that a retry would silently duplicate. In this order, a launch
that dies before the push leaves an experiment naming a job that never started — a run that did
not run — and a push that fails deletes it, because one failed push is the whole run.

The dataset's foreign key is what makes a concurrent delete safe: the transaction fails, and at
that point nothing has been queued.

### How a cell finds its job, and its score

The flow engine mints the iteration job ids, so a case is recorded before it has one. Three things
fill the gap, each copied out of the flow the first time it can be read:

- **Which iteration ran which case.** The case is what the loop iterates over, so it is in the
  iteration's own arguments by construction: `args -> 'iter' -> 'value' ->> 'case_id'` matches the
  cell, whatever order the iterations finish in.
- **What the agent answered.** The agent step's result and outcome, copied onto the cell as soon
  as that step is done — which is well before the iteration around it, since the scorers are still
  reading it.
- **What the scorers returned.** Each scorer step's result is read out of the iteration's flow
  status into the pending row that was written for it at launch.

All three are written once, when they first become readable, and every later read is of the rows.
A job that was retained away before anything read it leaves the cell saying so, rather than
looking like a case still being answered.

The flow itself cannot write them: it runs on workers that know nothing about these tables. So two
things call the collector. A run's flow ends with a step that calls `POST
/ai_evals/experiments/collect` on itself, which is what records a run nobody watched finish — the
one started and left behind. Reading a run collects it too, which covers the run whose flow never
reached that step: one cancelled part-way, or started while nothing served the `nativets` tag.

That step is bookkeeping, so it is `continue_on_error`: a run whose every case answered and scored
does not become a failed job because the call did not land.
