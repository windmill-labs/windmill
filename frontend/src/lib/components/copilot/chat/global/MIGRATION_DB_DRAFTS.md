# Migrating the AI chat global mode to DB-backed drafts (PR #9351)

Working notes for moving the global chat mode off localStorage-backed drafts and onto
the DB-backed user-draft layer introduced in
[PR #9351 "Db-backed user drafts"](https://github.com/windmill-labs/windmill/pull/9351).

> Status: the PR work is checked out locally (branch reset to the PR head + latest
> `main` merged in). This doc tracks the global-mode-specific follow-up that the PR
> itself does **not** complete.

---

## 1. How global-mode drafts work today

The global mode never touches `localStorage` directly. It persists drafts through the
shared `UserDraft` API:

```
core.ts tools
  → userDraftAdapter.ts   (getGlobalDraft / listGlobalDrafts / saveGlobalAppDraft / deleteGlobalDraft …)
    → UserDraft  (frontend/src/lib/userDraft.svelte.ts)
      → localStorage        ← on main
```

- `userDraftAdapter.ts` maps between the global mode's `WorkspaceItem` shape and the
  generic `UserDraft` entries (`UserDraftItemKind` = `script | flow | raw_app |
  trigger_* | resource | variable`).
- Reads go through `UserDraft.get()` / `UserDraft.list()`; writes through
  `UserDraft.save()` / `setDraftAndMeta()` / `remove()` / `clear()`.
- On `main`, `UserDraft` persists to `localStorage`, so `get`/`list` see every draft
  the user ever wrote in this browser, regardless of whether an editor is mounted.

## 2. What PR #9351 changes

**Backend** (`backend/windmill-api/src/drafts.rs`, `backend/windmill-common/src/user_drafts.rs`,
migration `20260528143710_draft_user_sync_schema`):

- Reshapes the existing `draft` table into **per-user** storage: adds an `email`
  column (NULL = legacy workspace-level draft), swaps the `DRAFT_TYPE` enum
  (`script/flow/app`) for `DRAFT_KIND` (every `UserDraftItemKind`), and replaces the
  PK with two partial unique indexes so per-user and legacy rows coexist.
- New endpoints:
  - `POST /w/{ws}/drafts/save_draft/{kind}/{path}` — upsert (or delete on `null`
    value) with optimistic concurrency (`last_sync` / `force`).
  - `GET  /w/{ws}/drafts/get_draft/{kind}/{path}` — fetch **my** draft value.
  - `GET  /w/{ws}/drafts/list_drafts` — **metadata-only** list of my drafts
    (`{ path, typ, saved_at }`), most-recent first. **No value, no summary.**
  - `GET  /w/{ws}/drafts/get/{kind}/{path}?username=…` — another user's draft (for the
    "other users' drafts" modal).

**Frontend** (`userDraft.svelte.ts` + `userDraftDbSyncer.svelte.ts`):

- Every `UserDraft` mutation now funnels into `UserDraftDbSyncer.save(...)` — a
  write-through layer with a debouncer + coalescing runner, optimistic concurrency,
  conflict surfacing, and a `keepalive` unload flush.
- **The localStorage *persistence* layer is removed.** `UserDraft.get()`/`list()` now
  only see **in-tab mounted handles** (the in-memory `entries` map). The comment on
  `UserDraft.list()` is explicit: *"for a workspace-wide view across sessions, call
  `DraftService.listDrafts` instead."*

**Global mode in the PR:** `userDraftAdapter.ts` is **untouched**; the only `core.ts`
changes drop the *old* server per-item draft reads (`getScriptByPathWithDraft` →
`getScriptByPath`, removing `remoteDraftRev`/`draft_created_at`).

## 3. The gap: why the global mode still needs work

- **Writes** ride along for free — `UserDraft.save(...)` already syncs to the DB.
- **Reads break.** The global mode is headless: it never mounts a live handle
  (`UserDraft.use`), it just does save-then-read. Under the PR:
  - `UserDraft.save()` for a path with **no mounted handle** pushes to the syncer but
    **does not populate `entries`** (`userDraft.svelte.ts` `save`).
  - `UserDraft.get()`/`list()` then return `undefined`/`[]`.
  - So `saveGlobalAppDraft` (adapter) and `getRequiredGlobalDraft` (`core.ts`) — which
    save then immediately read — would throw *"Could not read written draft"*, and
    `listGlobalDrafts` would miss anything from another tab/device.

There are ~15 read call-sites in `core.ts` going through
`getGlobalDraft` / `listGlobalDrafts` / `getGlobalDraftStoragePath`.

## 4. Migration strategy (shared principles)

The fix is to point the adapter's **read paths** at the DB while writes keep flowing
through the synced `UserDraft` layer:

1. **Reads → `DraftService`.** `getGlobalDraft` → `getDraft`; `listGlobalDrafts` →
   `listDrafts`. These become **async**, so `await` has to be threaded through the
   `core.ts` call-sites (most are already inside async tool handlers).
