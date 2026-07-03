# Backend (Rust)

- **Coding patterns**: MUST use the `rust-backend` skill when writing Rust code
- **Validation**: `docs/validation.md` — which `cargo check` flags to use
- **Enterprise**: `docs/enterprise.md` — EE file conventions and PR workflow
- **DB schema**: `backend/summarized_schema.txt`
- **API routes entry point**: `windmill-api/src/lib.rs`
- **OpenAPI spec**: `windmill-api/openapi.yaml`
- **DuckDB local jobs**: build the dynamic FFI library before running DuckDB scripts locally:
  ```bash
  cd backend/windmill-duckdb-ffi-internal && ./build_dev.sh
  ```
  Re-run after clean builds or when `target/debug/libwindmill_duckdb_ffi_internal.*` is missing.

## Cloud vs self-hosted gating

The `cloud` cargo feature is compiled into **all** EE builds, so `#[cfg(feature = "cloud")]` is **not** a "cloud-only" runtime gate — it only means the code is present. The real gate for behavior specific to the managed cloud (app.windmill.dev) is the runtime flag `*CLOUD_HOSTED` (`windmill_common::worker::CLOUD_HOSTED`, from the `CLOUD_HOSTED` env var; note it's loaded from `.env` via `dotenv`, so it won't show in `/proc/<pid>/environ` — check the running behavior, not the exec env).

Cloud-only logic must be behind `if *CLOUD_HOSTED { ... }`: feature-gate the helper so it compiles, then **runtime-gate the call**. `#[cfg(feature = "cloud")]` on its own is only sufficient for:
- pure helper/struct definitions (they only run when a gated caller invokes them),
- code already inside an `if *CLOUD_HOSTED { ... }` block,
- handlers that early-return on `!*CLOUD_HOSTED`,
- idempotent no-ops that are harmless off-cloud (e.g. cache invalidation).
