# System Prompt Testing Status

This document tracks implementation progress against
[system-prompt-testing-plan.md](/home/farhad/windmill__worktrees/prompt-testing-plan/docs/system-prompt-testing-plan.md).

It is intentionally operational:

- what has been implemented already
- what is only scaffolded
- what is still missing
- what should be built next

## Current Summary

The suite has made meaningful progress on the CLI-first path from the plan.

What is true today:

- there is one repo-level benchmark CLI under `ai_evals/cli`
- the CLI surface is the first real benchmark adapter behind it
- CLI cases are now skill-sensitive rather than only artifact-sensitive
- the benchmark harness and `wmill init` now share the same AI-guidance writer
- variants can be frozen as named snapshots through the benchmark CLI
- benchmark history scaffolding exists in `ai_evals/history`

What is not true yet:

- frontend is not yet exposed through the benchmark CLI
- repeated-run reliability is not implemented
- efficiency metrics are not implemented
- official history writing is not yet wired into normal benchmark commands
- no UI studio exists yet

## Comparison To The Plan

The plan in
[system-prompt-testing-plan.md](/home/farhad/windmill__worktrees/prompt-testing-plan/docs/system-prompt-testing-plan.md)
defined a seven-phase delivery order.

### Phase 1: Stabilize the benchmark model

Planned:

- shared case schema
- shared result schema
- initial core benchmark set

Status:

- partially done

Implemented:

- shared case storage under `ai_evals/cases/`
- initial frontend case manifests under `ai_evals/cases/frontend/`
- initial CLI case manifests under `ai_evals/cases/cli/`
- a practical result shape emitted by the benchmark CLI for `run` and `compare`

Still missing:

- a formally versioned shared result schema for all surfaces
- a bigger representative benchmark set
- repeated-run result aggregation

### Phase 2: Build the benchmark CLI shell

Planned:

- repo-level benchmark CLI entrypoint
- `run`, `compare`, and `history` command skeletons
- adapter selection layer
- temporary wiring to the first CLI adapter

Status:

- mostly done

Implemented:

- repo-level benchmark CLI in [index.ts](/home/farhad/windmill__worktrees/prompt-testing-plan/ai_evals/cli/index.ts)
- `list-cases`
- `list-variants`
- `run`
- `compare`
- `snapshot-variant`
- CLI adapter selection through `--surface cli`

Still missing:

- `history` command implementation
- frontend adapter selection

### Phase 3: Replace the CLI smoke suite with real artifact evaluation

Planned:

- temp-workspace runner
- automatic skill-bundle materialization
- artifact scoring
- repeated-run support
- baseline vs candidate skill-bundle comparison

Status:

- mostly done

Implemented:

- legacy `cli/test-skills` harness removed
- temp-workspace runner in [artifact-eval.ts](/home/farhad/windmill__worktrees/prompt-testing-plan/ai_evals/adapters/cli/artifact-eval.ts)
- automatic guidance materialization into the temp workspace
- required artifact checks for CLI cases
- required skill invocation checks for CLI cases
- required “next commands” guidance checks for CLI cases
- baseline vs candidate comparison through `compare`
- named variant snapshotting through `snapshot-variant`

Most important proof point:

- a deliberately broken skill variant produced a meaningful regression:
  - baseline: `2/2` CLI cases passed
  - broken variant: `0/2` CLI cases passed

Still missing:

- repeated runs
- aggregate reliability reporting
- richer case coverage beyond the current initial script and flow cases

### Phase 4: Add shared reporting and benchmark history around the CLI path

Planned:

- baseline vs candidate reports
- pass-rate summaries
- worst-failure reports
- official run schema
- git-tracked benchmark summary file
- history snapshot writer
- rollup generation for trend charts

Status:

- partially done

Implemented:

- history scaffold under `ai_evals/history/`
- official run schema scaffold in `benchmark-run.schema.json`
- `summary.jsonl`
- rollup placeholders
- snapshot writer script under `ai_evals/scripts/append-official-run.mjs`

