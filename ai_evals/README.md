# AI Evals

Small benchmark runner for the four Windmill AI generation modes:

- `cli`
- `flow`
- `script`
- `app`

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
- `--record`: append a compact tracked summary line to `ai_evals/history/<mode>.jsonl` for full-suite runs only

## Models

Use `bun run cli -- models` to see the current aliases.

Today:

- `haiku`
- `sonnet`
- `opus`
- `4o`
- `gemini-flash`
- `gemini-pro`
- `gemini-3-flash-preview`
- `gemini-3.1-pro-preview`

Notes:

- the command also prints accepted alias spellings such as `gpt-4o`, `claude-opus-4.6`, and `claude-haiku-4.5`
- frontend modes (`flow`, `script`, `app`) can use Anthropic, OpenAI, and Gemini-backed aliases
- `cli` mode always uses the Anthropic agent SDK, so only Anthropic aliases are valid there
- the judge model is separate and currently defaults to `claude-sonnet-4-6`

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

For `flow` mode, `validate` can express requirements such as:

- accepted input schema shapes
- required `results.*` reference validity
- required module/code/input characteristics

For `flow` mode, an `initial` fixture can also include a benchmark workspace catalog of
existing scripts and flows. That lets the real `search_workspace` and
`get_runnable_details` tools discover reusable workspace runnables during evals.

## Results And Artifacts

Every run writes:

- a summary JSON under `ai_evals/results/`
- generated artifacts in a sibling directory

If `--record` is used, the CLI also appends one compact JSON line to:

- `ai_evals/history/flow.jsonl`
- `ai_evals/history/script.jsonl`
- `ai_evals/history/app.jsonl`
- `ai_evals/history/cli.jsonl`

Each recorded line contains:

- run metadata (`createdAt`, `gitSha`, `mode`, `runModel`, `judgeModel`)
- suite totals (`caseCount`, `attemptCount`, `passedAttempts`, `passRate`, `averageDurationMs`, `averageJudgeScore`)
- average token usage (`averageTokenUsagePerAttempt`)
- per-case metrics under `cases[]` (`averageDurationMs`, `averageJudgeScore`, `averageTokenUsagePerAttempt`, pass rate)
- `failedCaseIds`

Example:

- summary: `ai_evals/results/2026-04-09T09-40-33.051Z__flow.json`
- artifacts: `ai_evals/results/2026-04-09T09-40-33.051Z__flow/`

Typical artifacts by mode:

- `flow`: `flow.json`
- `script`: `script.json` plus the generated script file
- `app`: `app.json` plus frontend/backend files
- `cli`: `assistant-output.txt` plus generated workspace files

## Layout

- `cases/`: one YAML file per mode
- `fixtures/`: initial and expected fixtures
- `core/`: shared loading, model resolution, validation, judging, and result writing
- `modes/`: one runner per mode
- `history/`: optional tracked pass-rate history written by `run --record`, one JSONL file per mode
- `results/`: local benchmark output and artifacts

## Notes

- Frontend modes reuse the production frontend chat code through the Vitest bridge.
- CLI mode creates an isolated workspace, writes the current checkout guidance into it, and benchmarks the real skills / `AGENTS.md` flow.
- Frontend progress streams live while the benchmark is running.
- Deterministic validators should stay focused on real correctness constraints, not one exact implementation shape.
