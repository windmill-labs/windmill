# AI Evals

Small benchmark runner for the Windmill AI generation modes:

- `cli`
- `flow`
- `script`
- `app`
- `global`

The benchmark always tests the current production prompts, tools, and guidance in this checkout.

Each attempt runs:

1. the real production path
2. deterministic validation
3. LLM judging

## Install

```bash
cd ai_evals
bun install
```

Frontend modes also require frontend dependencies:

```bash
cd frontend
bun install
```

## Commands

List model aliases:

```bash
cd ai_evals
bun run cli -- models
```

List cases:

```bash
cd ai_evals
bun run cli -- cases
bun run cli -- cases flow
```

Run benchmarks:

```bash
cd ai_evals
bun run cli -- run flow
bun run cli -- run flow flow-test4-order-processing-loop --model opus
bun run cli -- run flow flow-test0-sum-two-numbers --models haiku,opus,4o
bun run cli -- run flow flow-test0-sum-two-numbers --runs 3 --verbose
bun run cli -- run flow --record
GEMINI_API_KEY=... bun run cli -- run app app-test1-counter-create --model gemini-3-flash-preview
WMILL_AI_EVAL_BACKEND_URL=http://127.0.0.1:8000 bun run cli -- run flow --backend-validation preview
bun run cli -- run global global-test1-script-create
bun run cli -- run cli bun-hello-script
```

Public CLI surface:

- `models`
- `cases [mode]`
- `run <mode> [caseIds...]`

`run` options:

- `--runs <n>`: repeat each case `n` times
- `--output <path>`: custom result JSON path
- `--model <alias>`: choose the model under test
- `--models <a,b,c>`: run the same cases sequentially against several model aliases
- `--verbose`: stream assistant output for frontend runs
- `--skip-judge`: skip LLM judge scoring for the run
- `--execution-only`: only require the model/proxy/frontend loop to complete; skip validators, tool expectations, backend artifact validation, and judge scoring
- `--record`: append a compact tracked summary line to `ai_evals/history/<mode>.jsonl` for full-suite runs only
- `--backend-validation <mode>`: optional backend smoke validation (`off` or `preview`) for `script` and `flow` evals

## Models

Use `bun run cli -- models` to see the current aliases.

Today:

- `haiku`
- `sonnet`
- `opus`
- `4o`
- `gpt-5.5`
- `gemini-3-flash-preview`
- `gemini-3.1-pro-preview`
- `deepseek-v4-flash`
- `deepseek-v4-pro`

Notes:

- the command also prints accepted alias spellings such as `gpt-4o`, `gpt-55`, `claude-opus-4.6`, and `claude-haiku-4.5`
- frontend modes (`flow`, `script`, `app`, `global`) can use Anthropic, OpenAI, Gemini, and DeepSeek-backed aliases
- `cli` mode always uses the Anthropic agent SDK, so only Anthropic aliases are valid there
- the judge model is separate and currently defaults to `claude-sonnet-4-6`; use `--skip-judge` for deterministic-only runs

## Case Format

Cases live in one YAML file per mode under `ai_evals/cases/`.

Minimal shape:

```yaml
- id: flow-test0-sum-two-numbers
  prompt: |-
    Create a flow that takes two numbers, `a` and `b`, and returns their sum.
  initial: ai_evals/fixtures/...
  expected: ai_evals/fixtures/...
```

Optional fields:

- `initial`: starting state fixture
- `expected`: expected artifact fixture
- `validate`: extra deterministic validation rules
- `runtime.backendPreview`: optional real backend preview config for smoke validation

For `flow` mode, `validate` can express requirements such as:

- accepted input schema shapes
- required `results.*` reference validity
- required module/code/input characteristics

For `app` mode, `validate` can express narrow hard requirements such as:

- required frontend file paths or backend runnable keys
- minimum backend runnable counts
- required backend runnable types
- minimum datatable / datatable-table counts
- specific required datatable tables

For `global` mode, `validate` can express draft-level requirements such as:

- required draft type/path/language
- required or forbidden snippets in draft values
- required or forbidden draft counts
- forbidden draft paths

Global initial fixtures can also seed `liveEditorDrafts` with `type`,
`storagePath`, `effectivePath`, and `value` fields. These drafts emulate the
currently open script, flow, or raw app editor so cases can test prompts that
refer to "this" or the "current" item.

Global (and flow) initial fixtures can seed `workspace.datatables` so the
`list_datatables`, `get_datatable_table_schema`, and `exec_datatable_sql` tools
return seeded data during evals. Each entry is
`{ datatable_name, schemas: { <schema>: { <table>: { columns, rows? } } } }`.
SQL runs through a small in-memory engine (`datatableSqlEngine.ts`), not a real
database. Writes are **stateful within a case**: `CREATE`/`DROP`/`INSERT`/`UPDATE`/
`DELETE` mutate the seeded datatable in place, so a later `list_datatables`,
`get_datatable_table_schema`, `SELECT`, or `information_schema` query reflects them
— this is what stops a model from looping when it re-queries to verify a write.
The engine is best-effort: `SELECT` returns all rows of the referenced (or first)
table with no WHERE filtering/projection/joins, `WHERE` on UPDATE/DELETE supports
`col = value` predicates joined by `AND`, and anything unparseable is a no-op
success. So validate datatable cases through tool-use and SQL-argument assertions
(`requiredToolsUsed`, `stringIncludesAnyOf`) — not through exact returned row
values. An empty/absent `datatables` seed makes `list_datatables` return `[]`,
which is what the "no datatable configured" blocking cases rely on.

