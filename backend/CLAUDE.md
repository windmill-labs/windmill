# Backend Development (Rust)

## Project Structure

Windmill uses a workspace-based architecture with multiple crates:

- **windmill-api**: API server functionality
- **windmill-worker**: Job execution
- **windmill-common**: Shared code used by all crates
- **windmill-queue**: Job & flow queuing
- **windmill-audit**: Audit logging
- Other specialized crates (git-sync, autoscaling, etc.)

## Key References (MUST FOLLOW THESE)

- You MUST follow best-practices by using the `rust-backend` skill, everytime you write RUST code.
- When working with the database: read `summarized_schema.txt` before starting 
- When working with the API routes: you can read `windmill-api/src/lib.rs` to get started

## Adding New Code

### Module Organization

- Place new code in the appropriate crate based on functionality
- For API endpoints, create or modify files in `windmill-api/src/` organized by domain
- For shared functionality, use `windmill-common/src/`
- Follow existing patterns for file structure and organization

### API Endpoints

- Follow existing patterns in the `windmill-api` crate
- Use axum's routing system and extractors
- Update `backend/windmill-api/openapi.yaml` after modifying API endpoints

### Database Changes

- Update database schema with migration if necessary
- Use `sqlx` for database operations with prepared statements
- Use transactions for multi-step operations
- To apply pending migrations: `sqlx migrate run` (never manually run .sql files)
- **Never use `SQLX_OFFLINE=true`** — a live database is always available for compilation
- After all code changes are done, run `./update-sqlx` to regenerate the offline query cache

## Enterprise Features

- Enterprise files use the `*_ee.rs` suffix
- Enterprise source is in `windmill-ee-private` folder (sibling directory at `../../windmill-ee-private` or `~/windmill-ee-private`), symlinked into each crate's `src/`
- The `_ee.rs` files are gitignored in the main repo — they are tracked only in the `windmill-ee-private` repo
- You can and should modify `windmill-ee-private` directly when needed (e.g., when creating new crates that need EE code, mirror the package structure there)
- Use feature flags: `#[cfg(feature = "enterprise")]`
- Isolate enterprise code in separate modules

### EE PR Workflow (MUST DO when modifying `*_ee.rs` files)

When you modify any `*_ee.rs` file and create a PR on the windmill repo, you **MUST** also:

1. **Create a matching branch** in the `windmill-ee-private` repo (use the same branch name). If using worktrees, the EE worktree is at `~/windmill-ee-private__worktrees/<branch-name>/`
2. **Commit and push** the `_ee.rs` changes in that branch
3. **Create a PR** on `windmill-ee-private` with a link to the companion windmill PR
4. **Update `ee-repo-ref.txt`**: Run `bash write_latest_ee_ref.sh` from `backend/` to write the latest EE commit hash. **Important**: the script may fall back to `~/windmill-ee-private` (main branch) instead of the worktree — verify it wrote the correct commit hash from your branch, not from main. If wrong, manually write the correct hash.
5. **Commit `ee-repo-ref.txt`** in the windmill repo so CI picks up the correct EE ref

## Code Validation (MUST DO)

After making backend changes, you MUST run `cargo check` and fix all errors and warnings before considering the work done.

Only enable the feature flags relevant to your changes — do NOT use `all_sqlx_features` as it compiles the entire codebase and is very slow. Check the `[features]` section in `Cargo.toml` to identify which flags gate the crates/modules you modified.

Examples:
```bash
# Changed core code (no feature-gated modules)
cargo check

# Changed code behind the enterprise feature
cargo check --features enterprise

# Changed kafka trigger code
cargo check --features kafka
```

## Git Workflow

- **Never push directly to main** — always create a branch and open a pull request

## Testing

- Write unit tests for core functionality
- Use the `#[cfg(test)]` module for test code
- For database tests, use the existing test utilities

## Common Crates

- **tokio**: Async runtime
- **axum**: Web server and routing
- **sqlx**: Database operations
- **serde**: Serialization/deserialization
- **tracing**: Logging and diagnostics
- **reqwest**: HTTP client