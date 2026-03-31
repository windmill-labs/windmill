# Benchmark CLI

This directory is the start of the repo-level benchmark CLI described in
`docs/system-prompt-testing-plan.md`.

## Current Scope

The current implementation is intentionally small:

- `run` command
- `compare` command
- `list-cases` and `list-variants` discovery commands
- `cli` surface only

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

List available CLI variants:

```bash
cd ai_evals
bun run cli -- list-variants --surface cli
```

Run one CLI case:

```bash
cd ai_evals
bun run cli -- run --surface cli --case bun-hello-script --variant baseline
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

Compare two variant selections on one or more cases:

```bash
cd ai_evals
bun run cli -- compare --surface cli --case bun-hello-script --variant baseline --variant baseline --json
```

## Benchmarking A CLI Skill Change

If you change one of the generated CLI skills and want to know whether the
change improved the system, do not compare against the moving `baseline`
variant alone.

`baseline` points at the repo's current generated skills, so it changes when
the repo changes. To make a real before-vs-after comparison, freeze both sides
as path-based variants.

Create a snapshot directory:

```bash
mkdir -p ai_evals/variants/cli/snapshots
```

Before changing the skill, snapshot the current generated skills:

```bash
cp -R system_prompts/auto-generated/skills ai_evals/variants/cli/snapshots/baseline-skills
```

Create a frozen baseline variant manifest in
`ai_evals/variants/cli/baseline-frozen.json`:

```json
{
  "id": "baseline-frozen",
  "description": "Frozen CLI skills before the change",
  "skillsSource": {
    "type": "path",
    "path": "./snapshots/baseline-skills"
  }
}
```

After changing and regenerating the skills, snapshot the candidate:

```bash
cp -R system_prompts/auto-generated/skills ai_evals/variants/cli/snapshots/candidate-skills
```

Create `ai_evals/variants/cli/candidate.json`:

```json
{
  "id": "candidate",
  "description": "CLI skills after the change",
  "skillsSource": {
    "type": "path",
    "path": "./snapshots/candidate-skills"
  }
}
```

Then compare them on one or more cases:

```bash
cd ai_evals
bun run cli -- compare --surface cli --case bun-hello-script --case bun-hello-flow --variant baseline-frozen --variant candidate --json
```

Today, the most meaningful improvement signal is still output quality:

- required artifact checks passing more often
- more cases passing
- fewer required failures across repeated runs

The compare output also includes tool usage and invoked skills as diagnostics.

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

That matches the new `wmill init` overrides:

```bash
wmill init --use-default --ai-skills-source ./ai_evals/variants/cli/snapshots/candidate-skills
wmill init --use-default --ai-skills-source ./ai_evals/variants/cli/snapshots/candidate-skills --ai-agents-source ./my-candidate-AGENTS.md
```

## Next Steps

Later iterations should add:

- `history` command
- frontend adapters
- repeated-run reliability mode
- frozen-variant helper commands
- latency, token, and cost metrics in compare output
- shared result/history writing from this entrypoint
