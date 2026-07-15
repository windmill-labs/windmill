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
  The bundled DuckDB compile (~2min) is cached in a per-user dir shared across
  worktrees (keyed by the crate's `Cargo.lock`), so a fresh worktree reuses it and
  the build is near-instant — you don't pay the full compile per worktree. Editing
  the FFI crate's own source falls back to an isolated per-worktree `./target`.
- **Running data pipelines (DuckLake) from source**: see the section below — a plain build
  advertises the `duckdb` tag but cannot execute DuckDB scripts and has no working S3 proxy.

## Cargo features & running the dev backend

The dev backend runs under `cargo watch` and is launched by default with **only
`--features quickjs`** (see the tmux backend pane). That baseline compiles fast but
**deliberately omits most functionality** — notably S3/object storage, the S3 proxy, all
EE code, MCP, and every non-JS language runtime. A running server never gains a feature you
didn't compile in: feature-gated routes 404 or return a `"requires <feature>"` stub. So if
you touch code behind a feature gate, or need to *exercise* such a feature at runtime, you
MUST **restart the backend with the appropriate features** for what you're working on.

### Restarting the dev backend with the right features

The backend runs in tmux pane 1 as `cargo watch -x "run --features <…>"`. To restart it with a
different feature set — scope kills by pid/cwd, **never** `pkill -f target/debug/windmill` (it
kills every sibling worktree's backend):

1. Stop the current run: `tmux send-keys -t <pane1> C-c`, then kill *this worktree's*
   `cargo-watch` pid (find it via `/proc/<pid>/cwd`).
2. Relaunch in the same pane so it inherits the shell's `DATABASE_URL` etc.; the pane env's
   `PORT` may be stale, so set it explicitly:
   ```bash
   export PORT=$BACKEND_PORT
   cargo watch -x "run --features enterprise,private,parquet,quickjs"
   ```
3. Wait for `health check completed` in the pane before hitting the API.

cargo-watch only re-runs on a file change, so after an idle/failed run `touch README.md` (from
`backend/`, where the watch runs) is a cheap retrigger (touching a `.rs` forces a full rebuild).

### What each feature gate does (the ones you'll actually toggle)

`backend/Cargo.toml` `[features]` is the source of truth; this is the practical dev map. Combine
only what you need — build time scales with the set.

| Feature | Enables | Need it for |
|---|---|---|
| `quickjs` | Embedded JS engine for inline JS eval (the default dev baseline). | Keep in every dev set. |
| `private` | Compiles the `*_ee.rs` files (symlinked from `windmill-ee-private`). Gates **all** EE code, including the real S3 helpers, the S3 proxy, and advanced S3 permission checks. | Any EE code path, S3/object storage. |
| `enterprise` | EE business logic (autoscaling, SAML hooks, advanced S3 rule **enforcement**, WAP, forks, …). Pulls in `license`. | Running EE features. Advanced S3 permission rules only take effect with this. |
| `license` | License-key/plan plumbing (`LICENSE_KEY`). Pulled in by `enterprise`. Having the feature compiled does **not** require a license *key* at runtime — CE defaults to a free plan and most EE paths still run keyless. | License-gated behavior. |
| `parquet` | S3/object-storage support: the `job_helpers/*` and `apps_u/*` S3 endpoints, parquet/CSV preview, workspace large-file storage. Without it those routes return `"requires parquet"`. | Anything touching S3/object storage or datasets. |
| `duckdb` | DuckDB script executor (also needs the FFI dylib — see above). | DuckDB scripts, DuckLake. |
| `python` `rust` `php` `java` `ruby` `csharp` `nu` `deno_core` `mysql` `mssql` `bigquery` `oracledb` `rlang` | Each enables that language/DB runtime for job execution. | Running jobs in that language. |
| `mcp` | MCP gateway routes (baseline `quickjs` does NOT include it → MCP routes 404). | MCP work. |
| `websocket` `http_trigger` `kafka` `nats` `mqtt_trigger` `sqs_trigger` `gcp_trigger` `azure_trigger` `postgres_trigger` `native_trigger` | Each native trigger kind; none on by default (creating one 404s without its feature). | Working on / exercising that trigger. |
| `no_auth` | Treats every request as an admin superadmin (`CLOUD_HOSTED`-guarded). | Local auth-free experiments only. |

Convenience bundles (`ce`, `ee`, `oss`, …) exist in `[features]` but are heavy — prefer the
minimal explicit set for dev.

**Common combinations** (run from `backend/`):

| Goal | `--features` |
|---|---|
| Plain dev baseline (JS eval only) | `quickjs` |
| S3 / object storage / datasets (CE) | `quickjs,private,parquet` |
| S3 + EE (advanced S3 rules, on-behalf app reads, WAP, forks) | `quickjs,enterprise,private,parquet` |
| DuckLake / DuckDB (CE) | `quickjs,duckdb,parquet,private` (+ build the FFI) |
| + Python jobs | append `,python` |

## Workspace object storage in dev — use the local filesystem

For a dev workspace you don't need MinIO/S3: use the built-in **`FilesystemStorage`** large-file
storage (a root path on local disk). It is intentionally hidden from the settings-UI storage
dropdown (dev-only), so set it via the API. Requires the backend built with `parquet` (+ `private`
for the real S3 helpers, + `enterprise` if you want advanced permission rules enforced):

```bash
curl -X POST "$BASE/api/w/<ws>/workspaces/edit_large_file_storage_config" \
  -H "Authorization: Bearer <admin-token>" -H "Content-Type: application/json" \
  -d '{"large_file_storage":{"type":"FilesystemStorage","root_path":"/abs/writable/dir",
       "public_resource":false,"advanced_permissions":null,"secondary_storage":{}}}'
```

Optional `advanced_permissions` (EE) is a list of `{"pattern":"<glob>","allow":"read[,write,delete,list]"}`
rules: admins bypass them, non-admins are confined to matching grants. Uploads/reads then flow
through the normal `job_helpers/*` (viewer-scoped) and `apps_u/*` (app-author on-behalf) S3
endpoints. Caveat: direct DuckDB access rejects filesystem stores (`"Filesystem is not supported
in DuckDB"`) — DuckLake/datatable go through the S3 proxy instead, which works.

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
