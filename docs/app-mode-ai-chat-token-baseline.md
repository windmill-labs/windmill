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

## Follow-up: metadata-only `list_files`

The contentful `get_files` app-mode tool was replaced with `list_files` to make broad app discovery cheaper and less sticky in chat history.

Changes:

- Renamed the overview tool from `get_files` to `list_files`.
- Changed the overview response from truncated source/config contents to metadata only.
- `list_files` returns:
  - frontend files: `path`, character `size`, and file `kind`;
  - backend runnables: `key`, `name`, `type`, and lightweight optional metadata such as `path`, `language`, `contentSize`, and `staticInputKeys`.
- Updated app-mode prompt guidance so the model no longer starts every task with broad file discovery.
- Kept targeted content tools as the path for inspection:
  - `get_frontend_file(path)` for frontend source;
  - `get_backend_runnable(key)` for runnable configuration/source.

The same five cases were rerun with:

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
  --output results/app-token-after-list-files.json
```

Pass rate: **100% (5/5)**

| Case | Prompt tokens | Completion tokens | Total tokens | Tool calls | Tools used |
|---|---:|---:|---:|---:|---|
| `app-token-baseline-large-app-small-edit` | 41,020 | 422 | 41,442 | 3 | `list_files`, `get_frontend_file`, `patch_file` |
| `app-token-selected-large-frontend-context` | 41,020 | 422 | 41,442 | 3 | `list_files`, `get_frontend_file`, `patch_file` |
| `app-token-selected-large-backend-context` | 53,511 | 9,714 | 63,225 | 3 | `list_files`, `get_backend_runnable`, `set_backend_runnable` |
| `app-token-many-datatable-context` | 46,990 | 475 | 47,465 | 3 | `list_files`, `get_frontend_file`, `patch_file` |
| `app-token-large-datatable-discovery` | 131,607 | 5,084 | 136,691 | 8 | `get_datatables`, `list_files`, `set_backend_runnable`, `set_frontend_file`, `patch_file`, `lint` |

Aggregate token usage:

```json
{
  "totalTokenUsage": {
    "prompt": 314148,
    "completion": 16117,
    "total": 330265
  },
  "averageTokenUsagePerAttempt": {
    "prompt": 62829.6,
    "completion": 3223.4,
    "total": 66053
  }
}
```

Comparison against the post-rebase / PR #8922 run (`results/app-token-after-origin-main-pr8922.json`):

| Case | PR #8922 total | `list_files` total | Delta | Delta % | Prompt delta |
|---|---:|---:|---:|---:|---:|
| `app-token-baseline-large-app-small-edit` | 74,061 | 41,442 | -32,619 | -44.0% | -32,522 |
| `app-token-selected-large-frontend-context` | 74,061 | 41,442 | -32,619 | -44.0% | -32,522 |
| `app-token-selected-large-backend-context` | 71,050 | 63,225 | -7,825 | -11.0% | -7,787 |
| `app-token-many-datatable-context` | 35,497 | 47,465 | +11,968 | +33.7% | +11,886 |
| `app-token-large-datatable-discovery` | 97,128 | 136,691 | +39,563 | +40.7% | +38,295 |

Aggregate comparison against the post-rebase / PR #8922 run:

| Metric | PR #8922 | `list_files` | Delta | Delta % |
|---|---:|---:|---:|---:|
| Prompt tokens | 336,798 | 314,148 | -22,650 | -6.7% |
| Completion tokens | 14,999 | 16,117 | +1,118 | +7.5% |
| Total tokens | 351,797 | 330,265 | -21,532 | -6.1% |

Compared to the original baseline above, the `list_files` run is **-50,073 total tokens** (**-13.2% total**).

Interpretation:

- The small edit and selected-frontend cases improved substantially because broad discovery no longer injects truncated contents for the whole app.
- The selected-backend case also improved, despite still needing targeted runnable inspection.
- The datatable-context cases can require an extra `get_frontend_file` after `list_files`, so the small datatable edit regressed in this single-run sample.
- The large datatable case remains dominated by datatable/schema prompt bloat and model variability; moving datatable SDK/reference and schema discovery behind smaller on-demand tools is still the next likely high-impact optimization.
