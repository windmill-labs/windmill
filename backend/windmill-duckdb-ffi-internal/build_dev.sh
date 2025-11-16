CARGO_NET_GIT_FETCH_WITH_CLI=1 cargo build --release -p windmill_duckdb_ffi_internal
cp target/release/libwindmill_duckdb_ffi_internal.* ../target/debug/