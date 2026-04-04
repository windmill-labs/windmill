# Benchmark History

This directory stores the git-tracked benchmark history for official AI eval
runs.

## Purpose

The history layer answers a different question than a one-off benchmark run:

> Are our prompts and skill bundles getting better over time?

Only official benchmark outputs should be committed here.

## What Counts As Official

Tracked benchmark entries should come from one of these sources:

- post-merge runs on `main`
- scheduled nightly benchmark runs
- manually promoted benchmark runs the team wants to preserve

Ad hoc local experiments should stay under `ai_evals/results/` or another
untracked location.

## File Layout

- `benchmark-run.schema.json`: contract for one official run snapshot
- `runs/`: detailed per-run JSON snapshots
- `summary.jsonl`: one compact summary row per official run
- `rollups/`: chart-ready aggregates rebuilt from `summary.jsonl`

## Summary Metrics

Each official run should record enough information to compare quality,
reliability, efficiency, and provenance over time.

The expected metric groups are:

- quality: pass rate and judge-score rollups
- reliability: run-count and flake-related metrics
- efficiency: latency, tokens, tool usage, and cost metrics
- provenance: git SHA, suite/scoring version, provider, and model identity

## Usage

Append one official run snapshot with:

```bash
node ai_evals/scripts/append-official-run.mjs --input /path/to/run.json
```

This command will:

1. validate the input shape
2. write the detailed snapshot under `runs/`
3. upsert one summary row in `summary.jsonl`
4. rebuild the rollup JSON files

The writer is intentionally simple so frontend and CLI runners can adopt the
same format later.
