# UserDraft Path Key Investigation — Findings

Response to `docs/userdraft-path-key-investigation-prompt.md`.

## 1. Editors that use empty/new-item paths today

| Kind | Where empty path is used | Path source after typing | Notes |
|---|---|---|---|
| `script` | `routes/scripts/add/+page.svelte:111` — `UserDraft.use<Script>('script', '', …)` | `script.path` field **inside** draft value | `?nodraft=true` wipe at line 75; URL-hash sync mirrors edits. No `UserDraft.remove('script', '')` on `onSaveInitial`/`onDeploy` (cf. flows). |
| `flow` | `routes/flows/add/+page.svelte:76` — `UserDraft.use<Flow>('flow', '', …)` | `flow.path` field **inside** draft value | `onSaveInitial`/`onDeploy` both call `UserDraft.remove('flow', '')`. |
| `app` | `apps/editor/AppEditor.svelte:87-88` — `appDraftPath = newApp ? '' : (path ?? '')` | Path is **not** in `App` value; lives in route state, decided server-side at save. `/apps/edit/{path}` route uses real path. | `apps/add/+page.svelte` wipes on `nodraft`/import/template/hub. |
| `raw_app` | `apps_raw/add/+page.svelte:77-82` — `UserDraft.use('raw_app', '')`. Editor header binds `newEditedPath` separately. | Path lives in `newEditedPath` $state in `RawAppEditorHeader`, **not** in the draft value `{files, runnables, data, summary}`. | Discard on save: `UserDraft.remove('raw_app', path)` with the real path string. |
| `resource` | `ResourceEditor.svelte:106-113` — uses `initialPath ?? ''` (which is `''` when route opens with `path=''`) | `current.path` is `state.path` field **inside** draft value (`ResourceState.path`). | `useMany` is per-workspace fan-out; key is shared across all workspace specs at the empty path. |
| `variable` | `VariableEditor.svelte:82-89` — `initNew()` leaves `editPath = undefined` → key is `''` | `current.path` field **inside** `VariableState`. | Same per-workspace fan-out. |
| `trigger_schedule` and the 9 `trigger_*` kinds | **No empty-path entry at all.** `useTriggerDraftSync` (`useTriggerDraftSync.svelte.ts:59-63`) gates `useMany` on `p && ws` — when `initialPath = ''`, `getSpecs` returns `[]` and no entry is acquired. | Path lives in `path` $state in the editor inner. | New-trigger work isn't autosaved at all — only edits to an *already-saved* trigger are persisted. |

So **only `script`, `flow`, `app`, `raw_app`, `resource`, `variable`** ever write empty-path keys. The 10 trigger kinds already have nothing to migrate.

## 2-9. Per-kind feasibility

