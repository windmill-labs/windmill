# System Prompt And Skill Output Testing Plan

## Goal

Build a single testing strategy that answers one question reliably:

> Given a user task, how good is the artifact produced by our AI system?

This plan is intentionally focused on **black-box output evaluation**, not on unit testing frontend or CLI internals.

The intended end state is a **new repo-level benchmark CLI** that runs a shared
eval suite across multiple surfaces.

That benchmark CLI should be the main entrypoint for:

- running one case
- running a benchmark set
- comparing baseline vs candidate variants
- writing benchmark history snapshots

Frontend and Windmill CLI are not meant to become separate testing products.
They should be implemented as adapters behind this shared benchmark CLI.

The system under test is:

- Frontend AI Chat in `script`, `flow`, and `app` modes
- CLI local development experience driven by generated guidance and skills

The artifact under test is:

- Script code
- Flow JSON / module structure
- Raw app files and backend runnables
- Files and project artifacts produced in a local CLI workspace

## Non-Goals

This plan does **not** treat the following as the main testing target:

- Unit testing helper functions, stores, or tool wrapper internals
- UI rendering behavior, DOM interactions, or component-level correctness
- `wmill init` correctness as a standalone product area
- Backend route correctness except where it affects prompt delivery or AI configuration

Those may still need lightweight tests, but they are not the core of prompt reliability evaluation.

## Core Principles

### 1. Black-box evaluation only

The runner should provide an input task to the real system setup, let it run, collect the final artifact, and score the result.

In practice, this runner should be exposed through the new repo-level benchmark
CLI rather than through separate ad hoc test commands for each surface.

### 2. Headless execution

Frontend evaluation must be fully decoupled from the browser UI. It should exercise prompt assembly, tool selection, and tool execution logic without mounting Svelte components or clicking through the app.

### 3. Real prompt environment

All evals must use the same prompt-building path, tool definitions, and skill content that production uses, or a clearly defined variant of them.

### 4. Artifact-first scoring

The main score is based on the produced artifact, not on intermediate transcripts.

### 5. Reliability over one-off success

A prompt is not "good" because it passed once. Reliability means pass rate across repeated runs and across a representative case set.

### 6. Track benchmark history over time

The suite must not only evaluate the current output. It must also produce a
git-tracked benchmark history so the team can see whether the system is
improving over time.

This history should focus on official benchmark snapshots, not on every local
experiment.

### 7. Shared corpus, separate adapters

Frontend and CLI should share the same evaluation corpus format when possible, but each surface should have its own execution adapter.

### 8. CLI first, UI last

The CLI should be the first surface brought to a high-confidence benchmark
state.

It is the cleanest foundation for the suite because it produces direct files in
an isolated workspace, has less ambiguity than the frontend, and is easier to
score deterministically.

Frontend should reuse the benchmark model proven on the CLI rather than define
a parallel testing philosophy.

### 9. UI comes last

The testing suite must exist and be trustworthy before building a studio UI on top of it.

## Current State

## Shared Prompt Source Of Truth

The repo already has the right content split:

- `system_prompts/` is the shared source of truth for core Windmill prompt content
- frontend adds chat-specific tool instructions on top
- CLI materializes guidance and skill content from generated outputs

This is a strong foundation for a shared eval suite.

## Execution Priority

Even though the repo already has useful frontend eval scaffolding, the
implementation priority should be:

1. build the repo-level benchmark CLI and use the Windmill CLI adapter as the
   first implementation behind it
2. make the CLI artifact-evaluation path excellent
3. stabilize shared scoring, reporting, and benchmark history around that path
4. bring frontend onto the same benchmark model through the same benchmark CLI
5. build the UI only after the underlying suite is trustworthy

This keeps the hardest product question focused on artifact quality rather than
on UI workflow.

## Benchmark CLI As The Main Product

The testing suite should have one primary interface:

- a new repo-level benchmark CLI

