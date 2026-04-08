# AI Evals

Minimal benchmark runner for the four Windmill AI generation modes:

- `cli`
- `flow`
- `script`
- `app`

Each case is just:

- a `prompt`
- an optional `initial` fixture
- an optional `expected` fixture

For `flow` mode, an `initial` fixture can also include a benchmark workspace catalog of
existing scripts and flows. That lets the real `search_workspace` /
`get_runnable_details` tools discover reusable building blocks during evals.

Each attempt runs:

1. the real production prompt/tool/guidance path
2. deterministic validation
3. LLM judging

## Install

```bash
cd ai_evals
bun install
```

Frontend runs also require frontend dependencies:

```bash
cd frontend
bun install
```

## CLI

List cases:

```bash
cd ai_evals
bun run cli -- cases
bun run cli -- cases flow
```

Run a mode:

```bash
cd ai_evals
bun run cli -- run flow
bun run cli -- run flow flow-test5-simple-modification --runs 3
bun run cli -- run cli bun-hello-script
```

`run` always writes a JSON result file under `ai_evals/results/` unless you pass
`--output`.

It also writes generated artifacts next to that summary file, for example:

- summary: `ai_evals/results/2026-04-08T13-00-00.000Z__flow.json`
- artifacts: `ai_evals/results/2026-04-08T13-00-00.000Z__flow/<case-id>/attempt-1/flow.json`

## Layout

- `cases/`: one YAML file per mode
- `fixtures/`: initial and expected fixtures
- `core/`: shared case loading, validation, judging, and result writing
- `modes/`: one runner per mode

## Notes

- Frontend modes reuse the production frontend chat code through the Vitest bridge.
- CLI mode creates an isolated workspace, writes the current checkout guidance into it, and benchmarks the real skills / AGENTS flow.
- Frontend progress streams live while the benchmark is running.
