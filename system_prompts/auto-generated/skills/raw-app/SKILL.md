---
name: raw-app
description: MUST use when creating raw apps.
---

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

# Windmill Raw Apps

Raw apps let you build custom frontends with React, Svelte, or Vue that connect to Windmill backend runnables and datatables.

## App shape

A raw app has three logical parts:

- **Frontend** — bundled with esbuild from `index.tsx` as the entrypoint. Files include the entrypoint, components (`App.tsx`), styles, etc.
- **Backend runnables** — server-side scripts the frontend calls, each addressed by a unique key.
- **Data** — optional whitelisted datatables (managed PostgreSQL) that the backend runnables can query. The frontend never queries the database directly; backend runnables are the only bridge.

## Starting a new app

A new app gets the setup Windmill's own new-app dialog gives one: a framework, then where its data lives.

### Framework

Use the framework the user named. When they named none, follow what there is to follow: the app being edited, or the apps the request points at. With nothing to follow, use **React 19**, the recommended default, **without asking**. React 18, Svelte 5 and Vue are for when they are asked for.

### Data setup

When a new app has to store data and the user did not say where, **ask before creating anything**: no app, no schema, no table. Where the data lives is the user's decision, and tables created in the wrong place stay there. Look up the workspace's data tables first so every choice you offer exists, then ask, as proposed answers rather than open questions:

1. **Data table**: which one to use, the default first (`main` when it exists, otherwise the first listed). Skip this one when the workspace has a single data table.
2. **Schema**: where the app's new tables go.
   - A **new schema**, the usual choice, offered first. Suggest the first unused name among `app1`, `app2`, …
   - An **existing schema** of that data table, by name.
   - **None**: the tables go in `public`.
3. **Tables**: only when that data table already holds tables the app could use, whether to **reuse** them (name them) or **create new ones**.

Do not ask when:

- the user already said where the data goes. A part they left out takes its default above: the default data table, a new `appN` schema, new tables;
- the app, or its saved draft, already carries a data config (`data.datatable`, `data.schema` or `data.tables`). That config is the answer;
- the app stores nothing.

With no data table in the workspace there is nothing to choose: say that one has to be configured in the workspace settings before the app can store data.

A question left unanswered or dismissed is not a go-ahead for the defaults: stop there and say what is still needed.

Then build what was chosen: the data table and schema become the app's `data.datatable` and `data.schema`, the tables it uses, reused or new, go in `data.tables`, and a new schema is created (`CREATE SCHEMA IF NOT EXISTS`) before the tables that go in it.
`wmill app new --datatable <name> --schema <name>` records both and, for a schema that does not exist yet, writes the migration in `sql_to_apply/` that creates it: nothing exists in the database until `wmill app dev` applies that migration, so the app's table migrations come after it.

## Frontend

### Entrypoint

The entrypoint is `index.tsx` for React and `index.ts` for Svelte and Vue. It is both the bundling entrypoint (the bundler is esbuild) and the **mount** entrypoint: the preview executes the bundle against an empty `<div id="root">` and auto-renders nothing, so the entrypoint must mount a top-level `App` itself. Keep the UI in `App.tsx` / `App.svelte` / `App.vue` and keep the entrypoint as the mount shim.

React (`index.tsx`):

```tsx
import React from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'

createRoot(document.getElementById('root')!).render(<App />)
```

Svelte (`index.ts`): `mount(App, { target: document.getElementById('root')! })`. Vue (`index.ts`): `createApp(App).mount('#root')`.

**Never replace the entrypoint with a bare component** (`export default function App() { ... }` and no mount call). A component that is defined but never mounted renders a blank screen with **no error thrown** — it never executes, so nothing reaches the console or the error overlay. If an app renders blank, check that the entrypoint still mounts `App` into `#root`.

**Always begin every React file (`.tsx`/`.jsx`) that uses JSX with `import React from 'react'`.** esbuild uses the classic JSX transform, so `React` must be in scope wherever JSX appears — a missing import compiles fine but throws `React is not defined` at runtime, leaving a blank screen.