The benchmark CLI should be able to run:

- Windmill CLI evals
- frontend evals
- shared reporting and comparison commands

Illustrative command shape:

```bash
ai-evals run --surface cli --case bun-hello-script
ai-evals run --surface frontend-flow --case support-flow
ai-evals compare --surface cli --variant baseline --variant candidate-a
ai-evals history latest
```

The exact binary name can change, but the architecture should not:

- one benchmark CLI
- shared case loader
- shared scoring
- shared history writer
- separate surface adapters underneath

## Temporary Bootstrap Code

This bootstrap phase is now complete for frontend `flow`, `app`, and `script`.

Frontend AI benchmark ownership has moved into `ai_evals/`, and the frontend
source tree no longer owns a separate AI benchmark suite under
`frontend/.../__tests__/...`.

Benchmark authors should only need the repo-level benchmark CLI to run the
long-term suite.

The only temporary frontend-specific piece that remains is a thin Vitest/Vite
loader bridge so the benchmark runner can import the production chat modules in
the same module/runtime environment they already expect.

## Frontend: What Exists Today

The current frontend benchmark path is **decoupled from the UI** and now owned
by `ai_evals`.

They currently:

- run through the shared headless chat loop
- use production prompt builders
- use production tool definitions
- use benchmark-owned helper adapters that write to temp workspaces on disk
- execute through the frontend module/runtime environment only as a loader bridge

This means the current frontend evals are now a proper benchmark adapter,
not a frontend test suite.

That is the correct direction.

### Frontend Architecture Notes

There are three categories of code involved:

- shared production logic:
  - production system prompt builders
  - production tool definitions
  - production `runChatLoop`
- benchmark-only infrastructure:
  - case loading
  - variant loading
  - judge scoring
  - benchmark result shaping
  - history/reporting integration
- alternate helper adapters:
  - production helpers mutate UI/editor state
  - benchmark helpers mutate temp-workspace files

This is important because the benchmark suite is **not** meant to duplicate the
frontend chat logic. It is meant to reuse the production chat loop and tool
definitions while swapping the execution backend from UI state to filesystem
state.

## Frontend: What Is Missing

### Coverage gaps

- `script` is now exposed through the shared benchmark CLI, but it only has initial case coverage.
- Existing frontend coverage is still too small relative to the target benchmark corpus.

### Reliability gaps

- Frontend flow and app can already run with pass/fail results and repeated runs through the shared benchmark CLI.
- The remaining gap is turning that into stronger routine reliability gating with better deterministic validators and broader routine case coverage.
- Frontend reliability reporting is still less mature than the intended end state for official CI tiers and richer failure triage.

### Prompt-iteration gaps

- Frontend prompt variants are file-backed now, but the repo only ships baseline manifests by default.
- Creating and curating meaningful frontend candidate variants is still a mostly manual workflow compared with the CLI snapshot flow.
- Frontend prompt comparison exists through the shared `compare` command, but it still needs broader routine use and better variant coverage.

### Artifact-validation gaps

- The current flow and app helpers are file-backed now, but several effects are still lightweight and should become more realistic over time.
- Linting and runnable validation are currently too lightweight in the eval path.
- Datatable interactions are mocked rather than validated as output constraints.
- The suite does not yet enforce a strong deterministic validator layer before using an LLM judge.

### Corpus gaps

- Frontend surfaces already use shared case manifests under `ai_evals/cases/frontend/`.
- The remaining gap is breadth and representativeness, not the absence of a shared corpus.
- Cases still need richer metadata, stronger deterministic constraints, and a larger regression library built from real failures.

### Reporting gaps

- Frontend runs already emit the shared benchmark result shape and can write official history snapshots through the shared benchmark CLI.
- There is still no rich leaderboard or trend-oriented debugging workflow for frontend surfaces specifically.
- There is still no strong "worst failures first" report for debugging regressions.

