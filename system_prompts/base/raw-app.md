# Windmill Raw Apps

Raw apps let you build custom frontends with React, Svelte, or Vue that connect to Windmill backend runnables and datatables.

## App shape

A raw app has three logical parts:

- **Frontend** — bundled with esbuild from `index.tsx` as the entrypoint. Files include the entrypoint, components (`App.tsx`), styles, etc.
- **Backend runnables** — server-side scripts the frontend calls, each addressed by a unique key.
- **Data** — optional whitelisted datatables (managed PostgreSQL) that the backend runnables can query. The frontend never queries the database directly; backend runnables are the only bridge.

## Frontend

### Entrypoint

`index.tsx` is the bundling entrypoint (the bundler is esbuild) and the **mount** entrypoint. The preview executes the bundle against an empty `<div id="root">` and auto-renders nothing, so `index.tsx` must mount a top-level `App` itself. Keep the UI in `App.tsx` (or `App.svelte` / `App.vue`) and keep `index.tsx` as the mount shim:

```tsx
import React from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'

createRoot(document.getElementById('root')!).render(<App />)
```

Svelte: `mount(App, { target: document.getElementById('root')! })`. Vue: `createApp(App).mount('#root')`.

**Never replace `index.tsx` with a bare component** (`export default function App() { ... }` and no mount call). A component that is defined but never mounted renders a blank screen with **no error thrown** — the JSX never executes, so nothing reaches the console or the error overlay. If an app renders blank, check that `index.tsx` still calls `createRoot(document.getElementById('root')!).render(<App />)`.

**Always begin every React file (`.tsx`/`.jsx`) that uses JSX with `import React from 'react'`.** esbuild uses the classic JSX transform, so `React` must be in scope wherever JSX appears — a missing import compiles fine but throws `React is not defined` at runtime, leaving a blank screen.

### Generated bindings (`wmill.d.ts` / `wmill.ts`)

The frontend imports a generated module that mirrors the backend runnables. **Never write to it directly** — it gets regenerated whenever backend runnables change. Modifying it by hand will be overwritten.

### Calling backend runnables

Import the generated bindings and call the runnable like a function:

```typescript
import { backend } from './wmill';

// Call a backend runnable
const user = await backend.get_user({ user_id: '123' });
```

The frontend cannot reach datatables, workspace items, or external services on its own — it goes through `backend.<key>(args)` for everything server-side.

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

### Path runnables (script / flow / hubscript)

When `type` is `script`, `flow`, or `hubscript`, the runnable just stores a `path` to an existing workspace or hub item — no inline code. The referenced item's input/output schema becomes the runnable's surface.

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
