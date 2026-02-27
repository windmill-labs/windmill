# Windmill Architecture

Windmill is an open-source developer platform for building internal tools, workflows, API integrations, background jobs, and user interfaces.

## Platform Components

- **API Server** (`windmill-api`): HTTP requests, authentication, resource management. Routes in `windmill-api/src/`, entry point `windmill-api/src/lib.rs`
- **Workers** (`windmill-worker`): Execute jobs in sandboxed environments (NSJAIL). Poll queue, route to language executors
- **Queue** (`windmill-queue`): Job queue in PostgreSQL. Manages scheduling, priorities, worker tags
- **Common** (`windmill-common`): Shared code — error types, DB types, flow definitions, triggers
- **Native Triggers** (`windmill-native-triggers`): External service integrations (Nextcloud, Google). See `docs/domain/` for guides
- **Other crates**: `windmill-audit`, `windmill-git-sync`, `windmill-autoscaling`

## Job Execution Flow

1. API receives request → creates job record in DB
2. Job added to PostgreSQL queue
3. Workers poll queue for matching jobs (by tags/capabilities)
4. Worker routes job to language executor (Python, TypeScript, Go, Bash, SQL, etc.)
5. Script executes in NSJAIL sandbox
6. Results stored in DB
7. For flows: each step creates a new job through the same pipeline

## Backend Structure

```
backend/
  windmill-api/src/          # API routes organized by domain
  windmill-worker/src/       # Job execution, language handlers
  windmill-common/src/       # Shared types: error, flows, jobs, triggers, scripts
  windmill-queue/src/        # Queue operations
  windmill-native-triggers/  # External service integrations (Nextcloud, Google)
  windmill-audit/src/        # Audit logging
  migrations/                # SQL migrations (apply with: sqlx migrate run)
  summarized_schema.txt      # Compact DB schema overview
  Cargo.toml                 # Feature flags gate crate compilation
```

Feature flags gate optional modules (enterprise, kafka, native_trigger, parquet, etc.). Check `backend/Cargo.toml` `[features]`.

## Frontend Structure

```
frontend/src/
  routes/                    # SvelteKit page routes
  lib/
    components/              # UI components organized by domain
      common/                # Shared: Button, TextInput, Select, Toggle, etc.
      triggers/              # Trigger management UI
      graph/                 # Flow graph rendering
      workspaceSettings/     # Workspace configuration
    gen/                     # Auto-generated types/services from OpenAPI
    stores/                  # Svelte stores for global state
    utils/                   # Utility functions
    tutorials/               # Tutorial system config
```

Built with: Svelte 5 (Runes), SvelteKit, Tailwind CSS, Monaco Editor.

## Database

- PostgreSQL with `sqlx` for compile-time checked queries
- Schema overview: `backend/summarized_schema.txt`
- Direct access: `psql postgres://postgres:changeme@localhost:5432/windmill`
- Useful commands: `\d <table>` (definition), `\di <table>*` (indexes), `\d+ <table>` (extended)
- Migrations: `backend/migrations/`, apply with `sqlx migrate run`
- After code changes: run `./update-sqlx` from `backend/` to regenerate offline query cache
- **Never use `SQLX_OFFLINE=true`** — a live database is always available

## OpenAPI & Generated Client

- Spec: `backend/windmill-api/openapi.yaml` (+ `openflow.openapi.yaml` for flows)
- Tag → `{Tag}Service`, operationId → method name
- Regenerate after API changes: `cd frontend && npm run generate-backend-client`
