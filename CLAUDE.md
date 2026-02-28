# Windmill

Open-source platform for internal tools, workflows, API integrations, background jobs, and UIs. Rust backend + Svelte 5 frontend.

## Workflow

1. **Understand**: Before coding, read relevant docs from `docs/` to understand the area you're changing
2. **Plan**: For non-trivial changes, use plan mode. For large features, break into reviewable stages
3. **Execute**: Follow coding patterns from skills (`rust-backend`, `svelte-frontend`)
4. **Validate**: After every change, run the appropriate checks per `docs/validation.md`

## Documentation

- **Validation**: `docs/validation.md` — what checks to run based on what you changed
- **Autonomous mode**: `docs/autonomous-mode.md` — when running in bypass/auto permission mode
- **Enterprise**: `docs/enterprise.md` — EE file conventions and PR workflow
- **Backend patterns**: use the `rust-backend` skill when writing Rust code
- **Frontend patterns**: use the `svelte-frontend` skill when writing Svelte code
- **Domain guides**: `.claude/skills/native-trigger/` and `frontend/tutorial-system-guide.mdc`
- **Brand/UI guidelines**: `frontend/brand-guidelines.md`

## Dev Environment

- **Backend**: `cargo run` from `backend/` (API at http://localhost:8000)
- **Frontend**: `REMOTE=http://localhost:8000 npm run dev` from `frontend/` (port 3000+)
- **DB**: `psql postgres://postgres:changeme@localhost:5432/windmill`
- **Login**: `admin@windmill.dev` / `changeme`
- **Instance settings**: navigate to `/#superadmin-settings`

## Core Principles

- Search for existing code to reuse before writing new code
- Follow established patterns in the codebase
- Keep changes focused — don't refactor beyond what's asked