2. **Writes → awaited DB save when read back immediately.** For the save-then-read
   pattern, use an awaited DB write (`UserDraftDbSyncer.save({ …, immediate: true })`
   / `overwrite`, or `DraftService.saveDraft`) before reading. AI-driven overwrites
   pass `force: true` (no human at a conflict modal).
3. **Deletes** → `save_draft` with `value: null`. `clearGlobalDrafts` must enumerate
   via `listDrafts` (it can no longer rely on `UserDraft.list`).
4. **`getLiveEditorDraft` stays in-memory** — it's the open editor's path/rename
   mapping, unrelated to persistence.

### Gotchas

- **Secrets are safe.** Secret variable drafts already store `value: ''` and keep the
  real secret only in the in-memory `secretVariableDraftValues` map
  (`userDraftAdapter.ts` / `core.ts` `syncEphemeralSecretVariableDraftValue`). Keep
  that map **out** of the DB sync — DB-backing the draft value won't leak secrets.
- **Item-kind parity.** Every global kind (`raw_app`, `trigger_*`, `resource`,
  `variable`) already exists in the `DRAFT_KIND` DB enum and the generated
  `UserDraftItemKind`. Note `app` (low-code editor drafts) vs `raw_app` (global mode's
  app kind): `GLOBAL_DRAFT_KINDS` excludes plain `app`, so the global list must filter
  to `GLOBAL_DRAFT_KINDS`.
- **UI copy.** `core.ts` toasts say "Saving … to local storage" — reword to reflect DB
  persistence.
- **Tests.** `core.test.ts` mocks the storage layer; expect to swap mocks to
  `DraftService` per tool.
- After backend API changes: regenerate sqlx (`cargo sqlx prepare` / `update-sqlx`
  skill), update `openapi.yaml`, then `npm run generate-backend-client`.

## 5. Per-tool migration plan (checklist)

Work one tool at a time; each is independently shippable behind the existing
`wm_dev_global_ai` gate.

| # | Tool / helper | What changes |
|---|---------------|--------------|
| 1 | `list_workspace_items` | **✅ Done** — reuse list endpoints' `includeDraftOnly` + `isDraft` flag for script/flow/app; `listGlobalDrafts` removed (see §6) |
| 2 | `read_workspace_item` (`getGlobalDraft`) | read → `DraftService.getDraft`; async |
| 3 | `write_script` / `write_flow` / `write_app` | save-then-read → awaited DB save + DB read |
| 4 | `write` trigger / schedule | same as #3 for trigger kinds |
| 5 | `write_resource` / `write_variable` | same as #3; keep secret ephemeral map in-memory |
| 6 | `delete_draft` (`deleteGlobalDraft`) | `save_draft` null; drop `UserDraft.clear` reliance |
| 7 | `clearGlobalDrafts` | enumerate via `listDrafts` then delete each |
| 8 | deploy path | unaffected by storage; verify it reads the DB draft before deploying |

## 6. Tool #1 — `list_workspace_items` (implemented)

**Decision (revised):** *reuse the existing list endpoints* instead of a separate draft
fetch. Post #9351, `ScriptService.listScripts` / `FlowService.listFlows` /
`AppService.listApps` already accept `includeDraftOnly: true` and join the `draft` table
**scoped to the authed user** (`draft.email = authed.email`), returning two flags on each
row:

- `draft_only` — the item exists only as a draft (never deployed)
- `is_draft` — a deployed item that has a pending draft for this user

