# Windmill Development Guide

## Overview

Windmill is an open-source developer platform for building internal tools, workflows, API integrations, background jobs, workflows, and user interfaces. See @windmill-overview.mdc for full platform details.

## New Feature Implementation Guidelines

When implementing new features in Windmill, follow these best practices:

- **Clean Code First**: Write clean, readable, and maintainable code. Prioritize clarity over cleverness.
- **Avoid Duplication at All Costs**: Before writing new code, thoroughly search for existing implementations that can be reused or extended.
- **Adapt Existing Code**: Refactor and generalize existing code when necessary to avoid logic duplication. Extract common patterns into reusable utilities.
- **Follow Established Patterns**: Study existing code patterns in the codebase and maintain consistency with established conventions.
- **Single Responsibility**: Each function, component, and module should have a single, well-defined responsibility.
- **Incremental Implementation**: Break large features into smaller, reviewable chunks that can be implemented and tested incrementally.

## Language-Specific Guides

- Backend (Rust): see `backend/CLAUDE.md` and the `rust-backend` skill: `.claude/skills/rust-backend/SKILL.md`
- Frontend (Svelte 5): see `frontend/CLAUDE.md` and the `svelte-frontend` skill: `.claude/skills/svelte-frontend/SKILL.md`

## Dev Environment

- **Backend**: `cargo run` from `backend/` (API at http://localhost:8000)
- **Frontend**: `REMOTE=http://localhost:8000 npm run dev` from `frontend/`
  - The `REMOTE` env var configures the Vite proxy target. Without it, API calls proxy to `https://app.windmill.dev` instead of the local backend.
  - The dev server starts on port 3000 (or 3001+ if 3000 is in use).
- **Default login**: `admin@windmill.dev` / `changeme`
- **Instance settings**: navigate to `/#superadmin-settings` (opens the drawer overlay)

## UI Testing with Playwright MCP

When testing the frontend with the Playwright MCP tools:

1. **Start servers**: Launch backend (`cargo run`) and frontend (`REMOTE=http://localhost:8000 npm run dev`) as background tasks
2. **Wait for readiness**: Backend takes ~60s to compile; check output for `health check completed`. Frontend starts in ~5s.
3. **Login flow**: Navigate to `/user/login`, click "Log in without third-party", fill email/password, submit
4. **Instance settings drawer**: Navigate to `/#superadmin-settings` to open the drawer directly
5. **Toggle components**: The YAML toggle uses a custom `<Toggle>` component where the checkbox is visually hidden (`sr-only`). Click the wrapper `<label>` element (the parent container with `cursor=pointer`), not the checkbox ref directly.
6. **Console errors to ignore**: `critical_alerts` 404s are expected on CE builds (EE-only endpoint). VSCode worker 404s are dev-mode artifacts.

## Code Validation (MUST DO)

After making code changes, you MUST run the appropriate checks and fix all errors before considering the work done:

- **Backend**: Run `cargo check` from the `backend/` directory. Only enable the feature flags needed for the code you changed — check `backend/Cargo.toml` `[features]` section to identify which flags gate the crates/modules you modified. For example: `cargo check --features enterprise,parquet` if you only touched enterprise and parquet code.
- **Frontend**: Run `npm run check` from the `frontend/` directory.

## Querying the Database

`backend/summarized_schema.txt` provides a compact overview of all tables, columns, types, ENUMs, and foreign keys. Use it to quickly understand the data model and relationships. Note: this file is a simplified summary — it omits indexes, constraints details, and other metadata.

For exact table definitions (indexes, constraints, column defaults, etc.), query the database directly:

```bash
psql postgres://postgres:changeme@localhost:5432/windmill
```

Useful psql commands:
- `\d <table_name>` — full table definition with indexes and constraints
- `\di <table_name>*` — list indexes for a table
- `\d+ <table_name>` — extended table info including storage and descriptions

This is also helpful for:
- Inspecting database state during development
- Testing queries before implementing them in Rust
- Debugging data-related issues