Set `WMILL_AI_EVAL_DISABLE_ACTIVE_EDITOR_CONTEXT=1` to run those cases with
the old behavior where the live editor is only discoverable through
`list_workspace_items`.

App fixtures can also include an optional `datatables.json` file at the fixture root.

For `flow` mode, an `initial` fixture can also include a benchmark workspace catalog of
existing scripts and flows. That lets the real `search_workspace` and
`get_runnable_details` tools discover reusable workspace runnables during evals.

If `--backend-validation preview` is enabled:

- `script` evals run a real backend script preview in an isolated temp workspace
- `flow` evals run a real backend flow preview only for cases that define `runtime.backendPreview`
- `flow` cases with `initial.workspace` fixtures seed those scripts and flows into the preview workspace before preview
- when `WMILL_AI_EVAL_BACKEND_WORKSPACE` is set, `ai_evals` creates or reuses that workspace as a dedicated test workspace, clears managed eval assets under `f/evals/*` before each preview run, and then reseeds the current case fixtures

Supported backend env vars:

- `WMILL_AI_EVAL_BACKEND_VALIDATION=preview`
- `WMILL_AI_EVAL_BACKEND_URL=http://127.0.0.1:8000`
- `WMILL_AI_EVAL_BACKEND_EMAIL=admin@windmill.dev`
- `WMILL_AI_EVAL_BACKEND_PASSWORD=changeme`
- `WMILL_AI_EVAL_BACKEND_WORKSPACE=integration-tests` to reuse an existing workspace on CE installs with low workspace limits

Frontend modes require a reachable Windmill backend and send model requests through the workspace AI proxy at `/api/w/{workspace}/ai/proxy`. At startup, `ai_evals` checks the resolved backend URL and fails early with setup guidance if the backend cannot be reached or login fails.

For frontend modes:

- `ai_evals` creates a temporary backend workspace, or creates/reuses `WMILL_AI_EVAL_BACKEND_WORKSPACE` when it is set
- it upserts a provider resource under `f/evals/ai/<provider>`
- frontend requests go through `/api/w/{workspace}/ai/proxy`

## Results And Artifacts

Every run writes:

- a summary JSON under `ai_evals/results/`
- generated artifacts in a sibling directory

If `--record` is used, the CLI also appends one compact JSON line to:

- `ai_evals/history/flow.jsonl`
- `ai_evals/history/script.jsonl`
- `ai_evals/history/app.jsonl`
- `ai_evals/history/global.jsonl`
- `ai_evals/history/cli.jsonl`

Each recorded line contains:

- run metadata (`createdAt`, `gitSha`, `mode`, `runModel`, `judgeModel`)
- suite totals (`caseCount`, `attemptCount`, `passedAttempts`, `passRate`, `averageDurationMs`, `averagePassedDurationMs`, `averageJudgeScore`)
- average token usage (`averageTokenUsagePerAttempt`, `averageTokenUsagePerPassedAttempt`)
- per-case metrics under `cases[]` (`averageDurationMs`, `averagePassedDurationMs`, `averageJudgeScore`, `averageTokenUsagePerAttempt`, `averageTokenUsagePerPassedAttempt`, pass rate)
- `failedCaseIds`

The CLI headline duration and token averages use passed attempts only.
All-attempt averages are still recorded to make failures auditable without
letting failed attempts skew success cost comparisons.

Example:

- summary: `ai_evals/results/2026-04-09T09-40-33.051Z__flow.json`
- artifacts: `ai_evals/results/2026-04-09T09-40-33.051Z__flow/`

Typical artifacts by mode:

- `flow`: `flow.json`
- `script`: `script.json` plus the generated script file
- `app`: `app.json` plus frontend/backend files
- `global`: `global-drafts.json`
- `cli`: `assistant-output.txt`, `trace.json`, `wmill-invocations.jsonl`, plus generated workspace files
- backend-validated attempts also include `backend-preview.json`

## Layout

- `cases/`: one YAML file per mode
- `fixtures/`: initial and expected fixtures
- `core/`: shared loading, model resolution, validation, judging, and result writing
- `modes/`: one runner per mode
- `history/`: optional tracked pass-rate history written by `run --record`, one JSONL file per mode
- `results/`: local benchmark output and artifacts

## Notes

- Frontend modes reuse the production frontend chat code through the Vitest bridge.
- Global mode evaluates the production global AI tools and validates the resulting AI draft store.
- CLI mode creates an isolated workspace, writes the current checkout guidance into it, and benchmarks the real skills / `AGENTS.md` flow.
- CLI mode now also records a structured trace of invoked skills, tool calls, proposed `wmill` commands, and any attempted `wmill` executions.
- Frontend progress streams live while the benchmark is running.
- Deterministic validators should stay focused on real correctness constraints, not one exact implementation shape.
