---
name: update-sqlx
description: How to safely update SQLx offline query cache. MUST use when SQL queries change.
---

# SQLx Offline Query Cache

Windmill uses `SQLX_OFFLINE=true` in CI, which requires all `sqlx::query!` / `sqlx::query_as!` macros to have matching cached query data in `backend/.sqlx/`.

## When to Run

Run after **adding or editing** a SQL query in Rust source. Without it, CI fails with:
```
error: `SQLX_OFFLINE=true` but there is no cached data for this query
```

**Do NOT run it when a change only *removes* queries.** The cache is already complete for
CI; all that is left are orphaned entries, which are cosmetic and never break a build.
Running `prepare` to tidy them risks destroying the cache for no gain. Delete them
offline instead: for each `.sqlx/query-*.json`, normalize its `query` field (strip `\`
line-continuations, collapse whitespace) and check whether it still appears in any `.rs`
file. That detector reports ~48 false positives in a CE checkout — EE queries live in
`*_ee.rs` symlinks it cannot read — so **filter to the tables your change touched** and
delete only those.

## Before You Run Anything

1. **Back the cache up.** `prepare` deletes `.sqlx/` *before* regenerating, so any compile
   failure leaves it gutted (observed: 2350 → 142 entries).
   ```bash
   bash .agents/skills/update-sqlx/sqlx-cache.sh backup
   ```
   Its state is per-worktree, so a sibling worktree running `prepare` at the same time
   cannot overwrite your backup.
2. **Point `DATABASE_URL` at THIS worktree's database.** `prepare` compiles every
   `sqlx::query!` against the **live** database. Another worktree's DB lacks your
   migrations, so every new-table query fails and takes the cache down with it. The
   symptom is `relation "<your_new_table>" does not exist` — that is a wrong
   `DATABASE_URL`, not a broken query. See AGENTS.md → "Per-worktree ports and database".

## Queries Inside Tests Need `--all-targets`, Which Fails In A CE Checkout

`prepare` only caches queries in code it compiles, and `--workspace` alone does **not**
compile test targets. A `sqlx::query!` inside `tests/*.rs` therefore gets no entry, and CI
fails on the test target with the usual "no cached data" error even though the lib built
clean. `SQLX_OFFLINE=true cargo check --workspace --all-targets` is what reproduces it.

Adding `--all-targets` caches them — and, in a CE checkout, **aborts partway through**:
`backend/tests/otel.rs` imports `windmill_common::otel_ee`, which exists only behind the
`private` feature, so the compile dies after `prepare` has already emptied `.sqlx/`.
Observed: 2435 → 4 entries, `error: cargo check failed with status: exit status: 101`.

Do not fight it — the abort is a pre-existing EE gap, not something your change caused.
Take the entries you need and put the backup back:

```bash
bash .agents/skills/update-sqlx/sqlx-cache.sh backup

cd backend
DATABASE_URL=<this worktree's db> \
  cargo sqlx prepare --workspace -- --workspace --features all_sqlx_features --all-targets
# expected to fail; it still wrote the entries it got to before dying
cd ..

bash .agents/skills/update-sqlx/sqlx-cache.sh newq      # prints each added query
bash .agents/skills/update-sqlx/sqlx-cache.sh restore   # backup back, added entries grafted on
```

**Read what `newq` prints before running `restore`** — it shows each added entry's `query`
field, and every one should be yours. The set is small (one per new test query); anything
else in there means the run got further than you think.

Then verify both targets, since the lib passing says nothing about the tests:

```bash
SQLX_OFFLINE=true cargo check --workspace --features all_sqlx_features   # lib
SQLX_OFFLINE=true cargo check -p <your-crate> --all-targets              # tests
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
- **Never** run `prepare` without a `.sqlx` backup, or against a `DATABASE_URL` you have not confirmed belongs to this worktree.
- **Never** run `prepare` at all for a removal-only change.
- **Never** skip the verification step (step 4 above).
- **Never** leave a `--all-targets` run's output in place after it aborts — it is a
  near-empty cache. Restore the backup and graft on only the entries you verified.

Step 4 compares against `origin/main` because step 1 restored from it, so the two agree.
If you did **not** run step 1 — auditing a branch's cache on its own, say — compare
against `git merge-base HEAD origin/main` instead: `origin/main` advances, so its newer
entries would read as losses on your branch.

## Verification

After committing, the diff against `origin/main` should show:
- A few **new** cache files (for your changed queries)
- A few **deleted** cache files (for old queries that no longer exist)
- **Zero** net deletions from the EE cache set

```bash
git diff origin/main --stat backend/.sqlx/
```
