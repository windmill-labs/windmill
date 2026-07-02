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
