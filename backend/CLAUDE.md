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
- **Running data pipelines (DuckLake) from source**: see the section below — a plain build
  advertises the `duckdb` tag but cannot execute DuckDB scripts and has no working S3 proxy.

## Running data pipelines (DuckLake) from source

DuckLake pipelines need **both** the right cargo features **and** the prebuilt DuckDB FFI. A
plain `cargo run` (or `cargo run --features quickjs`) does **not** suffice, and the failure modes
are silent-ish, so agents lose time. Verify feature names against `backend/Cargo.toml` `[features]`.

**Feature sets** (run from `backend/`):

| Goal | Command |
|---|---|
| CE DuckLake (DuckDB scripts + S3 proxy) | `cargo run --features quickjs,duckdb,parquet,private` |
| + Python scripts | add `,python` |
| EE features (WAP, partitioning, forks, …) | add `,enterprise,license` |

`enterprise` already pulls in `license`, but list both when you want the license-gated paths.
`quickjs` is for JS eval, not DuckLake per se — keep it if your baseline build had it.

**Before running any DuckDB script**, build the FFI (see the bullet above):
`cd backend/windmill-duckdb-ffi-internal && ./build_dev.sh`.

**Two gotchas that a wrong feature set produces:**

1. **`duckdb` tag advertised, feature missing.** The `duckdb` worker tag is in the *unconditional*
   default tag list (`windmill-common/src/worker.rs`, `DEFAULT_TAGS`), so a worker advertises it even
   without the `duckdb` feature. Jobs then dispatch but fail at execution with
   `"Duck DB requires the duckdb feature to be enabled"` (`windmill-worker/src/worker.rs`). Fix:
   compile with `--features duckdb`.
2. **DuckLake writes 404 (no S3 proxy).** The workspace S3 proxy (`/w/{ws}/s3_proxy/*`) that
   DuckLake uses for reads/writes only mounts the real service under
   `#[cfg(all(feature = "private", feature = "parquet"))]` (`windmill-api/src/s3_proxy_oss.rs`);
   otherwise it's an empty router and every proxied request 404s. Fix: compile with **both**
   `private` and `parquet`.

## Cloud vs self-hosted gating

The `cloud` cargo feature is compiled into **all** EE builds, so `#[cfg(feature = "cloud")]` is **not** a "cloud-only" runtime gate — it only means the code is present. The real gate for behavior specific to the managed cloud (app.windmill.dev) is the runtime flag `*CLOUD_HOSTED` (`windmill_common::worker::CLOUD_HOSTED`, from the `CLOUD_HOSTED` env var; note it's loaded from `.env` via `dotenv`, so it won't show in `/proc/<pid>/environ` — check the running behavior, not the exec env).

Cloud-only logic must be behind `if *CLOUD_HOSTED { ... }`: feature-gate the helper so it compiles, then **runtime-gate the call**. `#[cfg(feature = "cloud")]` on its own is only sufficient for:
- pure helper/struct definitions (they only run when a gated caller invokes them),
- code already inside an `if *CLOUD_HOSTED { ... }` block,
- handlers that early-return on `!*CLOUD_HOSTED`,
- idempotent no-ops that are harmless off-cloud (e.g. cache invalidation).
