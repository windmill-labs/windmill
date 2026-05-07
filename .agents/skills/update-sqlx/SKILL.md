---
name: update-sqlx
description: How to safely update SQLx offline query cache. MUST use when SQL queries change.
---

# SQLx Offline Query Cache

Windmill uses `SQLX_OFFLINE=true` in CI, which requires all `sqlx::query!` / `sqlx::query_as!` macros to have matching cached query data in `backend/.sqlx/`.

## When to Run

Run after any change to SQL queries in Rust source files. Without it, CI will fail with:
```
error: `SQLX_OFFLINE=true` but there is no cached data for this query
```

## The Problem

`cargo sqlx prepare --workspace` **deletes all existing cache files** and regenerates only the ones found in the current compilation. If you don't compile with every feature flag (especially `private` for EE files), you will **silently delete EE query caches**, breaking CI for enterprise tests.

The standard `./update_sqlx.sh` script tries to compile with all features, but it often fails locally because the EE symlinks can be out of sync with `main`.

## Safe Procedure

Always preserve the existing EE caches from `origin/main`. Use this workflow:

```bash
cd backend

# 1. Restore the full cache from main (includes EE caches)
git checkout origin/main -- .sqlx/

# 2. Run prepare with OSS features (what compiles locally)
#    This regenerates OSS caches to match your code changes.
cargo sqlx prepare --workspace -- --workspace --features all_sqlx_features

# 3. Restore any EE caches that were deleted in step 2.
#    These are files present in origin/main but missing after prepare.
git ls-tree origin/main backend/.sqlx/ \
  | awk '{print $4}' | sed 's|backend/\.sqlx/||' | sort > /tmp/main_files.txt

find backend/.sqlx -name "*.json" -printf '%P\n' | sort > /tmp/current_files.txt

comm -23 /tmp/main_files.txt /tmp/current_files.txt > /tmp/missing_files.txt

while read f; do
  git show "origin/main:backend/.sqlx/$f" > "backend/.sqlx/$f"
done < /tmp/missing_files.txt

# 4. Verify nothing was lost from main
find backend/.sqlx -name "*.json" -printf '%P\n' | sort > /tmp/current_files.txt
comm -23 /tmp/main_files.txt /tmp/current_files.txt | wc -l
# Should output: 0
```

## If EE Compiles Locally

If your EE repo happens to be in sync, you can use the full script (faster):

```bash
cd backend
./update_sqlx.sh
```

But if it fails with EE compilation errors, use the safe procedure above.

## What NOT to Do

- **Never** run `cargo sqlx prepare --workspace` with only OSS features and commit the result — it will delete EE caches.
- **Never** set `SQLX_OFFLINE=true` for local `cargo sqlx prepare` — use a live database per CLAUDE.md. (CI runs with `SQLX_OFFLINE=true`, which is why the cache must be complete.)
- **Never** skip the verification step (step 4 above).

## Verification

After committing, the diff against `origin/main` should show:
- A few **new** cache files (for your changed queries)
- A few **deleted** cache files (for old queries that no longer exist)
- **Zero** net deletions from the EE cache set

```bash
git diff origin/main --stat backend/.sqlx/
```
