# Benchmark CLI

This directory is the start of the repo-level benchmark CLI described in
`docs/system-prompt-testing-plan.md`.

## Current Scope

The current implementation is intentionally small:

- `run` command
- `compare` command
- `history` command
- `list-cases` and `list-variants` discovery commands
- `cli`, `frontend-flow`, and `frontend-app` surfaces

This is the benchmark entrypoint for prompt and artifact evaluation.

The CLI benchmark workspace now uses the same shared AI-guidance writer as
`wmill init`, so benchmark runs and local CLI bootstraps go through the same
project-guidance generation path.

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

List available frontend cases:

```bash
cd ai_evals
bun run cli -- list-cases --surface frontend-flow
bun run cli -- list-cases --surface frontend-app
```

List available CLI variants:

```bash
cd ai_evals
bun run cli -- list-variants --surface cli
```

List available frontend variants:

```bash
cd ai_evals
bun run cli -- list-variants --surface frontend-flow
bun run cli -- list-variants --surface frontend-app
```

Snapshot the current guidance bundle into a named CLI variant:

```bash
cd ai_evals
bun run cli -- snapshot-variant --surface cli --variant candidate --description "Candidate CLI guidance bundle"
```

Run one CLI case:

```bash
cd ai_evals
bun run cli -- run --surface cli --case bun-hello-script --variant baseline
```

Run one frontend flow case:

```bash
cd ai_evals
bun run cli -- run --surface frontend-flow --case flow-test1-user-role-actions --variant baseline --runs 1
```

Run one frontend app case:

```bash
cd ai_evals
bun run cli -- run --surface frontend-app --case app-test1-counter-create --variant baseline --runs 1
```

Keep the temp workspace for inspection:

```bash
cd ai_evals
bun run cli -- run --surface cli --case bun-hello-script --variant baseline --keep-workspace
```

Print machine-readable output:

```bash
cd ai_evals
bun run cli -- run --surface cli --case bun-hello-script --variant baseline --json
```

Run the same case multiple times to measure reliability:

```bash
cd ai_evals
bun run cli -- run --surface cli --case bun-hello-script --variant baseline --runs 5 --json
```

Compare two variant selections on one or more cases:

```bash
cd ai_evals
bun run cli -- compare --surface cli --case bun-hello-script --variant baseline --variant baseline --json
```

Write official benchmark snapshots while comparing distinct variants:

```bash
cd ai_evals
bun run cli -- compare --surface cli --case bun-hello-script --variant baseline-frozen --variant candidate --runs 5 --write-history
```

Compare frontend prompt variants:

```bash
cd ai_evals
bun run cli -- compare --surface frontend-flow --case flow-test1-user-role-actions --variant baseline --variant candidate --runs 3
```

Write official history snapshots for frontend variants:

```bash
cd ai_evals
bun run cli -- compare --surface frontend-app --case app-test1-counter-create --variant baseline --variant candidate --runs 3 --write-history
```

Inspect the tracked history:

```bash
cd ai_evals
bun run cli -- history --view latest
bun run cli -- history --view summary --limit 10
```

## Benchmarking A CLI Skill Change

If you change one of the generated CLI skills and want to know whether the
change improved the system, do not compare against the moving `baseline`
variant alone.

`baseline` points at the repo's current generated skills, so it changes when
the repo changes. To make a real before-vs-after comparison, snapshot both
sides as named variants.

Before changing the skills, freeze the current bundle:

```bash
cd ai_evals
bun run cli -- snapshot-variant --surface cli --variant baseline-frozen --description "Frozen CLI skills before the change"
```

After changing and regenerating the guidance, snapshot the candidate:

```bash
cd ai_evals
bun run cli -- snapshot-variant --surface cli --variant candidate --description "CLI skills after the change"
```

Then compare them on one or more cases:

```bash
cd ai_evals
bun run cli -- compare --surface cli --case bun-hello-script --case bun-hello-flow --variant baseline-frozen --variant candidate --runs 5 --json
```

Today, the most meaningful improvement signal is still output quality:

- required artifact checks passing more often
- more cases passing
- fewer required failures across repeated runs

The benchmark CLI now reports basic reliability and efficiency-adjacent metrics
for repeated runs:

- pass rate
- average duration
- average assistant message count
- average tool-call count
- average skill-invocation count
- aggregated required-check failures

When `--write-history` is used on `compare`, the benchmark CLI also writes one
official snapshot per compared variant into `ai_evals/history/` and rebuilds:

- `summary.jsonl`
- `rollups/latest.json`
- `rollups/by_surface.json`
- `rollups/by_variant.json`
- `rollups/by_model.json`

The compare output also includes tool usage and invoked skills as diagnostics.

For frontend surfaces, the benchmark CLI shells into a frontend-native Vitest
adapter owned by `ai_evals`, so the evaluation runs with the frontend
project's module resolution and test environment while still producing the
same benchmark result shape as the CLI surface.

That means:

- the benchmark entrypoint remains `ai_evals/cli`
- frontend flow/app fixtures live under `ai_evals/fixtures/frontend/`
- the frontend source tree no longer owns a separate AI chat benchmark suite

By default, the repo only ships `baseline` frontend variants. To compare
distinct prompt variants or write official history snapshots, add a second
manifest under `ai_evals/variants/frontend/<surface>/`.

True efficiency metrics such as latency, token usage, and cost are planned, but
the current CLI does not emit them yet. Until that lands, use `compare`
primarily to answer "did this skill bundle produce better artifacts on the same
cases?"

Variant manifests can also override the top-level project instructions in
addition to the skills bundle:

```json
{
  "id": "candidate",
  "description": "Candidate guidance bundle",
  "skillsSource": {
    "type": "path",
    "path": "./snapshots/candidate-skills"
  },
  "agentsSourcePath": "./snapshots/candidate-AGENTS.md",
  "claudeSourcePath": "./snapshots/candidate-CLAUDE.md"
}
```

That matches the internal `wmill init` override env vars:

```bash
WMILL_INIT_AI_SKILLS_SOURCE=./ai_evals/variants/cli/snapshots/candidate-skills wmill init --use-default
WMILL_INIT_AI_SKILLS_SOURCE=./ai_evals/variants/cli/snapshots/candidate-skills WMILL_INIT_AI_AGENTS_SOURCE=./my-candidate-AGENTS.md wmill init --use-default
```

The `snapshot-variant` command also honors those same env vars. If they are
set when you run the snapshot command, it will freeze that overridden bundle
instead of the generated default.

## Next Steps

Later iterations should add:

- variant cleanup and diff helpers
- token and cost metrics in compare output
- richer history views and filtering
- frontend `script` surface
