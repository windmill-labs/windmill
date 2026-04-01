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
- frontend flow and app are now exposed through that same benchmark CLI
- frontend AI benchmark logic now lives under `ai_evals`, not under `frontend/__tests__`
- frontend benchmarks reuse production prompt builders, tool definitions, and `runChatLoop`
- CLI cases are now skill-sensitive rather than only artifact-sensitive
- the benchmark harness and `wmill init` now share the same AI-guidance writer
- variants can be frozen as named snapshots through the benchmark CLI
- repeated-run CLI benchmarking exists through `--runs`
- basic CLI reliability metrics now exist
- benchmark history writing and reading now exist in the benchmark CLI
- the CLI base is strong enough to stop blocking the frontend adapter

What is not true yet:

- frontend `script` is not yet exposed through the benchmark CLI
- frontend still has only baseline prompt variants wired by default
- frontend still uses a thin Vitest bridge for module/runtime loading
- CLI still needs broader case coverage and richer efficiency metrics
- token and cost metrics are not implemented
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
- richer shared aggregation/reporting conventions across surfaces

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
- `history`
- CLI adapter selection through `--surface cli`
- frontend adapter selection through `--surface frontend-flow|frontend-app`

Still missing:

- frontend `script` adapter selection

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
- repeated-run support through `--runs`
- aggregate CLI reliability reporting:
  - pass rate
  - average duration
  - average assistant messages
  - average tool calls
  - average skill invocations
  - aggregated required-check failures

Most important proof point:

- a deliberately broken skill variant produced a meaningful regression:
  - baseline: `2/2` CLI cases passed
  - broken variant: `0/2` CLI cases passed

Still missing:

- richer case coverage beyond the current initial script and flow cases
- richer efficiency metrics such as token and cost reporting

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

- mostly done

Implemented:

- history scaffold under `ai_evals/history/`
- official run schema scaffold in `benchmark-run.schema.json`
- `summary.jsonl`
- rollup placeholders
- shared history writer under `ai_evals/history/writer.mjs`
- snapshot writer script under `ai_evals/scripts/append-official-run.mjs`
- benchmark CLI wiring for `compare --write-history`
- benchmark CLI `history --view latest|summary|surface|variant|model`
- official run snapshots generated from repeated CLI benchmark results
- history rollups rebuilt from real benchmark writes
- shared history writing path generalized so frontend surfaces can use the same official run format
- frontend history writes validated through `compare --write-history` on `frontend-app`

Still missing:

- pass-rate summaries across repeated runs
- worst-failure reporting
- richer history filtering and reporting around those snapshots

### Phase 5: Finish the frontend black-box harness on top of the shared model

Planned:

- convert current flow and app evals into proper scored reliability tests
- add script eval support
- add repeated-run support
- add prompt-variant loading from files
- align frontend outputs with the shared result and history format
- expose frontend runs through the same benchmark CLI

Status:

- partially done

Implemented:

- shared frontend case scaffolding exists
- frontend benchmark fixtures were moved under `ai_evals/fixtures/frontend/`
- repo-level benchmark CLI support for `frontend-flow` and `frontend-app`
- ai_evals-owned frontend benchmark runner and adapter executed through the frontend Vitest environment
- frontend benchmark helpers now write to temp workspaces instead of mutating in-memory UI state
- repeated runs for frontend flow/app through the shared benchmark CLI
- frontend prompt variants loaded from files under `ai_evals/variants/frontend/`
- frontend run/compare payloads aligned to the shared benchmark result shape
- frontend judge score aggregation wired into compare output and official run generation
- frontend official history writes validated through the shared benchmark CLI
- legacy frontend AI chat eval tests removed from `frontend/src/lib/components/copilot/chat/__tests__`

Still missing:

- frontend `script` support
- more than the baseline prompt variant for frontend flow/app
- broader frontend reliability coverage across more cases in routine use
- regular use of frontend history writes in routine benchmark workflows
- optional future cleanup to remove or reduce the thin frontend Vitest bridge if production chat modules become directly importable

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
- Added repeated-run CLI benchmarking with aggregate reporting
- Added frontend flow/app benchmark adapters behind the shared benchmark CLI
- Added frontend run/compare support with shared aggregation and judge score reporting
- Validated frontend `run`, `compare`, and `compare --write-history` through the shared benchmark CLI
- Moved frontend AI benchmark fixtures and runner ownership into `ai_evals`
- Removed the old frontend AI benchmark test tree
- Kept the production frontend chat loop and tool-definition path as the shared execution core
- Swapped frontend benchmark helpers from in-memory state to temp-workspace files
- Shared AI-guidance generation between:
  - benchmark temp workspaces
  - `wmill init`
- Moved `wmill init` testing overrides to internal env vars instead of public flags
- Added docs for variant workflows and benchmark usage
- Added official benchmark history writing and reading

## What Is Left To Do

The highest-priority remaining work is:

1. Add the missing frontend `script` surface behind the same benchmark CLI.
2. Expand the frontend and CLI case corpora to cover more real prompt and skill behavior.
3. Add richer aggregate metrics:
   - pass rate
   - flake rate
   - latency
   - tool-call count
   - token and cost metrics if available
4. Add stronger official history summaries:
   - worst-failure views
   - better pass-rate rollups
   - more filtering
5. Add CI tiers.
6. Build the UI last.
7. Optionally reduce the thin frontend Vitest bridge once production chat modules are easier to load headlessly.

## Recommended Next Step

The best next implementation step is:

- add the frontend `script` adapter behind the existing benchmark CLI

Reason:

- flow and app now already run through the shared benchmark CLI and can write official snapshots
- `script` is the main frontend gap left before the frontend surface is structurally complete
- the remaining CLI work should stay tracked as follow-up TODOs while the main implementation path moves to fuller frontend coverage

## Relevant Files

- Plan: [system-prompt-testing-plan.md](/home/farhad/windmill__worktrees/prompt-testing-plan/docs/system-prompt-testing-plan.md)
- Benchmark CLI: [index.ts](/home/farhad/windmill__worktrees/prompt-testing-plan/ai_evals/cli/index.ts)
- CLI adapter: [artifact-eval.ts](/home/farhad/windmill__worktrees/prompt-testing-plan/ai_evals/adapters/cli/artifact-eval.ts)
- Frontend adapter runtime: [runtime.ts](/home/farhad/windmill__worktrees/prompt-testing-plan/ai_evals/adapters/frontend/runtime.ts)
- Frontend benchmark runner: [benchmarkRunner.ts](/home/farhad/windmill__worktrees/prompt-testing-plan/ai_evals/adapters/frontend/benchmarkRunner.ts)
- Frontend Vitest bridge: [vitestAdapter.test.ts](/home/farhad/windmill__worktrees/prompt-testing-plan/ai_evals/adapters/frontend/vitestAdapter.test.ts)
- CLI variants: [variants.ts](/home/farhad/windmill__worktrees/prompt-testing-plan/ai_evals/adapters/cli/variants.ts)
- Shared guidance writer: [writer.ts](/home/farhad/windmill__worktrees/prompt-testing-plan/cli/src/guidance/writer.ts)
- CLI benchmark docs: [README.md](/home/farhad/windmill__worktrees/prompt-testing-plan/ai_evals/cli/README.md)
