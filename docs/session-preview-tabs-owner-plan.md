# Plan — `SessionPreviewTabs` deep module (sessions preview-tab owner)

Status: **implemented.** The design below is the as-built architecture of
`frontend/src/lib/components/sessions/sessionPreviewTabs.svelte.ts` (owner class + adapter,
unit-tested in `sessionPreviewTabs.test.ts`); the "Why" section describes the pre-refactor
state it replaced.

## Why

A session's preview tabs were represented in **three** places kept aligned by a fragile
protocol:

- page-local `$state` in `routes/(root)/(logged)/sessions/+page.svelte` (`tabs`, `activeTabId`,
  `previewCollapsed`, `mountedTabIds`) — the live copy while mounted;
- the persisted session record (`session.previewTabs` / `activePreviewTabId` / `previewCollapsed`)
  in `sessionState.svelte.ts` — durable;
- the legacy `previewUrls` localStorage map in `sessionMode.svelte.ts` — a per-session single-URL seed.

They were synced by a **seed `$effect`** (persisted → local on session change), a **reconcile
`$effect`** (persisted → local mid-session, for the `open_preview` tool), and a **debounced
write-behind** (local → persisted). The invariants were enforced only by comments ("additive only…
or it fights the write-behind"). Mutation forked into a local path (UI picker, no dedup) and a
persisted path (`open_preview`, dedup); `session.target` and `tab.url` were written non-atomically,
so the live-editor identity was derived from two drifting fields.

**Goal:** one deep module owns a session's tabs behind a small interface that both the sessions
page (renderer) and the `open_preview` tool cross. Single live copy → both sync effects disappear.

## Agreed design (7 decisions)

1. **Home.** `SessionPreviewTabs` is a pure runes class held as a field on `SessionRuntime`
   (`runtime.previewTabs`), mirroring the existing `pipelineEditorState` precedent. The
   module-global `runtimes` map already gives it per-session lifecycle that outlives the page and
   is reachable by the tool (`runtimes.get(sessionId)`). The session record stays the **durable
   backing**; the owner is the single **live** copy.

2. **DOM boundary.** `mountedTabIds` (the lazy-mount render gate) stays **page-local** — it's pure
   "has this iframe been created yet" bookkeeping, duplicates no tab identity, and keeps DOM
   lifecycle out of the owner's interface. The owner holds `tabs` (incl. `loc`), `activeId`,
   `collapsed`. The page feeds observed iframe location back via `observeLocation(id, loc)` (only
   the page can read `contentWindow.location`).

3. **Editor target.** `session.target` stays a **session-record field** (read by `PreviewTabHost`'s
   `resolvePreviewTab`, `SessionWrapper`'s inline editor incl. `pipeline`, and the runtime load
   slots). The three tab-coupled writers (`open_preview`, `navigateEditorTo`,
   `promoteActiveTabToEditor`) funnel through the owner, which sets `target` **and** adds/focuses the
   tab in one operation — killing the non-atomic url/target drift. Non-tab target writes
   (`sessionSwitch` on open, `pickEditorTarget` standalone) stay direct; they don't open tabs.

4. **Persistence / coupling.** The owner is a **pure runes class** with **zero** `sessionState`/IDB
   imports (like `pipelineEditorState`). Constructor takes a hydration snapshot + an injected
   adapter `{ persist(snapshot), setTarget(target) }`. Owner holds its **own** `$state` (decoupled
   from the churny `sessionState.sessions` array — so `loc` changes no longer re-emit it); debounce
   lives **inside** the owner. Two adapters justify the seam: prod (`setSessionTabs` +
   `setSessionPreviewCollapsed` + `setSessionTarget` + `putSession`) and an in-memory test spy.

5. **`previewUrls` eliminated.** Create-from-page (`SessionPicker`) seeds the initial pinned tab
   straight into `session.previewTabs` (in-memory; persists when the session materializes on first
   send). Owner hydration has one source: `previewTabs ?? (target → editor tab) ?? []`. Deletes
   `previewUrls`, `captureSessionView`, `sessionPreviewSeedUrl`, `sessionPreviewUrl`, and the
   `wm_session_preview_urls` localStorage key. Edge accepted: a never-sent transient session loses
   its seed on reload, but the transient session itself is already discarded on reload — they vanish
   together.

6. **Interface (depth-defining).** Two destination ops, uniform target-setting:
   - `open(target)` — new-or-focus a tab; sets `session.target` via the adapter iff `target` is an
     editable item (page targets don't).
   - `navigate(target)` — retarget the **active** tab; same target rule.
   - `promoteActiveTabToEditor` **collapses into** `navigate()` (3 ops → 2; atomic target+url write
     becomes uniform).
   - plus `select(id)`, `close(id)`, `setCollapsed(bool)`, `observeLocation(id, loc)`.
   - reactive getters: `tabs`, `activeId`, `activeTab`, `collapsed`.
   - `get_preview_status` uses a **pure** `describePreview(tabs, activeId, target)` reading owner
     getters + `session.target`, so the owner needs no target-read dependency.
   - `resolvePreviewTab(url, target)` **stays** in `previewRouter.ts` for `PreviewTabHost`.

7. **Handlers & migration.** `open_preview`/`get_preview_status` handlers resolve
   `session → getOrCreateRuntime → runtime.previewTabs.open(item)` / `describePreview(...)`.
   `setSessionTabs` / `setSessionPreviewCollapsed` / `setSessionTarget` **stay** as low-level record
   writers and become the prod persistence adapter. `openSessionPreviewTab` + `describeSessionPreview`
   are **deleted** from `sessionState` (logic absorbed into the owner + pure `describePreview`). The
   page's reconcile `$effect` is **replaced** by a one-line render effect:
   `$effect(() => mountedTabIds.add(owner.activeId))`.

## Module sketch

```ts
// sessions/sessionPreviewTabs.svelte.ts
export type PreviewTabsAdapter = {
  persist: (snapshot: { tabs: SessionPreviewTab[]; activeId: string; collapsed: boolean }) => void
  setTarget: (target: SessionTarget) => void
}

export class SessionPreviewTabs {
  #tabs = $state<SessionPreviewTab[]>([])
  #activeId = $state('')
  #collapsed = $state(false)

  constructor(
    initial: { tabs: SessionPreviewTab[]; activeId: string; collapsed: boolean },
    private adapter: PreviewTabsAdapter
  ) { /* hydrate */ }

  get tabs() { return this.#tabs }
  get activeId() { return this.#activeId }
  get activeTab() { return this.#tabs.find((t) => t.id === this.#activeId) ?? this.#tabs[0] }
  get collapsed() { return this.#collapsed }

  open(target: PreviewTarget): { status: 'opened' | 'focused' } { /* dedup; setTarget if item; flush */ }
  navigate(target: PreviewTarget): void { /* retarget active; setTarget if item; flush */ }
  select(id: string): void {}
  close(id: string): void {}
  setCollapsed(collapsed: boolean): void {}
  observeLocation(id: string, loc: string): void {}

  #flush = /* debounce */ () => this.adapter.persist({ tabs: this.#tabs, activeId: this.#activeId, collapsed: this.#collapsed })
}
```

Hydration helper (owner or `createRuntime`): `previewTabs ?? (session.target → single editor tab) ?? []`.

## Implementation stages (validate between each)

a. **`SessionPreviewTabs` class + pure tests.** New `sessions/sessionPreviewTabs.svelte.ts` and
   `sessionPreviewTabs.test.ts`. Cover open (new/focus/dedup), navigate, select, close, setCollapsed,
   observeLocation, seed/hydration, and the atomic `setTarget` call — all via injected spies, no
   globals, no IDB. Add a `describePreview` pure fn + test.

b. **Wire onto the runtime.** Add `readonly previewTabs: SessionPreviewTabs` to `SessionRuntime`;
   construct it in `createRuntime` with the hydration snapshot and the prod adapter
   (`setSessionTabs` + `setSessionPreviewCollapsed` + `setSessionTarget` + `putSession`).

c. **Page becomes renderer.** `+page.svelte` reads `activeRuntime.previewTabs` getters; tab-strip
   handlers call owner methods; `onTabLoad` calls `observeLocation`. **Delete** the seed `$effect`,
   the reconcile `$effect`, page-local `tabs`/`activeTabId`/`previewCollapsed` `$state`, and
   `persistTabs`. Replace reconcile with `$effect(() => mountedTabIds.add(owner.activeId))`.

d. **Handlers → owner.** `sessionRuntime` `setOpenPreviewHandler`/`setGetPreviewStatusHandler`
   dispatch to `runtime.previewTabs`. **Delete** `openSessionPreviewTab` + `describeSessionPreview`
   from `sessionState`; move their tests to the owner's test file.

e. **Delete legacy seed.** Seed `previewTabs` at create-from-page in `SessionPicker`; **delete**
   `previewUrls`, `captureSessionView`, `sessionPreviewSeedUrl`, `sessionPreviewUrl` from
   `sessionMode.svelte.ts` and the localStorage key. Replace the `previewUrl` fallback uses with
   `${base}/`.

## Validation

- `npm run check:fast` (no new errors beyond the pre-existing `Timeout`-vs-`number` baseline).
- `npx vitest run` for `sessionPreviewTabs.test.ts`, `previewRouter.test.ts`, `sessionState.test.ts`,
  and the global `core.test.ts`.
- Playwright (dev server): fresh session → `open_preview` reveals the collapsed panel and opens the
  tab; UI picker still opens exactly one picker and one tab; switch between sessions preserves each
  one's tabs.

## Files touched

- **New:** `frontend/src/lib/components/sessions/sessionPreviewTabs.svelte.ts` (+ `.test.ts`).
- **Edit:** `sessionRuntime.svelte.ts` (field + adapter + handlers), `sessionState.svelte.ts` (delete
  `openSessionPreviewTab`/`describeSessionPreview`; keep low-level writers), `sessionMode.svelte.ts`
  (delete `previewUrls` + helpers), `routes/(root)/(logged)/sessions/+page.svelte` (renderer; delete
  effects), `SessionPicker.svelte` (seed `previewTabs` at create-from-page).
- **Unchanged:** `previewRouter.ts` (`resolvePreviewTab` stays), `PreviewTabHost.svelte`.

## Out of scope (separate candidates from the architecture review)

- One registry owning the runtime warm/teardown lifecycle (leaked runtimes; `runtimes` uncapped vs
  `editorWarmIds` cap-3 divergence).
- Collapsing the five `set*Handler` singletons into one session-tool gateway seam.