| Kind | When is real path known? | Can it be known before `UserDraft.use(...)`? | Safe to rebind `''` → `actualPath`? | What to do with the old empty entry? | Breakage risk for reload restore | Risk for live state / debounced persist / staleness meta | Special caveats |
|---|---|---|---|---|---|---|---|
| `script` | Eagerly: every draft contains `script.path` (default `''`, updated as user types). | **No.** The `''` IS the initial path; we'd be using stale state. We'd need a reactive `useMany(() => script.path ? [script.path] : [''])` reconcile. | Risky — `useMany` reconcile would *release* the empty entry and *acquire* the path entry. There's no built-in "move value" — the new entry starts with `defaultValue` and the in-memory `value` of the old entry is gone. We'd need a helper `UserDraft.rename(kind, oldPath, newPath)` or pass the existing draft as the new entry's `defaultValue`. | Move (don't copy): bug-free user model is "this is one draft, its identity is the path field." Discard the empty entry once content has been transplanted into the path entry. | Medium — `/scripts/add` plain reload restores from the empty entry. After rebind, reload of `/scripts/add` (still routed under `add`, no path in URL) wouldn't find the just-renamed entry. **Need** the route to read by `script.path` from URL-hash sync, OR keep the empty entry as a "current new-item" alias that mirrors. | URL-hash sync (lines 152-168) keeps edits in the URL; rebind during user typing would cause many entries (one per keystroke). MUST debounce the rebind or rebind only on blur/save. Staleness meta is irrelevant for unsaved new items (no `remoteRev`). | Path is user-editable mid-session; a path collision with an existing script's draft is possible. Must define merge policy. |
| `flow` | Eagerly: `flow.path` inside draft value. | **No.** Same as script. | Same as script — needs a rename primitive. | Same as script. | Same as script — `flows/add` route relies on empty entry for reload. | Same as script. URL-hash sync exists for flows too via fork-state but is less aggressive than script's. | `pathStoreInit`/`initialPath` (fork/hub flows) seed a non-empty path that would also be a candidate as the autosave key. |
| `app` | **Only at save** (`onSavedNewAppPath`). The `App` value has no `path` field; the user types it inside `RawAppEditorHeader`-style summary dialog at deploy time. | **No.** The user could type a path in a sidebar before save, but currently the path input is inside the save drawer. Plumbing it out is a real UX change. | Possible only after `path` becomes visible to the editor — i.e., after the save drawer is opened. Before that the editor has no path. | If we move the path input upstream (header), we can rebind on first character typed. Otherwise, leave the empty entry as-is until save. | High — `/apps/add` reload restores the empty entry. If we rebind too early on a path the user discards, the path-keyed entry persists and the user gets a confused state. | Live state is fine to swap (handle is reactive). Debounce concern: AppEditor has `firstMirror` skip semantics — rebinding mid-session would re-arm the skip. Staleness meta is irrelevant pre-save. | **This is the hardest one.** Recommend NOT applying this direction to `app` unless the path UX is moved out of the save drawer. |
| `raw_app` | Same as `app` — path lives in `newEditedPath` in the header, not in the draft value. | Same as `app` — depends on plumbing. | Same as `app`. | Same as `app`. | High — `apps_raw/add` reload restores the empty entry; multiple branches in `loadApp` call `UserDraft.discard('raw_app', '', undefined)` for "start fresh." | Same as `app`. Adapter writes via `saveGlobalAppDraft(workspace, path, value)` already keys by real path; only the editor's empty entry is the holdout. | Same as `app`. |
| `resource` | Eagerly: `ResourceState.path` inside draft value (user types into form). | **No.** New-item starts at `path = ''`. | Yes, **easier than script/flow** because the editor is a drawer, not a route — no reload-restore semantics to preserve. | Move on first non-empty `path` (debounced). | Low — there's no `/resources/add` reload; the drawer is always re-opened by `initNew()`. | Per-workspace fan-out (`useMany`) — rebind must apply to ALL `workspaceSpecs` entries together (or it'll split a single conceptual draft into N path-keyed and M empty-keyed entries). | Path is also bound to the outer prop (`$effect` at line 316: `path = current.path`). |
| `variable` | Eagerly: `VariableState.path` inside draft value. | **No.** Same as resource. | Yes, same as resource — drawer-based, no reload restore. | Same as resource. | Low — same reason as resource. | Same per-workspace fan-out concern as resource. | Same as resource. |
| `trigger_*` (10 kinds) | Already only autosaves when path is known. | N/A. | N/A — no empty-path entry exists. | N/A. | N/A. | N/A. | **No work needed.** |

## 10. Tests to add before the change

