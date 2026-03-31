# Benchmark CLI

This directory is the start of the repo-level benchmark CLI described in
`docs/system-prompt-testing-plan.md`.

## Current Scope

The current implementation is intentionally small:

- `run` command
- `cli` surface only
- runs the existing CLI artifact-eval adapter from `cli/test-skills/`

This is a migration step away from using `cli/test-skills/` as the primary
entrypoint.

## Usage

List available CLI cases:

```bash
bun ai_evals/cli/index.ts list-cases --surface cli
```

Run one CLI case:

```bash
bun ai_evals/cli/index.ts run --surface cli --case bun-hello-script
```

Keep the temp workspace for inspection:

```bash
bun ai_evals/cli/index.ts run --surface cli --case bun-hello-script --keep-workspace
```

Print machine-readable output:

```bash
bun ai_evals/cli/index.ts run --surface cli --case bun-hello-script --json
```

## Next Steps

Later iterations should add:

- `compare` command
- `history` command
- frontend adapters
- shared result/history writing from this entrypoint
