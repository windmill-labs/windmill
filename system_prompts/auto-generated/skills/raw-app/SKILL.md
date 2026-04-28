---
name: raw-app
description: MUST use when creating raw apps.
---

# Windmill Raw Apps

Raw apps let you build custom frontends with React, Svelte, or Vue that connect to Windmill backend runnables and datatables.

## Creating a Raw App

**You — the AI agent — create the app yourself by running `wmill app new` with the right flags. Do NOT tell the user to "run `wmill app new` and follow the prompts" or wait for them to do it.** The bare `wmill app new` is an interactive wizard that hangs waiting for stdin in any non-TTY context (which includes you). Always pass flags.

### Step 1 — Gather the three required values by asking the user

You need three things to run the command:

1. **summary** — a short description of the app
2. **path** — the windmill path, e.g. `f/folder/my_app` or `u/username/my_app`
3. **framework** — one of `react19` (recommended), `react18`, `svelte5`, `vue`

If the user's request did not supply *every* one of these explicitly, ask. Do not guess values, do not invent paths, do not pick a framework on the user's behalf, do not "just use react19 because it's the default".

Use whichever interactive question facility your runtime provides — a structured multi-choice tool if available, otherwise plain chat — and group all missing fields into a single round-trip so the user answers them at once:

- For `framework` — multiple-choice with the four allowed values; mark `react19` as `(Recommended)` and put it first.
- For `summary` and `path` — provide one or two example values as multiple-choice options (the user can pick "Other" to type a free-form answer).

Only proceed once you have concrete values for all three. If the user replies with something ambiguous, ask again rather than guessing.

### Step 2 — Run the command yourself

Once you have summary + path + framework, run it:

```bash
wmill app new \
  --summary "Customer dashboard" \
  --path f/sales/dashboard \
  --framework react19
```

That's the minimum. The datatable wizard and the "Open in Claude Desktop?" prompt are skipped silently because passing any of `--summary`/`--path`/`--framework` puts the command in non-interactive mode.

### Optional flags

Layer these in only when the user asked for them:

| Flag | When to add it |
|---|---|
| `--datatable <name>` | The user wants this app wired to a specific Windmill datatable. Without it, the app is created with no datatable. |
| `--schema <name>` | Together with `--datatable`. Creates the schema with `CREATE SCHEMA IF NOT EXISTS` if it doesn't already exist. |
| `--overwrite` | The target directory already exists and the user said it's OK to replace. Without it, non-interactive mode aborts with an error so you don't clobber existing work. |
| `--no-open-in-desktop` | Already implied in non-interactive mode; only needed if you're somehow running interactively. |

### Step 3 — Offer the visual preview

After `wmill app new` and any initial edits to `App.tsx` / `index.tsx`, **offer** to open the visual preview as a one-sentence next step (e.g. "Want me to open the visual preview?"). Don't auto-open — opening the dev page has side effects (browser window, possibly a `launch.json` entry when an embedded preview tool is in play) the user should consent to.

For apps the preview command runs from the app folder (`cd <app_path>__raw_app && wmill app dev …`); the `preview` skill picks the proxy vs direct branch based on whether the runtime exposes a tool that can embed a localhost URL. If the user already asked to see/preview/visualize the app in their original request, skip the offer and just invoke the skill.

### Anti-patterns to avoid

- ❌ Running `wmill app new` with no flags (the prompt will hang).
- ❌ Telling the user to "run `wmill app new` and follow the prompts" — that's a step backwards from what you can do directly.
- ❌ Inventing a path/summary/framework instead of asking the user.
- ❌ Defaulting to `react19` because the user didn't say — even sensible defaults must be confirmed.
- ❌ Passing `--overwrite` automatically when the directory exists — confirm with the user first.

### Interactive (only when a human is at the terminal)

```bash
wmill app new
```

This is the wizard. It only works when run by a human in a real terminal. Don't call it this way from an agent.

## App Structure

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

## Backend Runnables

Backend runnables are server-side scripts that your frontend can call. They live in the `backend/` folder.

### Creating a Backend Runnable

Add a code file to the `backend/` folder:

```
backend/<id>.<ext>
```

The runnable ID is the filename without extension. For example, `get_user.ts` creates a runnable with ID `get_user`.

### Supported Languages

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

### Example Backend Runnable

**backend/get_user.ts:**
```typescript
import * as wmill from 'windmill-client';

export async function main(user_id: string) {
  const sql = wmill.datatable();
  const user = await sql`SELECT * FROM users WHERE id = ${user_id}`.fetchOne();
  return user;
}
```