- `userDraft.test.ts`: add a `UserDraft.rename(kind, oldPath, newPath)` primitive (if introduced) covering: live-handle preservation, meta carry-over, no-op when paths equal, collision behavior (existing dest), refcount preservation, debounced-write timer carryover.
- For each migrating kind: an editor-level test that types a path, verifies the LS slot moves from `''` to the typed path, verifies no orphan `''` slot is left behind, verifies the global chat `listGlobalDrafts(ws)` reports the typed path (not `''`).
- Reload-restore test for `scripts/add` and `flows/add`: confirms a fresh page load still surfaces the unsaved work, either via URL-hash (existing for scripts) or via a "most-recent new-item" pointer (new mechanism if needed).
- Collision test: existing script at `u/me/foo` has its own UserDraft entry; user starts a new script and types `u/me/foo` — expected behavior must be defined (refuse rebind, or overlay with warning).
- Staleness test: rebinding after `remoteRev` was attached at the empty path should drop the meta (irrelevant — meta belongs to a backend item; the empty path has no backend item).

## Recommended migration strategy for existing empty-path drafts

- On `gcUserDrafts` (or a one-off boot sweep): scan `userdraft/w/{ws}/{kind}/` keys with empty trailing path. For kinds where path is in the value (`script`/`flow`/`resource`/`variable`), try to `value.path`; if non-empty and the path-keyed slot is absent, rewrite under the path-keyed key. Otherwise leave alone.
- For `app`/`raw_app`: cannot migrate (no path in value). Leave as `''` until the user opens `/apps/add`; the existing restore-from-empty behavior continues to work.
- Add a schema-version field in `StoredDraft` so future migrations are detectable. The current `StoredDraft<V>` shape has no version field — bump now while you're touching it.

## Minimal staged implementation plan (safest first)

1. **No-op for `trigger_*`** — already correct. Add a one-line comment in `useTriggerDraftSync` documenting that empty-path triggers are intentionally NOT persisted, so the next reviewer doesn't "fix" it.
2. **`variable`** (lowest risk: drawer-based, no reload semantics, no URL-hash sync). Add a reactive `useMany` reconcile keyed off `current.path` (when non-empty) instead of `editPath ?? ''`. Verify per-workspace fan-out still works.
3. **`resource`** — same change pattern as variable. The two editors are structurally identical; doing variable first lets you copy.
4. **Introduce `UserDraft.rename(kind, oldPath, newPath, opts)`** as a real API (currently you'd have to read-then-save-then-remove, which loses debounced writes). Cover with `userDraft.test.ts`.
5. **`script`** and **`flow`** — switch their `add` routes to reactive `useMany` keyed off `script.path`/`flow.path`; rebind on debounce/blur, not on every keystroke. Handle URL-hash sync interaction carefully (especially scripts). Provide a `?lastNewItem={path}` URL parameter or a small `localStorage.setItem('userdraft/lastNew/{kind}', path)` pointer so a plain reload of `/scripts/add` can find the most recent path-keyed entry.
6. **`app` and `raw_app`** — defer. Only attempt after moving the path input out of the save drawer into the header, so the editor has a known path during the session. If you don't move the input, leave the empty key as-is.

## Cases where this direction should NOT be applied

- **`app` and `raw_app` as they stand today.** Path is decided server-side at save; the editor has no concept of "user's chosen path" before deploy. Rebinding would require a UX change. The current empty-path behavior is correct.
- **`trigger_*` for new items.** They already don't autosave; rebinding has nothing to rebind from.
- **Any kind during a route reload window.** The `''` key is doubling as a "current-new-item pointer." Until that pointer is replaced (URL param, separate `lastNew/{kind}` key, or session-scoped flag), removing the empty-key behavior breaks reload restore.

## Summary

Empty-path drafts exist for 6 kinds (`script`, `flow`, `app`, `raw_app`, `resource`, `variable`); the 10 trigger kinds are already path-keyed. Of the 6, `variable` and `resource` are cheap wins (drawer-based, path is in value, no reload semantics). `script` and `flow` are doable but need a rename primitive plus a "last new-item" pointer for reload restore. `app` and `raw_app` should be skipped until the path UX is restructured. The biggest hidden contract: the empty key isn't just a missing path — it doubles as the "current new-item" pointer that survives page reloads. Whatever replaces it must restore that signal.
