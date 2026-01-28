---
name: raw-app
description: Create raw apps with React/Svelte/Vue frontend and backend runnables.
---

# Windmill Raw Apps

Raw apps let you build custom frontends with React, Svelte, or Vue that connect to Windmill backend runnables and datatables.

## Creating a Raw App

```bash
wmill app new
```

This interactive command creates a complete app structure with your choice of frontend framework (React, Svelte, or Vue).
This step MUST be done by the user, if asked to create a new raw app, you should tell the user to run that command first.

## App Structure

```
my_app.raw_app/
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
│   └── <id>.lock          # Lock file (run 'wmill app generate-locks' to create)
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

After creating, generate lock files:
```bash
wmill app generate-locks
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

| Command | Description |
|---------|-------------|
| `wmill app new` | Create a new raw app interactively |
| `wmill app dev` | Start dev server with live reload |
| `wmill app generate-agents` | Refresh AGENTS.md and DATATABLES.md |
| `wmill app generate-locks` | Generate lock files for backend runnables |
| `wmill sync push` | Deploy app to Windmill |
| `wmill sync pull` | Pull latest from Windmill |

## Best Practices

1. **Check DATATABLES.md** for existing tables before creating new ones
2. **Use parameterized queries** - never concatenate user input into SQL
3. **Keep runnables focused** - one function per file
4. **Use descriptive IDs** - `get_user.ts` not `a.ts`
5. **Always whitelist tables** - add to `data.tables` before querying
6. **Generate locks** - run `wmill app generate-locks` after adding/modifying backend runnables
