# Backend test suite: cost, coverage and the plan to trim it

Review of every `cargo test` target the "Backend only integration tests" workflow runs
(`.github/workflows/backend-test.yml`), made on 2026-10-17 against CI run 39017583576. Items carry
an ID (`S1`, `F3`, `C2`, ...) so pull requests can say which ones they handle. Seconds are CI wall
time unless stated otherwise; estimates marked "est." have not been measured.

## How the suite spends its time

The `cargo test` step took 24.5 min of its 30 min `timeout-minutes`: about 8 min compiling and
linking, about 16.4 min running tests.

- **Every file in a `tests/` directory is its own binary**, and each links the whole backend.
  There are about 240 of them (`backend/tests` 123, `windmill-api-integration-tests` 69,
  `windmill-common` 25, `windmill-queue` 9, the rest spread across crates).
- **Cargo runs the 391 test binaries one after another.** Only tests inside one binary run in
  parallel (`--test-threads=10`). 207 binaries finish in under 1.5 s each, and 115 run zero tests
  (84 of those are empty doctest runs).
- **Every `#[sqlx::test]` creates a database and applies all 683 migrations** (our sqlx fork,
  `sqlx-postgres/src/testing/mod.rs`, has no template database). That is about 1.3 s per test
  locally, against 0.17 s to clone a migrated template. There are about 1,336 such tests. In the
  biggest binaries tests complete in waves of 10 every 2.8 to 4.5 s whatever their bodies do, so
  setup, not test logic, sets the pace.