### Generated bindings (`wmill.d.ts` / `wmill.ts`)

The frontend imports a generated module that mirrors the backend runnables. **Never write to it directly** — it gets regenerated whenever backend runnables change. Modifying it by hand will be overwritten.

### Calling backend runnables

Import the generated bindings and call the runnable like a function. `./wmill` is the **only** way the frontend reaches anything server-side — datatables, workspace items, external services. Never `fetch` the Windmill API from frontend code: the bundle holds no token and builds no API URL.

| Export | Resolves to | Use it for |
|---|---|---|
| `backend.<key>(args)` | the runnable's result | the default — run and wait |
| `backendAsync.<key>(args)` | the **job id** (a string) | long-running work you want to track |
| `waitJob(jobId)` | the job's **result** (rejects if the job failed) | awaiting a `backendAsync` job |
| `getJob(jobId)` | a `Job` (`{ type, success, result, duration_ms, ... }`) | polling status without blocking |
| `streamJob(jobId, onUpdate?)` | the final result, calling `onUpdate` per chunk | showing output as it is produced |

A runnable is always called with **one object** whose keys are its `main` parameters — `main(user_id: string, limit: number)` is called as `backend.get_users({ user_id, limit })`, never with positional arguments. A runnable without parameters is called with no argument. Resource and variable ids handed to the `wmill` client are paths (`u/<user>/<name>` or `f/<folder>/<name>`).

Run and wait — the common case:

```tsx
import { backend } from './wmill';

const user = await backend.get_user({ user_id: '123' });
```

Start a long job, then await it:

```tsx
import { backendAsync, waitJob } from './wmill';

const jobId = await backendAsync.run_report({ month: '2026-08' }); // a string
const report = await waitJob(jobId);                               // the result itself
```

Or poll it without blocking, to render progress:

```tsx
import { getJob } from './wmill';

const job = await getJob(jobId);
if (job.type === 'CompletedJob') setReport(job.result);
```

`backendAsync` resolves a job id and nothing else — guard on it before storing or polling. A poll loop started on an `undefined` id never completes and shows as a row stuck "running" forever:

```tsx
const jobId = await backendAsync.run_report(args);
if (!jobId) throw new Error('run_report did not start a job');
```

**Never hand-write a job-polling runnable.** A backend runnable that calls `jobs/list`, or that returns `getResultMaybe(...)` for the frontend to poll, reimplements `backendAsync` + `waitJob` / `getJob` / `streamJob` — and it is what leads to guessing at base URLs and tokens.

### Keeping data out of recorded demos

An app can be demoed by recording a session: every interaction becomes a step carrying a snapshot of the page, replayed publicly or on the Hub. Password inputs are masked automatically. Mark anything else that must not appear with `data-wm-no-record` — the whole marked subtree is dropped from every snapshot, along with its values and the step's own metadata:

```tsx
<label data-wm-no-record>
  Customer SSN <input value={ssn} onChange={onSsn} />
</label>
```

Apply it to customer data, internal notes and anything else a viewer of the demo should not see. It costs nothing when the app is never recorded.

### Chat UIs over a flow in chat mode

A flow deployed with chat mode on is a chat backend (streaming answer, tool calls, memory, conversation history). Do not drive it through a runnable: add `windmill-chat` to `package.json` and use it directly, it detects the app's Windmill and credential.

```tsx
import { useWindmillChat } from 'windmill-chat/react'

const chat = useWindmillChat({ flowPath: 'f/support/assistant' })
// chat.messages ({ role, content, pending, success, tool? }), chat.status, chat.sendMessage(text), chat.stop()
```

`windmill-chat/ai-sdk` gives a `ChatTransport` for the Vercel AI SDK's `useChat`, `windmill-chat/assistant-ui` a runtime for assistant-ui. The flow must be deployed, not a draft. A sandboxed app needs `jobs:run` in its frontend SDK scopes, plus `flow_conversations:write` for the conversation sidebar; without them the chat keeps history in the browser.

## Backend runnables

Each runnable has a unique key (used to call it from the frontend) and one of four types:

