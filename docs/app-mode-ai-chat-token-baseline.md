# App Mode AI Chat Token Baseline

This baseline was collected before optimizing app-mode context/prompt/datatable behavior.

## Command

Secrets were loaded from `~/windmill/ai_evals/.env` without printing them.

```bash
cd ai_evals
set -a
source ~/windmill/ai_evals/.env
set +a
bun run cli -- run app \
  app-token-baseline-large-app-small-edit \
  app-token-selected-large-frontend-context \
  app-token-selected-large-backend-context \
  app-token-many-datatable-context \
  app-token-large-datatable-discovery \
  --model haiku \
  --runs 1 \
  --output results/app-token-baseline-current-max8.json
```

## Environment

- Mode: `app`
- Model under test: `anthropic:claude-haiku-4-5-20251001`
- Transport: `direct`
- Judge model: `claude-sonnet-4-6`
- Runs per case: `1`
- Token-heavy app cases use `runtime.maxTurns: 8`

## Results

Pass rate: **100% (5/5)**

| Case | Prompt tokens | Completion tokens | Total tokens | Tool calls | Tools used |
|---|---:|---:|---:|---:|---|
| `app-token-baseline-large-app-small-edit` | 73,682 | 519 | 74,201 | 4 | `get_files`, `get_frontend_file`, `patch_file` |
| `app-token-selected-large-frontend-context` | 36,305 | 348 | 36,653 | 2 | `get_frontend_file`, `patch_file` |
| `app-token-selected-large-backend-context` | 95,232 | 19,633 | 114,865 | 4 | `set_backend_runnable`, `get_backend_runnable` |
| `app-token-many-datatable-context` | 35,204 | 404 | 35,608 | 2 | `get_files`, `patch_file` |
| `app-token-large-datatable-discovery` | 114,964 | 4,047 | 119,011 | 7 | `get_files`, `get_datatables`, `set_backend_runnable`, `set_frontend_file`, `patch_file`, `lint` |

Aggregate token usage:

```json
{
  "totalTokenUsage": {
    "prompt": 355387,
    "completion": 24951,
    "total": 380338
  },
  "averageTokenUsagePerAttempt": {
    "prompt": 71077.4,
    "completion": 4990.2,
    "total": 76067.6
  }
}
```

## Interpretation

The highest-token cases are:

1. `app-token-large-datatable-discovery` — full datatable discovery with `get_datatables()` and app edits reached **119,011** total tokens.
2. `app-token-selected-large-backend-context` — selected large backend runnable plus a rewrite-style tool call reached **114,865** total tokens.
3. `app-token-baseline-large-app-small-edit` — a trivial heading edit still reached **74,201** total tokens, largely due broad file discovery.

These cases should be rerun after prompt/context/tool changes to compare total and prompt-token reductions.
