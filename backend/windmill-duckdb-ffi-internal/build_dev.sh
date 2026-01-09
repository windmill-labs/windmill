CARGO_NET_GIT_FETCH_WITH_CLI=true cargo build --release -p windmill_duckdb_ffi_internal
mkdir -p ../target/debug/
cp target/release/libwindmill_duckdb_ffi_internal.* ../target/debug/