| Type | What it is |
|---|---|
| `inline` | Custom code stored on the app itself. Most common for app-specific logic. |
| `script` | Reference to an existing workspace script by path. |
| `flow` | Reference to an existing workspace flow by path. |
| `hubscript` | Reference to a hub script by path. |

### Inline runnables

Inline runnables carry their own source code, and must expose a `main` function as their entrypoint.
On disk, a runnable's language is determined by its backend file extension.

**TypeScript example** (runnable `get_user`):

```typescript
import * as wmill from 'windmill-client';

export async function main(user_id: string) {
  const sql = wmill.datatable();
  const user = await sql`SELECT * FROM users WHERE id = ${user_id}`.fetchOne();
  return user;
}
```

**Python example** (runnable `get_user`):

```python
import wmill

def main(user_id: str):
    db = wmill.datatable()
    user = db.query('SELECT * FROM users WHERE id = $1', user_id).fetch_one()
    return user
```

#### The `wmill` client is already authenticated

An inline runnable runs as an ordinary Windmill job. `import * as wmill from 'windmill-client'` (TypeScript) and `import wmill` (Python) are already pointed at this instance and this workspace — there is nothing to configure.

**Don't read `WM_TOKEN` or `BASE_INTERNAL_URL` and build an API URL to `fetch`.** The client's own `setClient` already reads exactly those, and it also sets the credentials mode a raw app needs (`WM_RAW_APP` suppresses credentials, because a sandboxed bundle calls the API from an opaque origin that can never pair with `Access-Control-Allow-Origin: *`). Rebuilding that by hand drops the parts you can't see. Use `wmill.*` for everything Windmill, and `fetch` only for third-party APIs.

Use only `wmill` functions the SDK actually exports; for an endpoint none of them covers, the generated service classes (`JobService`, `ScriptService`, ...) are importable from `windmill-client`. What is not available is a name you guessed at: `getBaseUrl` and `getWorkspaceToken` are inventions, not API.

### Path runnables (script / flow / hubscript)

When `type` is `script`, `flow`, or `hubscript`, the runnable just stores a `path` to an existing workspace or hub item — no inline code. The referenced item's input/output schema becomes the runnable's surface.

Before writing an inline runnable, look for a workspace script or flow that already does the job, or a Hub script (a prebuilt integration at `hub/<version>/<app>/<name>`) for a third-party service, and reference it instead of copying its logic.

### Draft code vs deployed code

This decides whether an app works before anything is deployed:

- **Inline runnables run the app's current code.** The editor sends the runnable's source with each request, so an inline runnable works in the preview with nothing deployed.
- **Path runnables (`script` / `flow` / `hubscript`) run the DEPLOYED item at that path.** So do `wmill.runFlow`, `wmill.runFlowAsync` and `wmill.runScriptByPath` called from inside a runnable. A draft — including a draft you just created — does not exist for them.

So an app wired to a flow you just wrote does nothing until **that flow is deployed**. The app itself does NOT have to be deployed for this: the preview runs the app's draft, so the referenced flow is the only thing that has to exist deployed.

That makes the fix a one-item deploy, not a release. Offer to deploy exactly the referenced flow or script and leave the app a draft the user keeps testing in the preview — do not push the whole change set through the review-and-deploy page, and do not ask the user to deploy the app, unless they said they want to ship it.

Do NOT quietly reimplement the flow inside an inline runnable to dodge the deployment: that leaves the user with two copies of the same logic and an app that ignores the flow they asked for. Inline the logic only when the user actually wants it inline.

Prefer a **path runnable of type `flow`** over an inline runnable that calls `wmill.runFlowAsync`. The path runnable gives the frontend the flow's real input schema and works with `backend` / `backendAsync` / `waitJob` like any other runnable; a hand-written wrapper gives up all of that.

### Static inputs

`staticInputs` is an optional `Record<string, any>` for arguments not overridable from the frontend. Useful with path runnables to pre-fill some args while leaving the rest to the frontend caller.

## Who can open a deployed app

