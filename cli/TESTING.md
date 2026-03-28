# Testing Guide for Windmill CLI

## Running Tests

```bash
# Run unit tests only (fast — no backend, no database, no cargo build)
bun run test:unit

# Run all tests (unit + integration — requires PostgreSQL + cargo)
DATABASE_URL=postgres://postgres:changeme@localhost:5432 bun run test

# Run specific test files
bun test test/sync_pull_push.test.ts
bun test test/workspace_conflicts_unit.test.ts
```

## Test Categories

### Unit tests (`*_unit.test.ts`)

Pure local tests — no backend, no database. Uses `bunfig.unit.toml` (no preload).

Examples: `git_unit`, `lint_command_unit`, `tar_creation_unit`, `workspace_conflicts_unit`

### Integration tests

Require a running backend and PostgreSQL. The `setup.ts` preload builds the backend
binary and starts a shared backend instance.

Examples: `sync_pull_push`, `dev_server`, `standalone_commands`

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `DATABASE_URL` | PostgreSQL connection string (without database name) | `postgres://postgres:changeme@localhost:5432` |
| `TEST_BACKEND` | `cargo` or `docker` | `cargo` |
| `CI_MINIMAL_FEATURES` | `true` for CI mode (zip-only features) | unset |
| `EE_LICENSE_KEY` | Enterprise license for EE feature tests | unset |
| `TEST_FEATURES` | Additional cargo features (comma-separated) | unset |
| `TEST_CLI_RUNTIME` | `node` to test npm package | unset |
| `UNIT_ONLY` | `1` to skip backend setup in preload (used by `test:unit`) | unset |
| `VERBOSE` | `1` for backend process output | unset |

## Cleanup

Stale test databases (`windmill_test_*`) and orphaned backend processes from
previous crashed runs are automatically cleaned up when starting a new test run.

To manually check for leftovers:

```bash
# Check for stale test databases
psql postgres://postgres:changeme@localhost:5432/postgres -c \
  "SELECT datname FROM pg_database WHERE datname LIKE 'windmill_test_%';"

# Check for orphaned backend processes
ps aux | grep "target/debug/windmill" | grep -v grep
```
