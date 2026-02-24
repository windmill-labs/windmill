# Extract S3/Object Store into dedicated crates

## Goal

Extract all S3/object-store infrastructure from `windmill-common/src/s3_helpers.rs` (and related EE modules) into two crates:
- **`windmill-types/src/s3.rs`** — pure type definitions (no I/O, no DB)
- **`windmill-object-store/`** — new crate with S3 client building, settings management, upload/download, helpers

Delete `windmill-common/src/s3_helpers.rs` entirely. All consumers import types from `windmill_types::s3` and functions from `windmill_object_store` directly.

## Dependency graph (no cycles)

```
windmill-types                    (S3 types — no I/O, no DB)
    ^
windmill-common                   (DB, errors, global_settings — no more s3_helpers module)
    ^
windmill-object-store             (S3 clients, settings, upload/download)
    ^
windmill-api, windmill-worker, backend root, windmill-api-settings, etc.
```

---

## Step 1 — Move S3 types to `windmill-types/src/s3.rs`

Move all struct/enum/constant/bitflag definitions from `windmill-common/src/s3_helpers.rs` into a new file `windmill-types/src/s3.rs`. These are pure data types with serde derives — no functions that do I/O or DB access.

Types to move:
- `LargeFileStorage` (enum)
- `S3Storage`, `AzureBlobStorage`, `GoogleCloudStorage` (structs)
- `S3PermissionRule` (struct), `S3Permission` (bitflags)
- `ObjectStoreResource` (enum), `StorageResourceType` (enum)
- `S3Resource`, `AzureBlobResource`, `GcsResource`, `S3AwsOidcResource` (structs)
- `S3Object` (struct)
- `BundleFormat` (enum)
- `ObjectStoreSettings` (enum), `ObjectSettings` (enum)
- `FilesystemSettings`, `S3Settings` (structs)
- `DuckdbConnectionSettingsResponse`, `DuckdbConnectionSettingsQueryV2` (structs)
- `ExpirableObjectStore` (struct), `ObjectStoreRefresh` (struct), `ObjectStoreReload` (enum)

Add `pub mod s3;` to `windmill-types/src/lib.rs`.

Check if `windmill-types/Cargo.toml` needs new dependencies for these types (likely `bitflags` for `S3Permission`, and possibly `object_store` behind a feature flag if `ExpirableObjectStore` wraps `Arc<dyn ObjectStore>`). Keep `object_store` optional/feature-gated if so.

## Step 2 — Create `windmill-object-store` crate

### Crate structure

```
backend/windmill-object-store/
+-- Cargo.toml
+-- src/
    +-- lib.rs                   # module declarations + re-exports
    +-- settings.rs              # OBJECT_STORE_SETTINGS static, reload_object_store_setting, get_object_store, get_workspace_object_store
    +-- client.rs                # build_object_store_client, build_object_store_from_settings, build_s3_client, build_s3_client_from_settings
    +-- helpers.rs               # attempt_fetch_bytes, upload_artifact_to_store, get_etag_or_empty, render_endpoint, convert_json_line_stream, bundle, raw_app, lfs_to_object_store_resource
    +-- permissions.rs           # check_bucket_workspace_restriction
    +-- duckdb.rs                # format_duckdb_connection_settings, duckdb_connection_settings_internal, S3_PROXY_LAST_ERRORS_CACHE, DEFAULT_STORAGE
    +-- job_s3_helpers_ee.rs     # symlink -> ~/windmill-ee-private/windmill-object-store/src/job_s3_helpers_ee.rs
    +-- job_s3_helpers_oss.rs    # cfg(private) re-export + OSS stubs
```

Or keep as fewer files if the logical grouping feels over-split. The key is all S3 infrastructure lives in this crate.

### Functions to move from `windmill-common/src/s3_helpers.rs`

Statics:
- `OBJECT_STORE_SETTINGS` (RwLock)
- `DEFAULT_STORAGE`
- `S3_PROXY_LAST_ERRORS_CACHE`

Functions:
- `get_object_store()`
- `reload_object_store_setting()`
- `get_workspace_object_store()`
- `build_object_store_client()`
- `build_object_store_from_settings()`
- `build_s3_client()`
- `build_s3_client_from_settings()`
- `upload_artifact_to_store()`
- `attempt_fetch_bytes()`
- `get_etag_or_empty()`
- `render_endpoint()`
- `check_bucket_workspace_restriction()`
- `convert_json_line_stream()`
- `bundle()`
- `raw_app()`
- `lfs_to_object_store_resource()`
- `format_duckdb_connection_settings()`
- `duckdb_connection_settings_internal()`

