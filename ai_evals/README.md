# AI Evals

Repo-level benchmark suite for the Windmill CLI guidance and the frontend AI
chat surfaces.

## What It Is For

The default workflow is:

1. run the benchmark on the current checkout
2. save the local result JSON
3. make your change
4. run again
5. diff the two result files
6. optionally promote the good run into official history

This keeps local development focused on before/after comparisons instead of
named prompt variants.

## Entry Point

Install once:

```bash
cd ai_evals
bun install
```

Main commands:

```bash
cd ai_evals
bun run cli -- list-cases
bun run cli -- run --surface flow --runs 3
bun run cli -- diff-results ai_evals/results/before.json ai_evals/results/after.json
bun run cli -- history --limit 10
```

## Surfaces

- `cli`: benchmark the CLI skills / AGENTS / CLAUDE guidance path
- `flow`: benchmark frontend flow chat
- `app`: benchmark frontend app chat
- `script`: benchmark frontend script chat

## Layout

- `cli/`: benchmark CLI entrypoint
- `cases/`: eval manifests
- `fixtures/`: initial and expected frontend artifacts
- `history/`: tracked official benchmark history
- `results/`: local run outputs written by `run` and ignored by git

## Notes

- Frontend runs reuse the production frontend chat code through the Vitest
  adapter under `ai_evals/adapters/frontend/`.
- CLI runs create an isolated workspace and write the current checkout's
  guidance into it before running the benchmark prompt.
- Official history is separate from local experimentation. Use
  `bun run cli -- promote-result ...` only for runs you want to preserve.
