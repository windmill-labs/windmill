# Benchmark CLI

This directory is the start of the repo-level benchmark CLI described in
`docs/system-prompt-testing-plan.md`.

## Current Scope

The current implementation is intentionally small:

- `run` command
- `cli` surface only

This is the benchmark entrypoint for prompt and artifact evaluation.

## Usage

Install dependencies once:

```bash
cd ai_evals
bun install
```

List available CLI cases:

```bash
cd ai_evals
bun run cli -- list-cases --surface cli
```

Run one CLI case:

```bash
cd ai_evals
bun run cli -- run --surface cli --case bun-hello-script
```

Keep the temp workspace for inspection:

```bash
cd ai_evals
bun run cli -- run --surface cli --case bun-hello-script --keep-workspace
```

Print machine-readable output:

```bash
cd ai_evals
bun run cli -- run --surface cli --case bun-hello-script --json
```

## Next Steps

Later iterations should add:

- `compare` command
- `history` command
- frontend adapters
- shared result/history writing from this entrypoint
