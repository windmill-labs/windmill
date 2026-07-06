#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

# The `duckdb` "bundled" feature compiles the whole DuckDB C++ library from
# source (~minutes) and dominates this crate's build. The crate changes very
# rarely, so by default build into a per-user cache shared across worktrees and
# keyed by the dependency lock: a fresh worktree reuses the already-compiled
# DuckDB instead of rebuilding it, and distinct DuckDB versions don't collide.
# Override the cache root with WINDMILL_DUCKDB_FFI_TARGET.
#
# If you are actively editing this crate in a worktree, uncommitted changes to
# its source switch to an isolated per-worktree ./target so your incremental
# builds neither disturb nor are disturbed by the shared cache.

src_dirty="$(git status --porcelain -- src Cargo.toml Cargo.lock build.rs 2>/dev/null || true)"

if [ -n "$src_dirty" ]; then
  export CARGO_TARGET_DIR="$PWD/target"
  echo "duckdb-ffi: local crate changes detected -> isolated target $CARGO_TARGET_DIR"
else
  cache_root="${WINDMILL_DUCKDB_FFI_TARGET:-${XDG_CACHE_HOME:-$HOME/.cache}/windmill/duckdb-ffi-target}"
  if command -v sha256sum >/dev/null 2>&1; then hash_cmd=sha256sum; else hash_cmd="shasum -a 256"; fi
  key="$(cat Cargo.lock build.rs | $hash_cmd | cut -c1-16)"
  export CARGO_TARGET_DIR="$cache_root/$key"
  echo "duckdb-ffi: shared cache $CARGO_TARGET_DIR"
  # The cache key covers only the dependency inputs (Cargo.lock + build.rs), so
  # the expensive bundled-DuckDB build is shared even when this crate's own
  # source differs between checkouts. But the shared dir's uplifted cdylib is a
  # single fixed-name artifact written by whichever worktree built last. Touch
  # our sources so cargo re-links THIS checkout's cdylib and re-uplifts it
  # before we copy — the copied artifact then always matches this worktree,
  # never a sibling's. The bundled dependency is a separate crate and stays
  # cached, so this only costs a ~1s relink.
  find src -type f -exec touch {} +
fi

CARGO_NET_GIT_FETCH_WITH_CLI=true cargo build --release -p windmill_duckdb_ffi_internal
mkdir -p ../target/debug/
cp "$CARGO_TARGET_DIR/release/"libwindmill_duckdb_ffi_internal.* ../target/debug/
