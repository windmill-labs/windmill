# Benchmark CLI

The benchmark CLI is built around saved local results.

The normal loop is:

1. run the suite on the current checkout
2. make your change
3. run again
4. diff the two saved results

## Commands

List cases:

```bash
cd ai_evals
bun run cli -- list-cases
bun run cli -- list-cases --surface flow
```

Run the current checkout and save a result under `ai_evals/results/`:

```bash
cd ai_evals
bun run cli -- run --surface flow --runs 3
bun run cli -- run --surface cli --runs 3
```

Diff two saved results:

```bash
cd ai_evals
bun run cli -- diff-results ai_evals/results/before.json ai_evals/results/after.json
```

Show recent official history:

```bash
cd ai_evals
bun run cli -- history --limit 10
```

Promote one local result into official history:

```bash
cd ai_evals
bun run cli -- promote-result ai_evals/results/latest.json --label main
```

## Frontend Workflow

If you are improving frontend flow/app/script chat, use the surface directly:

```bash
cd ai_evals
bun run cli -- run --surface flow --runs 3
```

Optional frontend overrides:

- `--provider anthropic|openai`
- `--model <model>`
- `--system-prompt-file <path>` to fully replace the system prompt
- `--append-system-prompt-file <path>` to append extra instructions to the
  default system prompt

Example:

```bash
cd ai_evals
bun run cli -- run --surface flow --append-system-prompt-file ./prompt-experiment.md --runs 3
```

The benchmark user prompt still comes from the case manifest. These flags are
for current-checkout experiments, not for a checked-in variant system.

## CLI Guidance Workflow

If you are improving CLI skills or project guidance, run the `cli` surface:

```bash
cd ai_evals
bun run cli -- run --surface cli --runs 3
```

Optional CLI overrides:

- `--skills-source <path>`
- `--agents-source <path>`
- `--claude-source <path>`

Example:

```bash
cd ai_evals
bun run cli -- run --surface cli --skills-source ./system_prompts/auto-generated/skills --runs 3
```

For debugging one CLI case locally, keep the final temp workspace:

```bash
cd ai_evals
bun run cli -- run --surface cli --case bun-hello-script --runs 1 --keep-workspace
```

## Result Files

Each `run` command writes one JSON result file containing:

- run metadata
- aggregate metrics
- per-case summaries
- per-attempt details

Those files are meant for local comparison and are ignored by git.
