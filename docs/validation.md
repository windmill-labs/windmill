# Validation Check Matrix

After making changes, run the appropriate checks and fix all errors before considering work done.

## Backend: What to Check

| What changed | Command | Notes |
|---|---|---|
| Core code (no feature gates) | `cargo check` | |
| Enterprise code (`*_ee.rs`) | `cargo check --features enterprise,private` | Also do EE PR workflow (see `docs/enterprise.md`) |
| Enterprise + license-gated code | `cargo check --features enterprise,private,license` | When the feature requires a valid license key |
| Kafka trigger code | `cargo check --features kafka` | |
| Native trigger code | `cargo check --features native_trigger` | |
| Parquet code | `cargo check --features parquet` | |
| Multiple gated modules | `cargo check --features enterprise,parquet` | Combine only the flags you need |
| API route changes | `cargo check` | Then update `openapi.yaml` and run `npm run generate-backend-client` |
| Database migrations | `cargo check` | Test migration applies cleanly with `sqlx migrate run` |

**Never** use `--features all_sqlx_features` — it compiles everything and is very slow. Check `backend/Cargo.toml` `[features]` to find the right flags.

**Never** use `SQLX_OFFLINE=true` — a live database is always available.

After all code changes are done, run `./update_sqlx.sh` from `backend/` to regenerate the offline query cache.

## Frontend: What to Check

| When | Command | Time |
|---|---|---|
| During iteration | `npm run check:fast` | ~2s |
| Final PR validation | `npm run check` | ~50s |
| After backend API changes | `npm run generate-backend-client` first | |

## Cross-Cutting Checks

| Situation | Extra step |
|---|---|
| Added/modified API endpoints | Update `backend/windmill-api/openapi.yaml`, regenerate client |
| Modified Flow structures | Also update `openflow.openapi.yaml` |
| Changed DB schema | Update `backend/summarized_schema.txt` if needed |
| Enterprise file changes | Companion PR in `windmill-ee-private` (see `docs/enterprise.md`) |

## When to Write Tests

- **New utility functions** in `windmill-common`: always add unit tests
- **New API endpoints** with complex logic: add integration test
- **Bug fixes** for non-obvious bugs: add regression test
- **Pure UI changes**: no tests required (rely on type checking)
- **Refactoring**: ensure existing tests pass, don't add new ones

## When to Check Performance

Run `EXPLAIN ANALYZE` on new/modified queries when touching:
- Job queue tables (`v2_job`, `v2_job_completed`)
- Hot-path queries (polling, scheduling)
- Added/removed indexes