Still missing:

- benchmark CLI wiring to emit official history snapshots
- pass-rate summaries across repeated runs
- worst-failure reporting
- chart-ready data generated from real benchmark runs instead of placeholders

### Phase 5: Finish the frontend black-box harness on top of the shared model

Planned:

- convert current flow and app evals into proper scored reliability tests
- add script eval support
- add repeated-run support
- add prompt-variant loading from files
- align frontend outputs with the shared result and history format
- expose frontend runs through the same benchmark CLI

Status:

- mostly not done

Implemented:

- shared frontend case scaffolding exists
- current frontend flow/app evals were moved toward shared manifests earlier

Still missing:

- frontend adapter behind the benchmark CLI
- scored frontend reliability tests
- frontend `script` support
- repeated runs
- shared result/history alignment
- prompt-variant workflow for frontend

### Phase 6: Add CI tiers

Planned:

- fast PR smoke benchmark
- fuller nightly benchmark
- official history updates on `main` and scheduled runs
- manual benchmark mode for prompt authors

Status:

- not done

Implemented:

- local manual benchmark mode exists through the benchmark CLI

Still missing:

- PR CI integration
- nightly integration
- automated official history writes

### Phase 7: Build the UI studio

Planned:

- run selector
- variant selector
- per-case comparison view
- artifact diff view
- reliability dashboard
- trend dashboard backed by git-tracked benchmark history

Status:

- not started

This matches the plan’s intended order. The UI should still come last.

## What Was Actually Done

The most important implemented changes so far are:

- Added the repo-level benchmark CLI in `ai_evals/cli`
- Removed the old duplicate CLI harness so there is one benchmark entrypoint
- Added CLI `run`, `compare`, `list-cases`, and `list-variants`
- Added CLI `snapshot-variant` to freeze a named candidate bundle
- Added initial CLI benchmark cases:
  - `bun-hello-script`
  - `bun-hello-flow`
- Made CLI evals skill-sensitive instead of allowing silent skill bypass
- Shared AI-guidance generation between:
  - benchmark temp workspaces
  - `wmill init`
- Moved `wmill init` testing overrides to internal env vars instead of public flags
- Added docs for variant workflows and benchmark usage
- Added benchmark history scaffolding

## What Is Left To Do

The highest-priority remaining work is:

1. Add repeated-run support to CLI benchmarks.
2. Add aggregate metrics:
   - pass rate
   - flake rate
   - latency
   - tool-call count
   - token and cost metrics if available
3. Implement official history writing from the benchmark CLI.
4. Expand the CLI case corpus to cover more real skill behavior.
5. Bring frontend behind the same benchmark CLI.
6. Add CI tiers.
7. Build the UI last.

## Recommended Next Step

The best next implementation step is:

- repeated-run CLI benchmarking with aggregate reporting

Reason:

- the current CLI harness can now detect meaningful skill regressions
- the next missing layer is reliability, not basic capability
- repeated runs are also the foundation for useful history metrics and later UI reporting

## Relevant Files

- Plan: [system-prompt-testing-plan.md](/home/farhad/windmill__worktrees/prompt-testing-plan/docs/system-prompt-testing-plan.md)
- Benchmark CLI: [index.ts](/home/farhad/windmill__worktrees/prompt-testing-plan/ai_evals/cli/index.ts)
- CLI adapter: [artifact-eval.ts](/home/farhad/windmill__worktrees/prompt-testing-plan/ai_evals/adapters/cli/artifact-eval.ts)
- CLI variants: [variants.ts](/home/farhad/windmill__worktrees/prompt-testing-plan/ai_evals/adapters/cli/variants.ts)
- Shared guidance writer: [writer.ts](/home/farhad/windmill__worktrees/prompt-testing-plan/cli/src/guidance/writer.ts)
- CLI benchmark docs: [README.md](/home/farhad/windmill__worktrees/prompt-testing-plan/ai_evals/cli/README.md)