## Frontend: Perfect Testing Logic

The perfect frontend testing logic is:

Frontend should not be the place where the benchmark philosophy is invented.

It should consume the shared case format, validator model, reporting format,
and history format already proven through the CLI path.

### 1. Stay fully headless

Do not mount the chat UI.

Do not click through the frontend.

Do not use Playwright for prompt evaluation.

The runner should directly invoke:

- the production system message builder
- the production user message builder
- the production tool list
- the production chat loop

It is acceptable for the benchmark adapter to use the frontend Vitest/Vite
runtime as a thin loader bridge when production chat modules still depend on
that environment, as long as:

- the benchmark entrypoint remains the shared benchmark CLI
- the benchmark logic and fixtures live under `ai_evals`
- the frontend source tree does not own a separate benchmark suite

This keeps the suite decorrelated from the frontend UI while still testing the real AI logic.

### 2. Test the three frontend AI surfaces separately

#### Script mode

Input:

- user prompt
- optional initial script
- optional context such as selected workspace runnables or DB references

Output:

- final script code

Scoring:

- deterministic validators first
- LLM judge second

Deterministic validators should include:

- expected entrypoint present
- syntax / parse validity
- language-appropriate compile or lint check where feasible
- required behaviors or structures present
- forbidden patterns absent

#### Flow mode

Input:

- user prompt
- optional initial flow
- optional schema
- optional workspace context

Output:

- final flow definition

Scoring:

- flow JSON is structurally valid
- expected module types exist
- expected branches / loops / tools exist
- schema shape matches required inputs
- required data flow connections are present
- LLM judge scores completeness and overall quality

#### App mode

Input:

- user prompt
- optional initial app
- optional workspace context

Output:

- final frontend files
- final backend runnables

Scoring:

- expected files and runnables exist
- file structure is coherent
- app bundle / lint checks pass where feasible in headless mode
- required UI/backend behaviors are represented in the artifact
- LLM judge scores completeness and product quality

### 3. Use repeated runs, not single runs

Each case should run more than once.

Recommended starting point:

- PR smoke run: 2 runs per case on a small curated subset
- nightly reliability run: 5 to 10 runs per case on the full benchmark set

Primary metric:

- pass rate

Secondary metrics:

- average deterministic score
- average judge score
- worst-case judge score
- latency
- total tool calls

### 4. Keep tool traces as diagnostics only

Tool usage matters for debugging, but it should not be the primary score.

The suite should record:

- tool names
- tool arguments
- iteration count
- model/provider

But the main question remains:

> Was the final artifact good?

### 5. Make prompt variants easy to test

Prompt candidates should not require editing test code.

The suite should support a file-based prompt variant workflow.

Example direction:

- `ai_evals/variants/frontend/script/baseline.md`
- `ai_evals/variants/frontend/script/candidate-a.md`
- `ai_evals/variants/frontend/flow/baseline.md`
- `ai_evals/variants/frontend/app/baseline.md`

Each variant should be runnable side by side against the same case set.

### 6. Separate benchmark cases from test code

Benchmark cases should live in data files, not inline in test files.

Each case should define:

- surface
- user prompt
- initial artifact if any
- required constraints
- forbidden constraints
- judge rubric
- tags

This makes the benchmark editable by prompt authors without changing runner logic.

## CLI: What Exists Today

The current CLI tests prove only one narrow property:

> Given a prompt, does the model invoke the expected skill?

That is useful as a smoke signal, but it is far from sufficient for output evaluation.

The current CLI setup also depends on manual preparation of a `.claude/skills` folder, which makes repeated benchmarking and prompt iteration much harder than necessary.

## CLI: What Is Missing

### Output-evaluation gap

- The current suite does not score the artifact produced by the CLI workflow.
- It only checks whether a skill was invoked.
- It does not verify that the resulting files are good.

### Automation gap

