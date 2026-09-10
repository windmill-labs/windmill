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

## Module mocks

`mock.module` replaces a module for the **whole process**, and it does reach modules that
were already imported — a stub one file installs lands on a consumer an earlier file
loaded.

Handing the module back in `afterAll` is not a reliable undo. Files do run one at a time
(a root-level `afterAll` completes before the next file's body evaluates), so it looks
like it should be — but stubbing `bundle.ts` and restoring it that way still left
`raw_app_svelte_plugin_unit.test.ts` asserting against an empty bundle, green on Linux
and red on Windows, where the `readdir` file order differs. Treat a stub as permanent for
the run.

So the rule is about what you stub, not how you clean up: **stub only a module no other
in-process suite imports.** Check with `grep -rl "<exported fn>" test/` before reaching
for one. A suite that drives the CLI through a spawned process is out of reach of a
module mock and doesn't count.

`raw_app_push_policy_unit.test.ts` is the worked example: it stubs `gen/services.gen.ts`,
which passes the rule because nothing else in `test/` imports the three API functions it
replaces, and deliberately does not stub `bundle.ts`, which failed it.

## AI Benchmark Caveats

The repo-level benchmark CLI lives under `ai_evals/`, but it currently depends on
mocked frontend flow execution in a few places. Treat `flow` benchmark passes as
artifact-shape signal, not full runtime correctness, when either of these apply:

- deterministic flow validation does not currently reject syntactically invalid
  `rawscript` module bodies
- frontend benchmark calls to `test_run_flow` and `test_run_step` return mocked
  completed jobs for `mock-job-id-*` workspaces instead of executing the flow

If a prompt change depends on flow wiring or script runtime behavior, verify it
with additional validation or a real run before trusting the benchmark result.

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
