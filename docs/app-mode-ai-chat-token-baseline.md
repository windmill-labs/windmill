# App Mode AI Chat Token Baseline

This baseline was collected before optimizing app-mode context/prompt/datatable behavior.

> Note: The historical commands/results below include `app-token-selected-large-frontend-context` and `app-token-selected-large-backend-context`. Those cases were removed from the active eval suite because `runtime.appContext.selected` only verified that the file/runnable existed and did not serialize a selected file/runnable hint to the model. Future selected-file/runnable coverage should be reintroduced through the app context manager path.

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

## Follow-up: targeted datatable tools and shorter datatable prompt

The next pass reduced default datatable context by making datatable discovery metadata-first and removing the full datatable SDK reference from the system prompt.

Changes:

- Replaced the broad schema discovery tool with `list_datatables()` for datatable/schema/table names only.
- Added `get_datatable_table_schema(datatable_name, schema_name, table_name)` for targeted column lookup when column names/types are actually needed.
- Removed the full TypeScript + Python datatable SDK reference from the default app system prompt.
- Kept concise TypeScript and Python datatable examples in the prompt, which were enough for the benchmark cases.
- Strengthened prompt/tool guidance so table-list dashboards use `list_datatables()` directly and avoid schema/SDK lookups unless needed.

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
  --output results/app-token-after-datatable-tools-v3.json
```

Pass rate: **100% (5/5)**

| Case | Prompt tokens | Completion tokens | Total tokens | Tool calls | Tools used |
|---|---:|---:|---:|---:|---|
| `app-token-baseline-large-app-small-edit` | 37,516 | 425 | 37,941 | 3 | `list_files`, `get_frontend_file`, `patch_file` |
| `app-token-selected-large-frontend-context` | 37,516 | 358 | 37,874 | 3 | `list_files`, `get_frontend_file`, `patch_file` |
| `app-token-selected-large-backend-context` | 49,995 | 9,708 | 59,703 | 3 | `list_files`, `get_backend_runnable`, `set_backend_runnable` |
| `app-token-many-datatable-context` | 43,493 | 536 | 44,029 | 3 | `list_files`, `get_frontend_file`, `patch_file` |
| `app-token-large-datatable-discovery` | 24,193 | 2,043 | 26,236 | 4 | `list_datatables`, `list_files`, `get_frontend_file`, `set_frontend_file` |

Aggregate token usage:

```json
{
  "totalTokenUsage": {
    "prompt": 192713,
    "completion": 13070,
    "total": 205783
  },
  "averageTokenUsagePerAttempt": {
    "prompt": 38542.6,
    "completion": 2614,
    "total": 41156.6
  }
}
```

Comparison against the metadata-only `list_files` run (`results/app-token-after-list-files.json`):

| Case | `list_files` total | Datatable-tools total | Delta | Delta % | Prompt delta |
|---|---:|---:|---:|---:|---:|
| `app-token-baseline-large-app-small-edit` | 41,442 | 37,941 | -3,501 | -8.4% | -3,504 |
| `app-token-selected-large-frontend-context` | 41,442 | 37,874 | -3,568 | -8.6% | -3,504 |
| `app-token-selected-large-backend-context` | 63,225 | 59,703 | -3,522 | -5.6% | -3,516 |
| `app-token-many-datatable-context` | 47,465 | 44,029 | -3,436 | -7.2% | -3,497 |
| `app-token-large-datatable-discovery` | 136,691 | 26,236 | -110,455 | -80.8% | -107,414 |

Aggregate comparison:

| Metric | `list_files` | Datatable tools | Delta | Delta % |
|---|---:|---:|---:|---:|
| Prompt tokens | 314,148 | 192,713 | -121,435 | -38.7% |
| Completion tokens | 16,117 | 13,070 | -3,047 | -18.9% |
| Total tokens | 330,265 | 205,783 | -124,482 | -37.7% |

Compared to the post-rebase / PR #8922 run, the datatable-tools run is **-146,014 total tokens** (**-41.5% total**). Compared to the original baseline above, it is **-174,555 total tokens** (**-45.9% total**).

Interpretation:

- Removing the full datatable SDK reference from the default prompt saved about 3.5k prompt tokens in every case.
- The large datatable discovery case improved dramatically because the model used `list_datatables()` table-name metadata instead of loading full schemas.
- The small datatable-context edit is still higher than the post-rebase / PR #8922 run because selected file identifiers are not yet injected, so the model still discovers and reads `/index.tsx` before patching.
- A future context-manager-backed selected file/runnable flow should add cheap selected identifiers when that UX is ready, so selected-file tasks can skip `list_files()` without reintroducing implicit source-content bloat.