- The current setup requires manual copying of generated skills into a test folder.
- That makes the suite too fragile and too manual for rapid prompt iteration.

### Reliability gap

- There is no repeated-run measurement.
- There is no pass-rate metric.
- There is no baseline vs candidate comparison workflow.

### Prompt-variant gap

- There is no first-class way to test alternate skill bundles or alternate generated guidance.
- There is no clean candidate flow for "I changed skill content, show me whether reliability improved."

### Corpus gap

- CLI cases are not aligned with frontend benchmark cases.
- There is no shared benchmark language describing the task, initial state, and expected artifact.

### Reporting gap

- There is no stable output report for artifact comparison.
- There is no failure clustering by skill bundle, task family, or model.

## CLI: Perfect Testing Logic

The perfect CLI testing logic is:

This should be the reference implementation for the suite.

### 1. Evaluate the final artifact, not the skill invocation

Skill invocation should be kept as diagnostic metadata only.

The primary output should be the files produced in a temporary workspace.

Example CLI artifacts:

- generated script files
- generated flow files
- raw app project files
- schedule / trigger config files
- AGENTS / guidance files only when they are directly relevant to the task

### 2. Create the workspace automatically

The runner should create a fresh temporary project for every case.

It should seed that workspace with:

- initial files for the benchmark case
- the current generated CLI guidance and skills
- any fixture data required by the task

It should never depend on a manually maintained test folder.

### 3. Materialize the exact skill bundle under test

The runner should be able to test:

- the current production skill bundle
- a candidate skill bundle built from prompt changes

For CLI, a "prompt variant" is effectively a skill-bundle variant.

That means the suite should support alternate generated skill content without requiring ad hoc manual copies.

### 4. Score the final workspace

The scoring approach should match the frontend philosophy:

- deterministic validators first
- LLM judge second

Deterministic validators for CLI should include:

- expected files created
- expected file names and locations
- required content patterns present
- expected artifact type produced
- optional parse / lint / compile validation where feasible

### 5. Run repeated benchmarks

The CLI should use the same reliability logic as frontend:

- benchmark set
- repeated runs
- pass rate
- baseline vs candidate comparison

### 6. Keep skill traces as diagnostics

Record:

- invoked skills
- order of invocation
- turns
- file changes

But do not let that replace artifact evaluation.

## Perfect Shared Benchmark Model

The frontend and CLI should share the same benchmark concept.

Each evaluation case should define:

- `id`
- `surface`
- `user_prompt`
- `initial_state`
- `workspace_context`
- `artifact_checks`
- `judge_rubric`
- `tags`

The same task should be runnable on multiple surfaces when it makes sense.

This gives direct comparability between:

- frontend script vs CLI script
- frontend flow vs CLI flow
- frontend app vs CLI app

## Recommended Benchmark Categories

The first benchmark set should be broad, but not huge.

Recommended initial size:

- 20 to 30 core cases

Recommended categories:

- from-scratch script creation
- script modification
- from-scratch flow creation
- flow modification
- from-scratch raw app creation
- raw app modification
- reuse of workspace assets
- tasks requiring datatable awareness
- tasks requiring constraints or edge-case handling
- known regressions from real failures

Every category should contain both:

- "easy success" cases
- "high ambiguity" cases

This is essential for measuring reliability rather than only measuring best-case demos.

## Scoring Model

The suite should use three layers.

## Layer 1: Deterministic Validators

This is the hard gate.

Examples:

- parse succeeds
- artifact shape is valid
- required entrypoint exists
- expected files exist
- required module types exist
- expected inputs / schema fields exist
- forbidden patterns are absent

If layer 1 fails, the run is a failure.

## Layer 2: Task-Specific Validators

These are stronger artifact checks derived from the benchmark case.

Examples:

- flow contains a loop and a conditional branch
- app includes a reset button path and backend wiring
- script performs the requested transformation

These should still be deterministic whenever possible.

## Layer 3: LLM Judge