A draft app is reachable by nobody; deploying is what exposes it, and its backend runnables with it. Otherwise only users with access to the app can open it, unless the app is opened to:

- **anonymous** users: anyone with the URL, without logging in;
- **guests**: anyone the instance's identity provider authenticates, whether or not they belong to the workspace.

Either one lets those people run the app's backend runnables, so never open an app up unless the user asks.
`public: true` in `raw_app.yaml` deploys the app for anonymous users, and `guests: true` for guests. Remove the line and the next push closes the app again.

## Data Tables

Data tables are PostgreSQL databases managed by Windmill. Backend runnables query them via the `wmill` client; the frontend never queries them directly. **When the app needs to store or persist data** (user data, settings, application state, records, logs), use a data table.

### Critical rules

1. **Check what exists first**: look up the workspace's data tables and their tables before designing storage, and reuse a suitable table rather than creating another. Never assume a `main` data table exists.
2. **Whitelisted tables only**: a runnable can only query tables listed in the app's `data.tables` config. Queries against unlisted tables fail at runtime, so register a new table there before using it.
3. **No DDL inside runnables**: runnables only read and write rows (SELECT, INSERT, UPDATE, DELETE) on existing tables. Never CREATE, ALTER or DROP a table from a runnable.
4. **Qualify table names**: an unqualified name means the `public` schema, so write every other table as `schema.table`, in table creation and queries alike. The app's `data` config sets the default datatable and schema its tables go in; use them consistently across runnables.
5. **Pass the role**: when the app's `data.roles` gives a data table a role, every `wmill.datatable` call on it passes that role (`wmill.datatable('main', { role: 'analyst' })` in TypeScript, `wmill.datatable('main', role='analyst')` in Python). The role only reaches what it was granted, so a query outside it fails with `permission denied`.

`wmill datatable list` lists the workspace's data tables. Create or change tables with a migration in `sql_to_apply/` (see "SQL Migrations" above), then add them to `data.tables` in `raw_app.yaml`.

### Querying in TypeScript (Bun/Deno)

```typescript
import * as wmill from 'windmill-client';

export async function main(user_id: string) {
  const sql = wmill.datatable();  // Or: wmill.datatable('other_datatable')

  // Parameterized queries (safe from SQL injection)
  const user = await sql`SELECT * FROM users WHERE id = ${user_id}`.fetchOne();
  const users = await sql`SELECT * FROM users WHERE active = ${true}`.fetch();

  // Insert/Update
  await sql`INSERT INTO users (name, email) VALUES (${name}, ${email})`.execute();
  await sql`UPDATE users SET name = ${newName} WHERE id = ${user_id}`.execute();

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
    db.query('INSERT INTO users (name, email) VALUES ($1, $2)', name, email).execute()
    db.query('UPDATE users SET name = $1 WHERE id = $2', new_name, user_id).execute()

    return user
```

## Best Practices

1. **Check existing tables** before creating new ones — reuse beats schema growth.
2. **Use parameterized queries** — never concatenate user input into SQL.
3. **Terminate every datatable statement** — the tagged template and `db.query(...)` only build a statement. It runs when you call `fetch` / `fetchOne` / `fetchOneScalar` / `execute` (`fetch` / `fetch_one` / `fetch_one_scalar` / `execute` in Python). An INSERT or UPDATE without one writes nothing and raises nothing. Awaiting the statement itself is a no-op — it is not a promise.
4. **Keep runnables focused** — one function per runnable; small surface area.
5. **Use descriptive keys** — `get_user`, not `a`.
6. **Always whitelist tables** — adding a runnable that queries a new table requires the table to be in `data.tables` first.
7. **Mark sensitive UI with `data-wm-no-record`** — it is what keeps that data out of a recorded demo; passwords are handled for you.
8. **Reach for `backendAsync` + `waitJob`** for long work — never a hand-written job-polling runnable.
9. **Deploy what a path runnable points at** — a path runnable aimed at a draft fails at runtime; tell the user what needs deploying.
10. **Use `windmill-chat` for a chat over a chat-mode flow** — never a runnable that runs the flow and polls its stream.
