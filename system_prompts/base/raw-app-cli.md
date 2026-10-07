# Windmill Raw Apps — CLI workflow

This guide covers raw apps from the terminal: scaffolding via `wmill app new`, the on-disk layout, and the file-based conventions the CLI uses to represent backend runnables and data table configuration. The platform shape (how a raw app behaves at runtime — frontend bundling, runnable types, datatable SDK calls) is covered in the companion authoring guide.

## Creating a Raw App

**You — the AI agent — create the app yourself by running `wmill app new` with the right flags. Do NOT tell the user to "run `wmill app new` and follow the prompts" or wait for them to do it.** The bare `wmill app new` is an interactive wizard that hangs waiting for stdin in any non-TTY context (which includes you). Always pass flags.

### Step 1 — Settle the setup

You need three values to run the command:

1. **summary** — a short description of the app
2. **path** — the windmill path, e.g. `f/folder/my_app` or `u/username/my_app`
3. **framework** — one of `react19` (recommended), `react18`, `svelte5`, `vue`

**summary and path**: if the user's request did not supply them, ask. Do not guess values and do not invent paths.

**framework**: never ask. Use the one the user named, otherwise the one the project's existing apps use, otherwise `react19` (see "Starting a new app" in the authoring guide).

**data setup**: when the app has to store data and the user did not say where, ask which data table, which schema and which tables, as "Data setup" in the authoring guide lays out. `wmill datatable list` gives the data tables to offer.

Use whichever interactive question facility your runtime provides — a structured multi-choice tool if available, otherwise plain chat — and group everything missing into a single round-trip so the user answers at once:

- For `summary` and `path` — provide one or two example values as multiple-choice options (the user can pick "Other" to type a free-form answer).
- For the data setup — the choices the authoring guide lists, the default first.

Only proceed once every value is concrete. If the user replies with something ambiguous, ask again rather than guessing.

### Step 2 — Run the command yourself

Once the setup is settled, run it:

```bash
wmill app new \
  --summary "Customer dashboard" \
  --path f/sales/dashboard \
  --framework react19 \
  --datatable main \
  --schema app1
```

`--summary`, `--path` and `--framework` are the minimum. The datatable wizard and the "Open in Claude Desktop?" prompt are skipped silently because passing any of them puts the command in non-interactive mode, so the data setup only reaches the app through the flags below.

### Optional flags

| Flag | When to add it |
|---|---|
| `--datatable <name>` | The app stores data: the data table settled in step 1. Without it, the app is created with no datatable. |
| `--schema <name>` | Together with `--datatable`, when the app's tables go in a schema, new or existing. If the schema doesn't exist yet, writes a `sql_to_apply/` migration with `CREATE SCHEMA IF NOT EXISTS`, applied like any other migration (see "SQL Migrations" below). Leave it out for `public`. |
| `--overwrite` | The target directory already exists and the user said it's OK to replace. Without it, non-interactive mode aborts with an error so you don't clobber existing work. |
| `--no-open-in-desktop` | Already implied in non-interactive mode; only needed if you're somehow running interactively. |

Existing tables the app reuses have no flag: list them under `data.tables` in `raw_app.yaml` (see "Data tables" below).

### Step 3 — Offer the visual preview

After `wmill app new` and any initial edits to `App.tsx` / `index.tsx`, **offer** to open the visual preview as a one-sentence next step (e.g. "Want me to open the visual preview?"). Don't auto-open — opening the dev page has side effects (browser window, possibly a `launch.json` entry when an embedded preview tool is in play) the user should consent to.

For apps the preview command runs from the app folder (`cd <app_path>__raw_app && wmill app dev …`); the `preview` skill picks the proxy vs direct branch based on whether the runtime exposes a tool that can embed a localhost URL. If the user already asked to see/preview/visualize the app in their original request, skip the offer and just invoke the skill.

### Anti-patterns to avoid

- ❌ Running `wmill app new` with no flags (the prompt will hang).
- ❌ Telling the user to "run `wmill app new` and follow the prompts" — that's a step backwards from what you can do directly.
- ❌ Inventing a path or summary instead of asking the user.
- ❌ Asking which framework to use — `react19` is the default when nothing says otherwise.
- ❌ Creating a schema or tables for a new app before the user has said where its data goes.
- ❌ Passing `--overwrite` automatically when the directory exists — confirm with the user first.

### Interactive (only when a human is at the terminal)

```bash
wmill app new
```

This is the wizard. It only works when run by a human in a real terminal. Don't call it this way from an agent.

## On-disk app layout

```
my_app__raw_app/
├── AGENTS.md              # AI agent instructions (auto-generated)
├── DATATABLES.md          # Database schemas (run 'wmill app generate-agents' to refresh)
├── raw_app.yaml           # App configuration (summary, path, data settings)
├── index.tsx              # Frontend entry point
├── App.tsx                # Main React/Svelte/Vue component
├── index.css              # Styles
├── package.json           # Frontend dependencies
├── wmill.ts               # Auto-generated backend type definitions (DO NOT EDIT)
├── backend/               # Backend runnables (server-side scripts)
│   ├── <id>.<ext>         # Code file (e.g., get_user.ts)
│   ├── <id>.yaml          # Optional: config for fields, or to reference existing scripts
│   └── <id>.lock          # Lock file (run 'wmill generate-metadata' to create/update)
└── sql_to_apply/          # SQL migrations (dev only, not synced)
    └── *.sql              # SQL files to apply via dev server
```

## Backend runnables on disk

Add a code file to the `backend/` folder:

```
backend/<id>.<ext>
```

