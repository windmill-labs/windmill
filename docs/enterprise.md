# Enterprise (EE) Development

## File Conventions

- Enterprise files use the `*_ee.rs` suffix
- Source lives in `windmill-ee-private` (sibling repo), symlinked into each crate's `src/`
- `_ee.rs` files are gitignored in the main windmill repo — tracked only in `windmill-ee-private`
- Use feature flags: `#[cfg(feature = "enterprise")]` for enterprise logic
- The `private` feature flag gates compilation of `*_ee.rs` files
- The `license` feature flag gates features that require a valid license key at runtime
- Isolate enterprise code in separate modules

## Finding the EE Repo

- Standard location: `~/windmill-ee-private`
- Worktree location: `~/windmill-ee-private__worktrees/<branch-name>/`

## Detecting EE Changes

The `*_ee.rs` files in the windmill repo are symlinks — changes won't appear in `git diff` of the windmill repo. Check the EE repo directly: `git -C <ee-path> status --short`

## EE PR Workflow (MUST DO when modifying `*_ee.rs` files)

When you modify any `*_ee.rs` file and create a PR on windmill:

1. **Prefix the windmill PR title** with `[ee]`: `[ee] <type>: <description>`
2. **Create a matching branch** in `windmill-ee-private` (same branch name)
3. **Commit and push** the `_ee.rs` changes in that branch
4. **Create a companion PR** on `windmill-ee-private` with a link to the windmill PR (no `[ee]` prefix on this one)
5. **Update `ee-repo-ref.txt`**: Run `bash write_latest_ee_ref.sh` from `backend/`
   - **Verify** it wrote the correct commit hash from your branch, not from main (the script may fall back to `~/windmill-ee-private` on main)
   - If wrong, manually write the correct hash
6. **Commit `ee-repo-ref.txt`** in the windmill repo so CI picks up the correct EE ref

## Validation

```bash
# EE code (always include private to compile *_ee.rs files)
cargo check --features enterprise,private

# EE code that also requires license validation
cargo check --features enterprise,private,license
```
