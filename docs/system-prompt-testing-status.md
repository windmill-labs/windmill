# System Prompt Testing Status

This document describes the benchmark tool that exists today. It is the current
truth for `ai_evals/`.

The longer planning document in
[system-prompt-testing-plan.md](/home/farhad/windmill__worktrees/prompt-testing-plan/docs/system-prompt-testing-plan.md)
still contains useful background, but parts of its workflow are now historical
because the old variants/history system was removed.

## Current Tool

There is one repo-level benchmark CLI under `ai_evals/` with three commands:

- `bun run cli -- models`
- `bun run cli -- cases [mode]`
- `bun run cli -- run <mode> [caseIds...]`

Supported modes:

- `cli`
- `flow`
- `script`
- `app`

Public `run` options:

- `--runs <n>`
- `--output <path>`
- `--model <alias>`
- `--verbose`
- `--record`

There is no variant workflow and no compare command in the current tool.
Tracked history is intentionally minimal: `run --record` appends one compact
summary line to `ai_evals/history/<mode>.jsonl`. This is only allowed for
full-suite runs, not selected case ids. History lines include average token
usage when the benchmark mode reports it, plus average judge score and per-case
duration/judge/token usage summaries.

## How It Works

Each attempt runs:

1. the current production prompts, tools, and guidance from this checkout
2. deterministic validation
3. LLM judging

Results are written locally under `ai_evals/results/` as:

- a summary JSON file
- a sibling artifacts directory containing the generated flow/script/app/workspace

If `--record` is used, the CLI also appends a compact JSONL summary line to the
tracked file for that mode under `ai_evals/history/`.

## Current Architecture

- `ai_evals/cases/`: one YAML manifest per mode
- `ai_evals/fixtures/`: initial and expected fixtures
- `ai_evals/core/`: shared case loading, model resolution, validation, judging, and result writing
- `ai_evals/history/`: optional tracked pass-rate history written by `run --record`, one JSONL file per mode
- `ai_evals/modes/`: one runner per mode

Execution model:

- `flow`, `script`, and `app` reuse the production frontend chat loop and production tool definitions through the frontend Vitest bridge
- `cli` creates a temp workspace, writes the current checkout guidance into it, and runs the Anthropic agent SDK against that workspace

## Case Model

Each case is intentionally small:

- `prompt`
- optional `initial`
- optional `expected`
- optional `validate`

`validate` is mainly used for stronger deterministic checks where exact fixture
matching would be too strict, especially for `flow` creation cases.

Examples of current deterministic checks:

- schema contains one of several accepted input shapes
- `results.*` references resolve
- required code/input characteristics exist in some module
- expected workspace files are created in `cli` mode

## Model Selection

Model aliases are resolved through a shared registry in `ai_evals/core/models.ts`.

Current aliases:

- `haiku`
- `sonnet`
- `opus`
- `4o`

Notes:

- the `models` command also shows accepted alias spellings such as `gpt-4o` and `claude-opus-4.6`
- frontend modes can use Anthropic and OpenAI-backed aliases
- `cli` mode is Anthropic-only because it runs through the Anthropic agent SDK
- the judge model is separate and currently defaults to `claude-sonnet-4-6`

## What Is Working Well

- one simple local benchmark CLI
- real production execution paths instead of synthetic prompt variants
- local result and artifact persistence by default
- live frontend progress output
- reusable flow/script/app/cli runners under one tool
- deterministic validation can now catch real runtime-invalid flow wiring

## What Still Needs Work

- broader case coverage across all four modes
- stronger deterministic validators for more cases, especially app/script semantics
- clearer per-case validation metadata as the corpus grows
- CI automation for smoke and nightly runs

## Recommended Next Focus

The next high-value work is:

1. add more realistic benchmark cases
2. keep simplifying deterministic validators so they check correctness, not one exact implementation
3. add CI only after the local benchmark signal is trustworthy