(draft-only rows also set `is_draft = true`). `listWorkspaceItems` *already* called the
script/flow endpoints with `includeDraftOnly: true` — it just hardcoded `isDraft: false`.
So the core fix is to read the flags: no new endpoint, no `DraftService.listDrafts`, no
`typ`→type mapper. Two follow-ups from review were also needed: the **app** call was
missing `includeDraftOnly` (issue #1), and listing under a **`path_prefix`** needed a
small backend change so draft-only rows are filtered by prefix instead of dropped (issue
#2 / option (c) — see below).

### What was implemented

- **`core.ts` `listWorkspaceItems`** — all three list calls pass `includeDraftOnly: true`
  (the app call previously omitted it — **issue #1 fix**), and `isDraft` is derived from
  the returned flags:
  - script/flow: `draft_only === true || is_draft === true` (both fields on the list row)
  - app: `is_draft === true` (`ListableApp` exposes only `is_draft`; draft-only apps set it)
- **`core.ts` list tool** — dropped the `listGlobalDrafts` merge. Script/flow/app drafts
  now arrive through `listWorkspaceItems`. Adds a small in-memory **live-editor merge**
  (`listLiveEditorDrafts`, below) so the open editor's unsaved/renamed draft still shows
  at its effective path with `isLiveDraft: true`, overriding the deployed entry (and
  dropping the stale pre-rename key). A `TODO(db-drafts)` comment records that
  schedule/trigger/resource/variable drafts are **not** discoverable here until their
  list endpoints gain `includeDraftOnly` (still reachable by path via
  `read_workspace_item`).
- **`userDraftAdapter.ts`** — removed `listGlobalDrafts` (and the short-lived
  `draftKindToWorkspaceItem` helper); added `listLiveEditorDrafts(workspace)` which reads
  the open editor's draft from the in-memory live registry (`getLiveEditorDraft`).
  Value-less; existence/path are taken from the registry and **not** gated on the in-tab
  value cell, so it's decoupled from the read-after-write gap (§3). `GLOBAL_DRAFT_KINDS`
  stays (still used by `clearGlobalDrafts`, tool #7).
- **`WorkspaceItemDrillPicker.svelte`** — removed the `aiDrafts`/`withAiDrafts`
  machinery; `loadKind` already uses `includeDraftOnly`, so draft-only script/flow/app
  items surface naturally.
- **`global_drafts/+page.svelte`** (dev inspector) — now calls `DraftService.listDrafts`
  directly (raw `{ path, typ, saved_at }`) and deletes via `DraftService.saveDraft`
  (`value: null`). Self-contained; no dependency on the adapter.
- **Backend — `scripts.rs` / `flows.rs` / `apps.rs` `list_*`** (**issue #2, option (c)**):
  the draft-only append no longer bails when `path_start` is set; instead the draft-only
  query filters by it (`AND ($N::text IS NULL OR path LIKE $N || '%')`, mirroring the
  deployed query's `and_where_like_left`). Other narrowing filters (`path_exact`,
  `created_by`, `label`, languages, pages past 0) still skip the append. sqlx offline cache
  regenerated for the three changed queries.

> **⚠️ Reviewer — please confirm (c):** removing the `path_start.is_none()` guard from the
> draft-only append is the chosen fix for issue #2. The guard was presumably there so
> pickers/selectors get a deployed-only listing; the global-chat list tool needs draft-only
> rows under a prefix, and honoring `path_start` in-query (rather than dropping draft-only)
> is the least-surprising behavior. If a caller relies on "prefix query ⇒ no synthesized
> draft-only rows", this changes that. Flagging for sign-off.

### Known limitation (intentional, awaiting backend)

`list_workspace_items` cannot surface schedule/trigger/resource/variable drafts — those
list endpoints have no per-user draft join yet. The list tool defaults to
`['script','flow']`, so the gap only bites when those kinds are explicitly requested.
Lift it by adding `includeDraftOnly` (email-scoped) to those endpoints.

### Verification (done)

- Backend confirmed live: a seeded draft-only script returns `draft_only: true,
  is_draft: true` from `listScripts?include_draft_only=true`, scoped to the authed email.
  After the (c) change, the same query **with** `path_start` set returns the draft-only row
  when it matches the prefix (and omits it otherwise) — verified via the API.
- Unit (`core.test.ts`): `flags backend draft scripts and forwards path_prefix + limit` and
  `requests draft-only apps and flags them via is_draft` (issue #1 guard) pass; the
  live-editor **list** assertions in `lists … the live script/flow editor draft` pass too.
  Those two live-editor tests then fail later in their *edit/write* half on the unrelated
  read-after-write gap (`getXByPath mock not configured` — see §3/§8). No regressions.
- E2E (not yet run): with `wm_dev_global_ai` enabled, create a draft script via the global
  mode, reload, confirm `list_workspace_items` flags it `isDraft: true`.

## 8. Handoff — current branch state & next steps

**Branch `claude-change-ed070a5a`** = PR #9351 (`remove-workspace-drafts`, head
`9c8c4edb`) + a merge of latest `origin/main`, plus the tool #1 commit. Environment
already prepared in this worktree:

- DB migration `20260528143710_draft_user_sync_schema` is **applied** to the dev DB
  (`windmill_claude_change_ed070a5a`). A fresh DB just needs `sqlx migrate run`.
- TS client was **regenerated** (`npm run generate-backend-client`) — `frontend/src/lib/gen`
  is gitignored, so a fresh checkout must regenerate it to get `DraftService.{saveDraft,
  getDraft, listDrafts, getDraftForUser}`.
- Dev server runs with `REMOTE=http://localhost:8070 PORT=3070`.

**Root cause of the ~28 failing `core.test.ts` tests (NOT tool #1):** the read-after-write
gap from §3. The PR's `UserDraft.save`/`setDraftAndMeta` only update the in-tab store when
a handle is *mounted*; the headless global tools never mount one, so `UserDraft.get` after
a write returns `undefined` and `getRequiredGlobalDraft` throws *"Could not read written
draft"*. In tests this also surfaces as `getXByPath mock not configured`. This blocks
tools #2–#8 and is the highest-leverage thing to fix next.

**Suggested next step:** either tool #2 (`read_workspace_item` → `DraftService.getDraft`),
or — more impactful — close the read-after-write gap directly (make a write readable
without a mounted handle: e.g. have `UserDraft.save` seed an entry, or route the global
write/read tools through `DraftService` directly), which unblocks the bulk of the failing
tests at once.