After creating, tell the user they can generate lock files by running:
```bash
wmill generate-metadata
```

### Optional YAML Configuration

Add a `<id>.yaml` file to configure fields or static values:

**backend/get_user.yaml:**
```yaml
type: inline
fields:
  user_id:
    type: static
    value: "default_user"
```

### Referencing Existing Scripts

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

### Calling Backend from Frontend

Import from the auto-generated `wmill.ts`:

```typescript
import { backend } from './wmill';

// Call a backend runnable
const user = await backend.get_user({ user_id: '123' });
```

The `wmill.ts` file provides type-safe access to all backend runnables.

## Data Tables

Raw apps can query Windmill datatables (PostgreSQL databases managed by Windmill).

### Critical Rules

1. **ONLY USE WHITELISTED TABLES**: You can ONLY query tables listed in `raw_app.yaml` → `data.tables`. Tables not in this list are NOT accessible.

2. **ADD TABLES BEFORE USING**: To use a new table, first add it to `data.tables` in `raw_app.yaml`.

3. **USE CONFIGURED DATATABLE/SCHEMA**: Check the app's `raw_app.yaml` for the default datatable and schema.

### Configuration in raw_app.yaml

```yaml
data:
  datatable: main           # Default datatable
  schema: app_schema        # Default schema (optional)
  tables:
    - main/users            # Table in public schema
    - main/app_schema:items # Table in specific schema
```

**Table reference formats:**
- `<datatable>` - All tables in the datatable
- `<datatable>/<table>` - Specific table in public schema
- `<datatable>/<schema>:<table>` - Table in specific schema

### Querying in TypeScript (Bun/Deno)

```typescript
import * as wmill from 'windmill-client';

export async function main(user_id: string) {
  const sql = wmill.datatable();  // Or: wmill.datatable('other_datatable')

  // Parameterized queries (safe from SQL injection)
  const user = await sql`SELECT * FROM users WHERE id = ${user_id}`.fetchOne();
  const users = await sql`SELECT * FROM users WHERE active = ${true}`.fetch();

  // Insert/Update
  await sql`INSERT INTO users (name, email) VALUES (${name}, ${email})`;
  await sql`UPDATE users SET name = ${newName} WHERE id = ${user_id}`;

  return user;
}
```

### Querying in Python

```python
import wmill

def main(user_id: str):
    db = wmill.datatable()  # Or: wmill.datatable('other_datatable')

    # Use $1, $2, etc. for parameters
    user = db.query('SELECT * FROM users WHERE id = $1', user_id).fetch_one()
    users = db.query('SELECT * FROM users WHERE active = $1', True).fetch()

    # Insert/Update
    db.query('INSERT INTO users (name, email) VALUES ($1, $2)', name, email)
    db.query('UPDATE users SET name = $1 WHERE id = $2', new_name, user_id)

    return user
```

## SQL Migrations (sql_to_apply/)

The `sql_to_apply/` folder is for creating/modifying database tables during development.

### Workflow

1. Create `.sql` files in `sql_to_apply/`
2. Run `wmill app dev` - the dev server watches this folder
3. When SQL files change, a modal appears in the browser to confirm execution
4. After creating tables, **add them to `data.tables`** in `raw_app.yaml`

### Example Migration

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

### Migration Best Practices

- **Use idempotent SQL**: `CREATE TABLE IF NOT EXISTS`, etc.
- **Number files**: `001_`, `002_` for ordering
- **Always whitelist tables** after creation
- This folder is NOT synced - it's for local development only

## CLI Commands

`wmill app new` is the exception: you run it yourself, with flags, per the "Creating a Raw App" section above.

For everything else, tell the user which command fits their intent and let them run it — these touch the workspace or local lock files, and the user should consent each time:

| Command | Description |
|---------|-------------|
| `wmill app dev` | Start dev server with live reload (see the `preview` skill for the full open-the-app-in-the-IDE-pane procedure). |
| `wmill app generate-agents` | Refresh AGENTS.md and DATATABLES.md |
| `wmill generate-metadata` | Generate lock files for backend runnables |
| `wmill sync push` | Deploy app to Windmill |
| `wmill sync pull` | Pull latest from Windmill |

## Best Practices

1. **Check DATATABLES.md** for existing tables before creating new ones
2. **Use parameterized queries** - never concatenate user input into SQL
3. **Keep runnables focused** - one function per file
4. **Use descriptive IDs** - `get_user.ts` not `a.ts`
5. **Always whitelist tables** - add to `data.tables` before querying
6. **Generate locks** - tell the user to run `wmill generate-metadata` after adding/modifying backend runnables
