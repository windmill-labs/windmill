CARGO_NET_GIT_FETCH_WITH_CLI=true cargo build --release -p windmill_duckdb_ffi_internal
mkdir -p ../target/debug/
# Copy only the dynamic library the worker dlopen()s. The crate also builds an
# rlib (so `cargo test` can link a unit-test harness on a cdylib crate); that
# and the .d depfile must NOT be copied — a glob would catch them.
for ext in so dylib; do
  f="target/release/libwindmill_duckdb_ffi_internal.$ext"
  [ -f "$f" ] && cp "$f" ../target/debug/
done