The runnable ID is the filename without extension. For example, `get_user.ts` creates a runnable with ID `get_user`.

### Supported languages (extension-driven)

| Language         | Extension    | Example          |
|------------------|--------------|------------------|
| TypeScript       | `.ts`        | `myFunc.ts`      |
| TypeScript (Bun) | `.bun.ts`    | `myFunc.bun.ts`  |
| TypeScript (Deno)| `.deno.ts`   | `myFunc.deno.ts` |
| Python           | `.py`        | `myFunc.py`      |
| Go               | `.go`        | `myFunc.go`      |
| Bash             | `.sh`        | `myFunc.sh`      |
| PowerShell       | `.ps1`       | `myFunc.ps1`     |
| PostgreSQL       | `.pg.sql`    | `myFunc.pg.sql`  |
| MySQL            | `.my.sql`    | `myFunc.my.sql`  |
| BigQuery         | `.bq.sql`    | `myFunc.bq.sql`  |
| Snowflake        | `.sf.sql`    | `myFunc.sf.sql`  |
| MS SQL           | `.ms.sql`    | `myFunc.ms.sql`  |
| GraphQL          | `.gql`       | `myFunc.gql`     |
| PHP              | `.php`       | `myFunc.php`     |
| Rust             | `.rs`        | `myFunc.rs`      |
| C#               | `.cs`        | `myFunc.cs`      |
| Java             | `.java`      | `myFunc.java`    |

After creating or editing a backend runnable — especially when its imports or arguments changed — its local lock and `wmill-lock.yaml` go stale. Offer to run `wmill generate-metadata` and run it once the user agrees (or automatically if the project's `AGENTS.md` opts into that) — YOU run it, don't just name it and wait. It writes local files only (not a deploy), and keeping the lock current avoids noise in git-sync/CI:
```bash
wmill generate-metadata
```
After it runs, check the regenerated `.lock` diff and tell the user which dependency versions changed (e.g. `requests 2.31.0 → 2.32.0`), so they can catch an unwanted bump before deploying.

### Optional YAML configuration

Add a `<id>.yaml` file alongside the code to configure fields or static values:

**backend/get_user.yaml:**
```yaml
type: inline
fields:
  user_id:
    type: static
    value: "default_user"
```

### Referencing existing scripts

To use an existing Windmill script instead of inline code:

**backend/existing_script.yaml:**
```yaml
type: script
path: f/my_folder/existing_script
```

For flows:
```yaml
type: flow
path: f/my_folder/my_flow
```

## Data tables — `raw_app.yaml` config

The `data` block in `raw_app.yaml` controls which tables the app can query.

```yaml
data:
  datatable: main           # Default datatable
  schema: app_schema        # Schema the app's tables go in (optional); still write them as app_schema.<table>
  tables:
    - main/users            # Table in public schema
    - main/app_schema:items # Table in specific schema
  roles:                    # Optional: the role the app uses each datatable through
    main: analyst
```

**Table reference formats:**
- `<datatable>` — All tables in the datatable
- `<datatable>/<table>` — Specific table in public schema
- `<datatable>/<schema>:<table>` — Table in specific schema

**Roles:** when a datatable is under roles, its queries run as a role, which only reaches what it was granted. `roles` records the role the app uses each datatable through; the app's code must pass the same role: `wmill.datatable('main', { role: 'analyst' })` in TypeScript, `wmill.datatable('main', role='analyst')` in Python. A datatable without an entry is used as its default role.

## SQL Migrations (sql_to_apply/)

The `sql_to_apply/` folder is for creating/modifying database tables during development.

### Workflow

1. Create `.sql` files in `sql_to_apply/`
2. Run `wmill app dev` — the dev server watches this folder
3. When SQL files change, a modal appears in the browser to confirm execution
4. After creating tables, **add them to `data.tables`** in `raw_app.yaml`

### Example migration

**sql_to_apply/001_create_users.sql:**
```sql
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT,
    created_at TIMESTAMP DEFAULT NOW()
);
```

After applying, add to `raw_app.yaml`:
```yaml
data:
  tables:
    - main/users
```

A migration runs with no default schema, so a table outside `public` is created with its schema, `CREATE TABLE IF NOT EXISTS app_schema.items (...)`, listed as `main/app_schema:items`, and queried as `app_schema.items`.

### Migration best practices

- **Use idempotent SQL**: `CREATE TABLE IF NOT EXISTS`, etc.
- **Number files**: `001_`, `002_` for ordering
- **Always whitelist tables** after creation
- This folder is NOT synced — it's for local development only

## CLI Commands

Commands you run yourself, not the user:
- `wmill app new` — run it with flags, per the "Creating a Raw App" section above.
- `wmill app lint <app_folder>` — checks the app's structure and that it builds. Run it after editing, before offering a preview or a deploy; a bundle that compiles still says nothing about behavior, so a preview is what checks that.
- `wmill generate-metadata` — (re)generates local lock files and refreshes `wmill-lock.yaml` content hashes; writes local files only (not a deploy). After adding or editing a runnable, offer it and run it on agreement — or automatically if the project's `AGENTS.md` opts into that (see "After creating a runnable" above).

For the rest, tell the user which command fits their intent and let them run it — these deploy to the workspace, overwrite local files, or launch a long-running server, so the user should consent each time:

| Command | Description |
|---------|-------------|
| `wmill app dev` | Start dev server with live reload (see the `preview` skill for the full open-the-app-in-the-IDE-pane procedure). |
| `wmill app generate-agents` | Refresh AGENTS.md and DATATABLES.md |
| `wmill sync push` | Deploy app to Windmill |
| `wmill sync pull` | Pull latest from Windmill |

