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
<!-- cli-only -->
`wmill app new --datatable <name> --schema <name>` records both and, for a schema that does not exist yet, writes the migration in `sql_to_apply/` that creates it: nothing exists in the database until `wmill app dev` applies that migration, so the app's table migrations come after it.
<!-- /cli-only -->
<!-- chat-only -->

### An app that arrives already set up

An app the user just started from the new-app dialog is already set up: its files are the framework template it was started with, and a `data` config it carries (`data.datatable`, `data.schema` or `data.tables`) is the data setup it was started with. Build that app in place, with that data config, and in that framework unless the user now names another. Do not ask for either, and do not create a second app. Only when it carries no data config and has to store data is the data setup still to ask.
<!-- /chat-only -->

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
<!-- cli-only -->
On disk, a runnable's language is determined by its backend file extension.
<!-- /cli-only -->

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
<!-- cli-only -->
`public: true` in `raw_app.yaml` deploys the app for anonymous users, and `guests: true` for guests. Remove the line and the next push closes the app again.
<!-- /cli-only -->
<!-- chat-only -->
When a deploy widens who may open the app, tell the user in plain words.
<!-- /chat-only -->

## Data Tables

Data tables are PostgreSQL databases managed by Windmill. Backend runnables query them via the `wmill` client; the frontend never queries them directly. **When the app needs to store or persist data** (user data, settings, application state, records, logs), use a data table.

### Critical rules

1. **Check what exists first**: look up the workspace's data tables and their tables before designing storage, and reuse a suitable table rather than creating another. Never assume a `main` data table exists.
2. **Whitelisted tables only**: a runnable can only query tables listed in the app's `data.tables` config. Queries against unlisted tables fail at runtime, so register a new table there before using it.
3. **No DDL inside runnables**: runnables only read and write rows (SELECT, INSERT, UPDATE, DELETE) on existing tables. Never CREATE, ALTER or DROP a table from a runnable.
4. **Qualify table names**: an unqualified name means the `public` schema, so write every other table as `schema.table`, in table creation and queries alike. The app's `data` config sets the default datatable and schema its tables go in; use them consistently across runnables.
5. **Pass the role**: when the app's `data.roles` gives a data table a role, every `wmill.datatable` call on it passes that role (`wmill.datatable('main', { role: 'analyst' })` in TypeScript, `wmill.datatable('main', role='analyst')` in Python). The role only reaches what it was granted, so a query outside it fails with `permission denied`.
<!-- cli-only -->

`wmill datatable list` lists the workspace's data tables. Create or change tables with a migration in `sql_to_apply/` (see "SQL Migrations" above), then add them to `data.tables` in `raw_app.yaml`.
<!-- /cli-only -->

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