### EE modules to move from `windmill-common`

- `windmill-common/src/job_s3_helpers_ee.rs` (symlinked EE file with OIDC token generation)
- `windmill-common/src/job_s3_helpers_oss.rs` (OSS wrapper)

Create new EE private directory:
```bash
mkdir -p ~/windmill-ee-private/windmill-object-store/src/
# Move the actual EE file content there
cp ~/windmill-ee-private/windmill-common/src/job_s3_helpers_ee.rs ~/windmill-ee-private/windmill-object-store/src/job_s3_helpers_ee.rs
# Create symlink
ln -s ~/windmill-ee-private/windmill-object-store/src/job_s3_helpers_ee.rs backend/windmill-object-store/src/job_s3_helpers_ee.rs
```

The OSS file uses the standard pattern:
```rust
#[cfg(feature = "private")]
pub use crate::job_s3_helpers_ee::*;

#[cfg(not(feature = "private"))]
// ... OSS stub implementations
```

### Cargo.toml

```toml
[package]
name = "windmill-object-store"
version.workspace = true
authors.workspace = true
edition.workspace = true

[features]
default = []
private = []
enterprise = []
parquet = [
  "windmill-common/parquet",
  "dep:object_store",
  "dep:aws-sdk-sts",
  "dep:aws-smithy-types-convert",
  "dep:datafusion",
  "dep:aws-config",
  "dep:aws-credential-types",
]

[dependencies]
windmill-common = { workspace = true, default-features = false }
windmill-types.workspace = true
serde = { workspace = true, features = ["derive"] }
serde_json.workspace = true
sqlx.workspace = true
tokio.workspace = true
tracing.workspace = true
reqwest.workspace = true
uuid.workspace = true
chrono.workspace = true
bytes.workspace = true

# Check s3_helpers.rs for actual deps used — these are likely needed:
# bitflags, once_cell/lazy_static, etc.

# Behind parquet feature (mirror windmill-common's parquet feature deps)
object_store = { workspace = true, optional = true }
aws-sdk-sts = { workspace = true, optional = true }
aws-smithy-types-convert = { workspace = true, optional = true }
datafusion = { workspace = true, optional = true }
aws-config = { workspace = true, optional = true }
aws-credential-types = { workspace = true, optional = true }
```

Check `windmill-common/Cargo.toml`'s parquet feature carefully — mirror exactly what `s3_helpers.rs` needs.

### Workspace Cargo.toml changes (`backend/Cargo.toml`)

1. Add `"./windmill-object-store"` to `[workspace] members`
2. Add `windmill-object-store = { path = "./windmill-object-store" }` to `[workspace.dependencies]`
3. Add `"windmill-object-store/private"` to the `private` feature list
4. Add `"windmill-object-store/enterprise"` to the `enterprise` feature list
5. Add `"windmill-object-store/parquet"` to the `parquet` feature list

## Step 3 — Delete `windmill-common/src/s3_helpers.rs` and update all consumers

Remove the `s3_helpers` module from `windmill-common/src/lib.rs` and delete the file.

Also remove `job_s3_helpers_oss.rs` and `job_s3_helpers_ee.rs` from `windmill-common/src/` (they moved to windmill-object-store). Remove their module declarations from `windmill-common/src/lib.rs`.

Remove any now-unused dependencies from `windmill-common/Cargo.toml` (object_store, aws-sdk-sts, etc. that were only needed by s3_helpers).

### Consumer files to update

Every file that imports from `windmill_common::s3_helpers` or `windmill_common::job_s3_helpers_oss` needs updating. Change type imports to `windmill_types::s3::*` and function imports to `windmill_object_store::*`.

Also add `windmill-object-store` (and propagate private/enterprise/parquet features) to the Cargo.toml of each consuming crate.

**windmill-common** (internal consumers — these move to importing from windmill-types or windmill-object-store):
- `src/worker.rs` — uses `attempt_fetch_bytes()`
- `src/jobs.rs` — uses `get_object_store()`
- `src/ai_types.rs` — uses `S3Object`
- `src/lib.rs` — remove `pub mod s3_helpers;`, `pub mod job_s3_helpers_oss;`, `#[cfg(feature = "private")] mod job_s3_helpers_ee;`

NOTE: `windmill-common` files that call functions from `windmill-object-store` create a circular dependency (`windmill-common` <-> `windmill-object-store`). Check `src/worker.rs` and `src/jobs.rs` — if they only use types, switch to `windmill_types::s3::*`. If they call functions like `attempt_fetch_bytes()` or `get_object_store()`, you need to either:
- Move that logic out of `windmill-common` into the calling crate, or
- Accept the circular dep by using the functions inline, or
- Keep just those specific functions in `windmill-common` and move the rest

