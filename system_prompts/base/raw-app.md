# Windmill Raw Apps

Raw apps let you build custom frontends with React, Svelte, or Vue that connect to Windmill backend runnables and datatables.

## App shape

A raw app has three logical parts:

- **Frontend** — bundled with esbuild from `index.tsx` as the entrypoint. Files include the entrypoint, components (`App.tsx`), styles, etc.
- **Backend runnables** — server-side scripts the frontend calls, each addressed by a unique key.
- **Data** — optional whitelisted datatables (managed PostgreSQL) that the backend runnables can query. The frontend never queries the database directly; backend runnables are the only bridge.

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

## Backend runnables

Each runnable has a unique key (used to call it from the frontend) and one of four types:

| Type | What it is |
|---|---|
| `inline` | Custom code stored on the app itself. Most common for app-specific logic. |
| `script` | Reference to an existing workspace script by path. |
| `flow` | Reference to an existing workspace flow by path. |
| `hubscript` | Reference to a hub script by path. |

### Inline runnables

Inline runnables carry their own source code. For file-based raw apps, the runnable language is determined by the backend file extension. The script must expose a `main` function as its entrypoint.

**TypeScript example** (`backend/get_user.ts`):

```typescript
import * as wmill from 'windmill-client';

export async function main(user_id: string) {
  const sql = wmill.datatable();
  const user = await sql`SELECT * FROM users WHERE id = ${user_id}`.fetchOne();
  return user;
}
```

**Python example** (`backend/get_user.py`):

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

Prefer the `wmill` functions that appear in the SDK reference; for an endpoint none of them covers, the generated service classes (`JobService`, `ScriptService`, ...) are importable from `windmill-client`. What is not available is a name you guessed at: `getBaseUrl` and `getWorkspaceToken` are inventions, not API.

### Path runnables (script / flow / hubscript)

When `type` is `script`, `flow`, or `hubscript`, the runnable just stores a `path` to an existing workspace or hub item — no inline code. The referenced item's input/output schema becomes the runnable's surface.

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

## Data Tables

Data tables are PostgreSQL databases managed by Windmill. Backend runnables query them via the `wmill` client; the frontend never queries them directly.

### Critical rules

1. **Whitelisted tables only**: a runnable can only query tables listed in the app's `data.tables` config. Tables not in this list are not accessible.
2. **Add tables before using**: queries against unlisted tables fail at runtime. When you introduce a new table, register it in `data.tables` first.
3. **Use the configured datatable/schema**: the app's `data` config sets the default datatable and schema; reference them consistently across runnables.

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

## Best Practices

1. **Check existing tables** before creating new ones — reuse beats schema growth.
2. **Use parameterized queries** — never concatenate user input into SQL.
3. **Keep runnables focused** — one function per runnable; small surface area.
4. **Use descriptive keys** — `get_user`, not `a`.
5. **Always whitelist tables** — adding a runnable that queries a new table requires the table to be in `data.tables` first.
6. **Mark sensitive UI with `data-wm-no-record`** — it is what keeps that data out of a recorded demo; passwords are handled for you.
7. **Reach for `backendAsync` + `waitJob`** for long work — never a hand-written job-polling runnable.
8. **Deploy what a path runnable points at** — a path runnable aimed at a draft fails at runtime; tell the user what needs deploying.
