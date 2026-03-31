# AI Evals

This directory contains the shared benchmark data and supporting assets for
black-box AI output evaluation across frontend chat modes and the CLI.

## Purpose

The goal is to evaluate the final artifact produced by the AI system, not the
internal implementation details of the frontend or CLI.

The intended end state is a new repo-level benchmark CLI for running the shared
eval suite across both the Windmill CLI and the frontend.

Current code under `cli/test-skills/` should be treated as bootstrap/prototype
implementation work, not the final entrypoint.

## Layout

- `cli/`: repo-level benchmark CLI entrypoint and orchestration
- `cases/`: benchmark case manifests
- `fixtures/`: shared reusable starting states and assets
- `history/`: git-tracked benchmark summaries and chart rollups
- `scripts/`: benchmark-history and reporting utilities
- `variants/`: prompt or skill-bundle candidates
- `results/`: run outputs and reports (intended to stay untracked)

## Initial Scope

The first implementation step is to move the existing frontend flow and app
benchmark prompts out of inline test code and into shared case manifests.

That scaffolding exists now, but the next implementation priority should be the
repo-level benchmark CLI shell, with the Windmill CLI adapter as the first real
surface behind it.

Later phases should add:

- repo-level benchmark commands (`run`, `compare`, `history`)
- CLI artifact-evaluation cases
- shared result schemas
- repeated-run reliability reporting
- official benchmark history snapshots and rollups
- frontend `script` cases
- frontend adapters aligned to the shared CLI-proven scoring model

## Current Entry Point

The benchmark CLI shell now lives under `ai_evals/cli/`.

Current usage:

```bash
bun ai_evals/cli/index.ts list-cases --surface cli
bun ai_evals/cli/index.ts run --surface cli --case bun-hello-script
```

At the moment this is intentionally thin and delegates to the bootstrap CLI
adapter code under `cli/test-skills/`.