Investigate these two files carefully before proceeding.

**backend root crate** (`src/main.rs`, `src/monitor.rs`):
- `reload_object_store_setting()`, `ObjectStoreReload` -> `windmill_object_store::`

**windmill-api**:
- `src/apps.rs` — `build_object_store_client()`, `S3Object`, `S3Permission`
- `src/job_helpers_oss.rs` — `StorageResourceType`
- `src/jobs.rs` — `upload_artifact_to_store()`, `BundleFormat`
- `src/args.rs` — `build_object_store_client()`
- `src/triggers/http/handler.rs` — `build_object_store_client()`
- `src/s3_proxy_oss.rs` + `src/s3_proxy_ee.rs` — heavy user of S3 helpers (keep these files in windmill-api, just update imports)

**windmill-api-settings**:
- `src/lib.rs` — `ObjectSettings`, `build_object_store_from_settings()`

**windmill-api-workspaces**:
- `src/workspaces.rs` — `LargeFileStorage`

**windmill-trigger-http**:
- `src/lib.rs` — S3 resource types

**windmill-worker**:
- `src/python_executor.rs` — `get_object_store()`, `OBJECT_STORE_SETTINGS`
- `src/common.rs` — `ObjectStoreResource`, `LargeFileStorage`
- `src/global_cache.rs` — `attempt_fetch_bytes()`
- `src/bun_executor.rs` — `attempt_fetch_bytes()`
- `src/duckdb_executor.rs` — `S3Object`, `S3_PROXY_LAST_ERRORS_CACHE`, `DEFAULT_STORAGE`
- `src/bigquery_executor.rs` — `convert_json_line_stream()`
- `src/pg_executor.rs` — `convert_json_line_stream()`
- `src/mssql_executor.rs` — `convert_json_line_stream()`
- `src/snowflake_executor.rs` — `convert_json_line_stream()`
- `src/universal_pkg_installer.rs` — S3 settings
- `src/oracledb_executor.rs` — S3/object store features

## Step 4 — S3 proxy (keep in windmill-api)

The S3 proxy (`s3_proxy_ee.rs` / `s3_proxy_oss.rs`) in `windmill-api` depends on axum for route registration. Keep these files in `windmill-api` — just update their imports from `windmill_common::s3_helpers::*` to `windmill_types::s3::*` (types) and `windmill_object_store::*` (functions).

Do NOT move the proxy into `windmill-object-store` — it would pull in axum as a dependency of a storage crate, which is wrong.

## Step 5 — Verification

Run all CI check commands and fix all errors and warnings:

```bash
cargo check
cargo check --features parquet
cargo check --features all_sqlx_features
RUSTFLAGS="-D warnings" cargo check --features all_sqlx_features,private
```

Then regenerate SQLX cache: `bash backend/update_sqlx.sh`
(It may fail on `windmill-operator` — that's pre-existing and unrelated.)

## Key risks and gotchas

1. **Circular dependency with windmill-common**: `windmill-common/src/worker.rs` and `src/jobs.rs` call S3 functions. If windmill-object-store depends on windmill-common, these files can't import from windmill-object-store. Investigate what they actually use — if it's just types, use `windmill_types::s3`. If it's functions, the code may need to be restructured.

2. **Feature flag propagation**: The `parquet` feature must propagate through every crate in the chain. Check `backend/Cargo.toml` features section — every crate that had `"windmill-common/parquet"` and uses S3 functions now also needs `"windmill-object-store/parquet"`.

3. **EE symlinks**: The EE private files must be created in `~/windmill-ee-private/windmill-object-store/src/` and symlinked into the crate. The old symlinks in `windmill-common/src/` should be removed.

4. **`OBJECT_STORE_SETTINGS` is a global static**: It's written to by `reload_object_store_setting()` (called from main.rs and monitor.rs) and read by `get_object_store()` (called from many places). Make sure it's `pub` in the new crate so all consumers can access it.

5. **SQLX queries**: If any functions being moved contain `sqlx::query!` macros, the SQLX offline cache files in `backend/.sqlx/` will need regeneration. These are workspace-wide so they should just work, but verify.

6. **Never use `SQLX_OFFLINE=true`** — a live database is always available. The SQLX cache is for CI only.

7. **CI uses `-D warnings`** — verify with `RUSTFLAGS="-D warnings"` locally. Common issues: unused imports in the shim file, dead code in feature-gated branches.