- **The per-test pools of one binary share a 20-connection parent pool.** A test that holds a
  connection while waiting for a second one (the `create_script` hold-and-wait fixed in #11843)
  can starve its siblings into the 30 s acquire timeout.

What makes a test cheap here, and what to keep in mind when adding one:

- Put a new test in an existing binary of the same domain rather than a new file: a new file
  costs a full link plus a 1.2 to 1.5 s floor on every CI run.
- Do not wait out real time to prove something. Poll for the state you expect
  (`pg_stat_activity` for a lock wait, `v2_job` for a queued job), and avoid real dependency
  installs when the assertion does not need them.
- `#[serial]` turns a binary's parallel tests into a sequence; reach for it only when the test
  mutates process-global state, and prefer giving that state a per-test key.
- A test that is skipped by an env-var check inside its body still pays for its database and
  reports a pass. Use `#[ignore = "reason"]` instead, so the skip is visible and free.

## Structural items (largest savings)

| ID | Change | Est. saving |
|---|---|---|
| S1 | Create each `#[sqlx::test]` database from a migrated template in the sqlx fork (one template per migrator, `CREATE DATABASE .. TEMPLATE`), instead of migrating every test DB | several minutes, to be measured |
| S2 | Fewer, larger test binaries: one or two per crate (`autotests = false` + `tests/main.rs` with `mod` per file), or cargo-nextest for cross-binary parallelism. Files that mutate process globals (`MCP_DISABLE_TOKEN_QUERY_PARAM`, `MIN_VERSION`, `CLOUD_HOSTED`, `LICENSE_KEY_ID`, operator rights cache, `set_var`) go to an "isolated" binary or restore what they change. `windmill-api-integration-tests` alone: ~80 s plus 67 links | 2 to 4 min est. |
| S3 | `[lib] doctest = false` on the 83 crates with no doctests (only `windmill-common` has any) | ~14 s |
| S4 | `[lib] test = false` on the 25 crates whose `src/` has no `#[test]` (alerting, api-assets, api-client, api-flow-conversations, api-inputs, api-integration-tests, api-openapi, api-schedule, api-sse, api-workers, autoscaling, macros, native-triggers, parser-nu, parser-py-imports, parser-ts, parser-wac, sql-datatype-parser-wasm, test-utils, trigger-azure, trigger-email, trigger-kafka, trigger-mqtt, trigger-nats, trigger-sqs) | 20 to 60 s of compile/link est. |

## Fast wins on single files

| ID | File | Problem | Fix | Est. saving |
|---|---|---|---|---|
| F1 | `windmill-queue/tests/concurrency_counter_test.rs` | `long_haul_stress` is 30 of the binary's 33 s (500 x 100 admits on a pre-seeded row; `stress_never_over_admits` pins the #11587 missing-row path). `bench_over_limit_storm` repeats `bail_path_no_admits_when_full` | `#[ignore]` both as manual soak/bench | ~30 s |
| F2 | `backend/tests/worker.rs` | 13 `#[serial(pg_cache)]` Postgres tests form the binary's critical path. `test_postgresql_arg_type_combinations` (33 jobs) and `test_postgresql_custom_types_on_cached_connection` (21 jobs) start a fresh worker per job via `run_until_complete` (~23 s and ~8 s) | Push all cases, drain them with one `in_test_worker`, as `test_postgresql_100_jobs_cached` does | 15 to 25 s |
| F3 | `backend/tests/workspace_dependencies.rs` | `hub_sync_blacklist_from_workspace_deps` waits for a real `bun install express`; the assertions only need the enqueued job. `basic_manual_named` is ignored and asserts nothing | Assert on the queued dependency job (or an empty dependency set); delete the dead test | ~15 s |
| F4 | `backend/tests/dedicated_workers.rs` | 30 `#[serial]` tests at ~2 s each (global `WORKER_CONFIG`). Six flow-shape tests pay worker start-up only to vary the path shape | Run the flow shapes under one `in_test_worker_dedicated`; longer term make the dedicated list per worker | ~9 s (up to ~45 s) |
| F5 | `backend/tests/ai_sessions.rs` | The round-trip test pushes about 36 MB through unoptimised crypto and serde to exceed `PULL_RESPONSE_BUDGET` | A test override for the budget and caps | 10 to 14 s est. |
| F6 | `backend/tests/ai_tool_queue.rs` | All 4 tests `#[serial]` with no stated reason; one waits out a 3 s agent timeout | Verify and drop `serial`; shorter timeout | ~9 s est. |
| F7 | `windmill-api-integration-tests/tests/jobs_unauthed.rs` | Calls `cancel_persistent`, which sleeps 5 s in the product, only to prove the route matches | Send that request without auth (400 before the sleep) | ~5 s |
| F8 | `windmill-trigger-websocket/tests/connect_retry.rs` | Waits out real 2 s + 4 s backoff | Make the base delay overridable for tests | ~6 s |
| F9 | `windmill-api-integration-tests/tests/nested_dev_workspace.rs` | Asserts a 5 s timeout elapses to show archive blocks | Poll `pg_stat_activity` for the lock wait, then release and assert success | ~4 s |
| F10 | `backend/tests/instance_config.rs` | 48 DBs for small `apply_*` / `from_db_*` cases, partly duplicated by the unit tests in `windmill-common/src/instance_config.rs` | Table-drive into about 25 tests | 7 to 9 s est. |
| F11 | `backend/tests/job_payload.rs` | 8 script-only tests run 3 times through a `MIN_VERSION` matrix that only gates flow payloads; the 3 `test_dependencies_payload_min_*` are identical; the matrix mutates a global shared by parallel tests | Matrix only for flow/restart tests, made deterministic | 4 to 7 s |
| F12 | `backend/tests/flow_engine_parity.rs` | The deno_core vs QuickJS parity it names no longer exists; 60 Bun subprocess steps only echo inputs | In-process steps, rename | 4 to 8 s est. |

## Correctness: flakes, fake passes, tests that never run

| ID | Item | Action |
|---|---|---|
| C1 | `backend/tests/error_handler.rs` still has the shared deployed-script-cache race and the 500 ms fixed sleeps that #11691 fixed only in `success_handler.rs`; `test_error_handler_triggered_on_failure` failed in about 3 of 31 recent runs | Apply the `success_handler.rs` fix (disable the cache, poll), merge the two files |
| C2 | Runtime-skipped tests report a pass: `windmill-common/tests/secret_backend_integration.rs` (9), `aws_sm_integration.rs` (9), `azure_kv_integration.rs` (4), `windmill-api-integration-tests/tests/fork_secret_replication_azure.rs` (1) | `#[ignore = "requires ..."]`, run them with `--ignored` where the service exists |
| C3 | Never compiled in CI because the feature is missing from the CI list: `otel.rs` (16, `otel`), `agent_workers.rs` (11, `agent_worker_server`), `operator_db_sync.rs` (18, `operator`), the `failpoints` savepoint test in `worker.rs`, `billing_workspace.rs` (`cloud`), the `websocket`-gated tests in `preserve_on_behalf_of.rs` and `trigger_listener_queries.rs` (the websocket crate is an unconditional dependency, so the cfg may simply be dropped) | Decide per suite: add the feature to CI or delete the tests |
| C4 | `#[ignore]` with a stale reason: `windmill-api-integration-tests/tests/permissions.rs` (4), `windmill-common/tests/low_disk_alerts.rs`, `windmill-common/tests/sqlx_begin_cancel_safe.rs` (the only guard for the sqlx `[patch]`), `windmill-runtime-nativets/tests/otel_e2e.rs` | Un-ignore and fix, or delete |
| C5 | Never run anywhere: `parsers/windmill-parser-wasm/tests/wasm.rs` (crate excluded from the workspace), `windmill-api-integration-tests/tests/trigger_e2e.rs` (external brokers), R/Nu/Java tests in `worker.rs` | Delete, or move unique cases to native tests |
| C6 | Flake risks: `job_payload.rs` `MIN_VERSION` matrix (F11); `bun_jobs.rs` private-registry tests mutate global npm config under ~60 parallel installs; `relock_skip.rs` counts completions in time windows; fixed sleeps in `v2_job_delete_orphans.rs:351` and `backend/tests/ci_tests.rs:90`; `token_hash.rs` sets `MIN_VERSION` under parallel tests; `worker.rs` tag tests swap `CUSTOM_TAGS_PER_WORKSPACE` while unserialised tests push jobs | Fix when touching the file |

## Removals (duplicates and tests that assert nothing)

| ID | Item |
|---|---|
| R1 | `backend/tests/protection_rules.rs::test_protection_rules` duplicates the larger copy in `windmill-api-integration-tests/tests/protection_rules.rs` (left behind by the subcrate move); move the file's other 4 tests next to it |
| R2 | `windmill-api-integration-tests/tests/triggers.rs`: 32 of 37 tests insert a row and read it back without calling Windmill code; keep the 5 `test_capture_api_*` |
| R3 | `windmill-api-integration-tests/tests/raw_apps.rs` and `service_logs.rs` (2xx-only, covered by `raw_apps_list_scope.rs` and `wm_token_confinement.rs:578`) |
| R4 | `worker.rs` `test_relative_imports_bun` / `test_nested_imports_bun` (covered by `bun_jobs.rs`); `backend/tests/nativets_jobs.rs` (keep its NUL case in `worker.rs`) |
| R5 | `backend/tests/volume_tests.rs`: 9 tests that only run hand-written SQL plus 2 struct-literal tests |
| R6 | `error_handler.rs::test_error_handler_settings`, `success_handler.rs::test_success_handler_cache` / `_settings` (jsonb round trips); `configs.rs::test_no_alert_entries_in_config_table_after_migration` (asserts nothing on a fresh DB) |
| R7 | Dead files: `windmill-common/tests/instance_group_auto_add.rs` (asserts its own simulation, all ignored), `secret_backend_migration.rs` (duplicates `secret_backend_integration.rs`), `windmill-queue/tests/job_cleanup_test.rs` |
| R8 | `workspace_fairness.rs`: `fairness_lifts_when_load_drops` duplicate, `fairness_pull_query_skips_capped_workspace` (asserts SQL written in the test), the two near-vacuous ignored tests and the ignored simulations |
| R9 | Commented-out tests (`dependency_map.rs`, `windmill-macros/tests/annotations.rs` incl. the empty `hash_collision`, parser-nu) and 8 unused fixtures in `backend/tests/fixtures/` (`djob_debouncing.sql`, `workspace_dependencies.sql`, `instance_group_auto_add.sql`, `secret_backend.sql`, `schedule_push.sql`, `permissions_test.sql`, `variables_test.sql`, `resources_test.sql`) |

Security regression tests (GHSA, IDOR, SSRF, token confinement) were all kept: each pins a fixed
bug, and their cost is mostly the per-binary floor that S1 and S2 remove.

## Merges (each saves a binary floor and a link)

Grouped by shared setup: the 10 small `app_*` / `raw_app*` files into one apps-authz binary; the
`jobs_*` / `job_token_*` / `inline_preview_auth` / `run_lineage_auth` / `preview_restart_source_access`
files; the 3 `mcp_*` files; the 4 S3 authz files; the 6 workspace-scope files; the 4 `wac_*` files;
`restart_*` into `zombie_flow_recovery.rs`; the perpetual pair; the 5 `drafts_*` files into
`windmill-api-integration-tests/tests/drafts.rs`; the 3 `git_sync_*` files; the 3 datatable
migration files; the 5 `fork_*` integration files; the 7 route-smoke files from the axum 0.8 upgrade
(`health`, `group_history`, `folder_history`, `granular_acls`, `job_metrics`, `favorites`,
`capture_unauthed`); the 13 one-to-four-test `windmill-common` files; the 3 small queue-depth files.
Files that must stay alone: `app_guest_cloud_hosted.rs` (`CLOUD_HOSTED`), `datatable_fork_migrations.rs`
(`DATABASE_URL`), `response_timeout_env.rs` (`LazyLock`), `dependency_job_tag.rs`, `mcp_token_query_param.rs`.
S2 makes most of these moot.

## Maintainability

`authed()` / `client()` reqwest helpers are redefined in about 77 test files, a hand-built
`NewScript { .. }` literal is copied across files, and `worker.rs` repeats a 13-field
`JobPayload::Code(RawCode { .. })` literal 41 times. Shared builders in `windmill-test-utils`
would remove most of that and most of the churn when a field is added.