Use an LLM judge only after deterministic validation.

The judge should answer:

- Did the artifact satisfy the request?
- Is it complete?
- Is it coherent for Windmill?
- How close is it to the intended solution?

The judge score is valuable, but it should not be the only oracle.

## Benchmark History

The suite should persist official benchmark summaries in a git-tracked history
layer so improvements and regressions can be reviewed over time.

## What Should Be Git-Tracked

Only official benchmark outputs should be committed:

- post-merge benchmark snapshots on `main`
- scheduled nightly benchmark snapshots
- manually promoted benchmark snapshots when the team wants to record a result

Each official snapshot should produce:

- one detailed run JSON
- one entry in an append-only summary file
- regenerated rollups for trend views

## What Should Not Be Git-Tracked

The following should remain local or external by default:

- raw transcripts
- full model messages
- large generated artifact bundles
- ad hoc local experiments
- temporary comparison runs

This keeps git history focused on stable benchmark signals instead of noisy
debug output.

## Reliability Metrics

Every prompt or skill candidate should be reported with:

- total cases
- passes
- pass rate
- average judge score
- median judge score
- worst-case judge score
- average latency
- average turns

Per-case results should also be retained.

This is the minimum needed to compare:

- baseline vs candidate
- provider vs provider
- frontend vs CLI

## Benchmark Metrics

The history layer should track metrics in four groups.

## Quality Metrics

- `pass_rate`
- `deterministic_pass_rate`
- `judge_score_mean`
- `judge_score_median`
- `judge_score_p10`
- `category_pass_rate`

## Reliability Metrics

- `runs_per_case`
- `flake_rate`
- `path_consistency`

## Efficiency Metrics

- `latency_ms_mean`
- `latency_ms_median`
- `tokens_prompt_mean`
- `tokens_completion_mean`
- `tokens_total_mean`
- `tool_calls_mean`
- `iterations_mean`
- `estimated_cost_mean`
- `cost_per_success`
- `latency_per_success`

## Provenance Metrics

- `timestamp`
- `git_sha`
- `suite_version`
- `scoring_version`
- `surface`
- `variant_name`
- `provider`
- `model`
- `judge_model`

The provenance metrics are essential. Without them, a trend line can mix prompt
changes with upstream model drift and become hard to interpret.

## Efficiency Score

The suite should not collapse everything into one number.

It should track at least three top-level composite scores:

- `quality_score`
- `efficiency_score`
- `value_score`

Recommended interpretation:

- `quality_score`: how good the artifact is
- `efficiency_score`: how fast and cheap the system is relative to peers
- `value_score`: quality-adjusted efficiency

These composite scores should sit on top of the raw metrics, not replace them.

## Proposed Suite Architecture

The suite should be built in six layers.

## Layer 1: Benchmark Data

Purpose:

- define the cases once

Contents:

- case files
- reusable initial fixtures
- evaluation metadata

## Layer 2: Benchmark CLI

Purpose:

- provide one shared entrypoint for the suite

Responsibilities:

- load cases and variants
- select a surface adapter
- run one case or a benchmark set
- invoke shared scoring and history writing
- expose comparison and history commands

## Layer 3: Surface Adapters

Purpose:

- run a case against one surface

Adapters:

- frontend-script adapter
- frontend-flow adapter
- frontend-app adapter
- CLI adapter

Responsibilities:

- prepare the correct prompt environment
- prepare the initial artifact state
- run the real model loop
- return the final artifact plus diagnostics

## Layer 4: Scoring And Reporting

Purpose:

- evaluate the final artifact
- aggregate repeated runs
- compare variants

Responsibilities:

- deterministic validation
- LLM judging
- pass/fail computation
- result serialization
- comparison reports

## Layer 5: Benchmark History

Purpose:

- preserve official benchmark summaries over time
- support trend analysis and regression review

Responsibilities:

- store official run snapshots
- append benchmark summary entries
- generate rollups for charts and dashboards
- keep provenance metadata for every tracked run

## Layer 6: UI Studio

Purpose:

- provide a user interface for the exact same benchmark CLI and runner stack

Important rule:

The UI must not define its own execution semantics.

It must only be a frontend over the same suite used in CI and local benchmarking.

## Proposed Development Order

### Phase 1: Stabilize the benchmark model

Deliverables:

- shared case schema
- shared result schema
- initial core benchmark set

### Phase 2: Build the benchmark CLI shell

Deliverables:

- repo-level benchmark CLI entrypoint
- `run`, `compare`, and `history` command skeletons
- adapter selection layer
- temporary wiring to the first CLI adapter

### Phase 3: Replace the CLI smoke suite with real artifact evaluation

Deliverables:

- temp-workspace runner
- automatic skill-bundle materialization
- artifact scoring
- repeated-run support
- baseline vs candidate skill-bundle comparison

### Phase 4: Add shared reporting and benchmark history around the CLI path

Deliverables:

- baseline vs candidate reports
- pass-rate summaries
- worst-failure reports
- official run schema
- git-tracked benchmark summary file
- history snapshot writer
- rollup generation for trend charts

### Phase 5: Finish the frontend black-box harness on top of the shared model

Deliverables:

- convert current flow and app evals into proper scored reliability tests
- add script eval support
- add repeated-run support
- add prompt-variant loading from files
- align frontend outputs with the shared result and history format
- expose frontend runs through the same benchmark CLI

### Phase 6: Add CI tiers

Deliverables:

- fast PR smoke benchmark
- fuller nightly benchmark
- official history updates on `main` and scheduled runs
- manual benchmark mode for prompt authors

### Phase 7: Build the UI studio

Deliverables:

- run selector
- variant selector
- per-case comparison view
- artifact diff view
- reliability dashboard
- trend dashboard backed by git-tracked benchmark history

This phase comes last because the UI is only valuable once the underlying suite is stable and trusted.

## Proposed Prompt Variant Workflow

The suite should make it cheap to test new prompt candidates.

Recommended workflow:

1. Edit or add a candidate prompt file.
2. Run the benchmark against baseline and candidate.
3. Compare pass rate and score.
4. Inspect worst regressions first.
5. Promote only if the candidate improves the benchmark materially.

For CLI, the same workflow applies, but the tested unit is the generated skill bundle rather than a single chat system prompt.

## Suggested Repository Direction

This plan does not require the UI studio to exist first.

A reasonable repo structure would be:

```text
ai_evals/
  cli/
  cases/
  fixtures/
  history/
    runs/
    rollups/
  variants/
    frontend/
      script/
      flow/
      app/
    cli/
  results/        # gitignored
  scripts/
  adapters/
  scoring/
  reports/
```

The exact folder names can change, but the architectural split should remain.

## What "Done" Looks Like

This project is successful when all of the following are true:

- one repo-level benchmark CLI is the primary way to run prompt evals
- frontend prompt behavior is tested headlessly and independently from the UI
- CLI local-dev behavior is tested by evaluating the final files it produces
- benchmark cases are shared where possible between frontend and CLI
- prompt and skill candidates can be tested without editing test code
- reliability is reported as pass rate over repeated runs
- baseline vs candidate comparisons are easy to run and inspect
- the UI studio is only a thin interface over the same trusted runner

## Final Recommendation

The current frontend evals should be treated as a useful starting point, not the finished solution.

They already prove that the repo can test AI behavior without coupling to the browser UI.

The main work now is:

- build the repo-level benchmark CLI as the durable entrypoint
- replace CLI invocation checks with artifact evaluation
- make the CLI path the reference benchmark implementation
- unify frontend under that same benchmark model
- make frontend evals complete and reliability-oriented only after the shared
  scoring model is stable
- build the UI only after the suite is strong enough to stand on its own
