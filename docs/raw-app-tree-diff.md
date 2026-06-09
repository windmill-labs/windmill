# Plan: Raw-app diff as a file tree

## ⚠️ REVISED APPROACH (supersedes the original below)

The first cut stacked all per-file diffs inside ONE content box under the single
`raw_app` row (`RawAppDiffStack`). That conflated separate files into one box. The
revised model treats **each file (and `app.yaml`, each runnable) as its own synthetic
diff item** — an independent, collapsible row like any other workspace item.

**Runnables render as script/flow rows.** A runnable *is* a script (or flow) — usually
an inline script (`inlineScript.content` + `language`). So `rawAppDiffToItems` emits
runnables as `RawAppRunnableItem` (kind `script`/`flow`, `runType:'flow'` → flow icon),
carrying the reshaped runnable (code/language hoisted to the top level) so the **normal
script-style diff** renders them — Content (syntax-highlighted code) + Metadata tabs —
instead of a YAML blob. The body is always rendered with `kind="script"` (so a flow
runnable doesn't hit FlowDiffViewer with non-OpenFlow data); only the row icon/label
reflects `script` vs `flow`. Files + the `app.yaml` metadata leaf remain
`RawAppFileItem`s.

**Synthetic items + composite paths.** Replace each `raw_app` entry in the diff list
with N synthetic items via a reusable `rawAppDiffToItems(appPath, parentRaw, forkRaw)`
helper. Each carries a **composite path** `appPath/filePath` (e.g.
`u/admin/classy_app/App.tsx`, `…/app.yaml`, `…/runnables/foo`) and is
`WorkspaceItemDiff`-shaped (`kind: 'raw_app_file'`; `exists_in_source/fork` derived
from status so `statusOf`/badges work unchanged; `ahead/behind: 0` → hidden) plus
carried `{ original, current, lang, isMetadata, appPath, fullYamlOriginal,
fullYamlCurrent }`. Because the path is composite, the **existing** tree builder,
search, count and sidebar nesting treat them as ordinary file leaves (the app becomes
a folder) — so the Phase-3 rawfile special-casing is **deleted**
(`rawFilesByKey`, rawfile `NavEntry`, `scrollToRawFile`, child-row rendering,
`rawFileAnchorPrefix`, `rawFileStatusDotClass`).

**Rendering.**
- `WorkspaceItemDiffViewer` gains a `raw_app_file` branch: one `DiffEditor`, open by
  default, with a **per-file size guard** ("Load diff" card past the line budget). The
  old `raw_app` Tree/YAML branch and `RawAppDiffStack.svelte` are **deleted**.
- The `app.yaml` item shows the **metadata** diff by default + an **Expand** toggle
  that swaps in the full whole-app YAML (escape hatch preserved, lazy).
- `RowIcon` gains a `raw_app_file` case (file icon).
- Per-app guard is dropped (no app box). File rows link to the app editor.

**ForkDiffDrawer.** A derived `displayDiffs` expands `comparison.diffs`, replacing each
loaded `raw_app` with its synthetic items; an unloaded raw app stays a single
placeholder row ("Loading diff…") until its value arrives. `displayDiffs` feeds the
list, tree, search and the recomputed "N items" count.

**Kept from the original:** the parser `parseRawAppDiff` + `normalizeRawApp`
(value-wrapper fix), `extToLang` extensions, and the parser unit tests. `rawAppDiffToItems`
is built on top of `parseRawAppDiff`.

---

## Context

Everywhere Windmill shows a diff for a **raw app**, it renders one big YAML diff of
the entire serialized app (`files` + `runnables` + `data` + metadata) in a single
Monaco editor. A raw app is conceptually a *folder of files*, not a single document,
so a flat YAML diff is hard to scan — you can't tell at a glance which files changed,
and an edit to one file is buried in a wall of serialized YAML.

The goal: a **parser** that turns a pair of raw-app objects into a per-file diff, so
the UI can present it as a navigable file tree. Start with the session diff bar
(`ForkDiffDrawer`), but the parser must be reusable so every raw-app diff surface
benefits. Outcome: a VS-Code-"Changes"-style view — changed files nested in the
existing sidebar tree, each rendered as its own diff block, with a GitHub-style guard
for very large diffs.

## Decisions (locked)

- **Tree contents:** synthesize virtual paths so the tree covers the *whole* diff —
  real `files` at natural paths, each runnable as `runnables/<name>`, and all
  remaining metadata (`summary`/`data`/`policy`/`custom_path`) collapsed into one
  `app.yaml` leaf.
- **Only modified files** appear (unchanged omitted).
- **Reusable seam = the parser** (a pure function). Rendering reuses the existing
  `ForkDiffDrawer` sidebar by nesting raw-app files under the raw_app leaf.
- **Parser returns a flat list**; consumers build the tree with the existing
  `buildFileTree` util.
- **Main content = vertical stack of per-file Monaco diffs**; sidebar nests the same
  files for navigation. Both driven by one parse.
- **Loading:** already eager — `ForkDiffDrawer.fetchComparison` loops `loadDiffFor`
  over every diff on open, so raw-app values are already fetched up front. The guard
  only gates Monaco mounts, not the network.
- **Guard:** metric = total changed lines (added+removed), GitHub-style.
  Scope = **per-app** (one "Load full diff" card for the whole stack) **and**
  **per-file** (an oversized single file gets its own collapsed card).
- **Integration scope:** switch `WorkspaceItemDiffViewer`'s raw_app branch to the
  tree-stack. NOTE (corrected during impl): `WorkspaceItemDiffViewer` is consumed
  **only** by `ForkDiffDrawer` (the session diff bar). `DiffDrawer` — used by the
  full-page `CompareWorkspaces` fork compare — has its *own* raw_app rendering and
  receives **pre-serialized** `data` (content/metadata/lang), not the raw app
  object, so it cannot reuse the parser without the caller passing raw objects +
  a new DiffDrawer branch. That surface is a follow-up (see below); this change
  covers the session diff bar, the agreed starting point.
- **YAML fallback:** keep a `Tree | YAML` toggle; Tree is default, YAML flips to the
  old whole-app diff as an escape hatch.
- **Namespacing:** real files keep natural paths at root; `runnables/` folder;
  `app.yaml` at root. Parser disambiguates the rare real-file collision.

## Key existing code to reuse

- `frontend/src/lib/components/raw_apps/fileTreeUtils.ts` — `buildFileTree(paths)` →
  `TreeNode[]` (folders-first, alphabetical).
- `frontend/src/lib/editorLangUtils.ts` — `extToLang(ext)` (missing `html`/`md`; will
  extend).
- `frontend/src/lib/utils.ts` — `orderedYamlStringify`, `cleanValueProperties`,
  `replaceFalseWithUndefined`.
- `frontend/src/lib/components/DiffEditor.svelte` — Monaco diff (lazy-imported,
  content-sized; `inlineDiff` → unified).
- `frontend/src/lib/components/WorkspaceItemDiffViewer.svelte` — shared per-item
  renderer; raw_app currently falls into the generic single-YAML branch.
- `frontend/src/lib/components/sessions/ForkDiffDrawer.svelte` — sidebar tree
  (`buildTree`, `renderTreeNode`, keyboard nav `flattenVisible`/`firstChildKey`/
  `parentFolderKeyFor`), main-column `<details>` rows, `rowId`/`scrollToDiff`,
  status icons (`Plus`/`Minus`/`Pencil`) + colors.
- `frontend/src/lib/components/sessions/appDraftCodec.ts` — raw-app shape reference
  (`RawAppDraft` / `RuntimeRawApp`).

## Implementation

### Phase 1 — Parser core (pure + tested)
**New `frontend/src/lib/components/raw_apps/rawAppDiffUtils.ts`:**
- Types:
  ```ts
  type RawAppDiffStatus = 'added' | 'removed' | 'modified'
  type RawAppDiffEntry = { path: string; status: RawAppDiffStatus; original?: string; current?: string; lang: string }
  ```
- `parseRawAppDiff(original?, current?): RawAppDiffEntry[]`
  - Normalize each side to `{ files, runnables, summary, data, policy, custom_path }`;
    missing side → empty maps (whole-app add/remove falls out naturally).
  - **files:** key union; compare string content; `lang = extToLang(ext)`; emit changed only.
  - **runnables:** key union → `runnables/<name>`; `orderedYamlStringify` each; `lang: 'yaml'`.
  - **metadata:** `{ summary, data, policy, custom_path }` per side → `app.yaml`;
    `orderedYamlStringify`; emit if changed; `lang: 'yaml'`.
  - **collision:** if a real file is literally `app.yaml` or under `runnables/`, suffix
    the synthesized path.
- `rawAppDiffStats(entries)` → `{ added, removed, modified, changedLines }`
  (changedLines = added+removed lines per entry; simple line count of the two sides).

**Extend `extToLang()` in `editorLangUtils.ts`** with `html`, `md`/`markdown`, `xml`,
`svg`, `txt`, `scss`, `less` (raw apps are web files; these currently return `'unknown'`).

**Tests `frontend/src/lib/components/raw_apps/rawAppDiffUtils.test.ts` (vitest):**
file add/remove/modify, runnable add/remove/modify, metadata-only change, whole-app
add & remove, collision case, changedLines counting, unchanged-omitted.

### Phase 2 — Reusable stack renderer + WorkspaceItemDiffViewer
**New `frontend/src/lib/components/raw_apps/RawAppDiffStack.svelte`** (presentational,
reusable):
- Props: `entries: RawAppDiffEntry[]`, `inlineDiff?`, `lineBudget = 1500`, `anchorPrefix?`.
- **Per-app guard:** `stats.changedLines > lineBudget` → one "Load full diff (N changed
  lines)" card; click reveals stack.
- **Per block:** header (path + add/remove/modify badge reusing the existing icon+color
  convention) and a lazily-imported `DiffEditor`, content-sized. **Per-file guard:** entry
  over budget → its own "Load diff" card. Each block gets `id={anchorPrefix + path}`.

**`WorkspaceItemDiffViewer.svelte`:** add an explicit `kind === 'raw_app'` branch *before*
the generic `hasContent` path:
- Parse via `parseRawAppDiff(originalRaw, currentRaw)`.
- Small `Tree | YAML` toggle (ToggleButtonGroup/Tabs). **Tree** (default) →
  `RawAppDiffStack`; **YAML** → the existing single whole-app `DiffEditor` (unchanged).
- Reaches the session diff bar (ForkDiffDrawer). It does **not** reach DiffDrawer /
  CompareWorkspaces — those are a follow-up (see below).

### Phase 3 — Sidebar nesting in ForkDiffDrawer (most involved / riskiest)
- Extend tree types so a `raw_app` node carries
  `rawFiles: { name; path; status; anchorId }[]` children, derived reactively from
  `loadedDiffs[key]` once parsed.
- Render nested children in `renderTreeNode` (reuse the status-dot snippet); clicking a
  child scrolls to its block inside the app's main-column `<details>` via `anchorPrefix`.
- Extend keyboard-nav helpers (`flattenVisible`, `firstChildKey`, `parentFolderKeyFor`,
  `navEntries`) to include raw-file leaves.
- Pass `anchorPrefix = rowId(d) + '-rf-'` to `WorkspaceItemDiffViewer` → `RawAppDiffStack`.
- Sidebar sort order: `app.yaml` first → files → `runnables/` (folders-first otherwise).

Phases 1–2 already deliver the tree-stack everywhere and can ship without Phase 3 if it
needs more iteration.

## Verification

- `npm run check:fast` while iterating; `npm run check` before done.
- Run the parser vitest suite (`rawAppDiffUtils.test.ts`).
- **Playwright (backend on :8210 / frontend on :3210, EE):** in a session with a forked
  raw app, modify two files + one runnable + the summary, open the session diff bar →
  confirm: files nested under the raw_app in the sidebar, per-file diff blocks in the
  main column, sidebar click scrolls to the right block, `Tree/YAML` toggle works.
  Force a large file (paste a big/minified blob) to confirm both the per-file and
  per-app "Load (full) diff" guard cards appear and load on click.

## Follow-up (out of scope here)
- **DiffDrawer / CompareWorkspaces:** to bring the tree-stack to the full-page fork
  compare and deploy diffs, `CompareWorkspaces` must pass the raw app objects (not
  pre-serialized content/metadata) and `DiffDrawer` needs a `raw_app` branch that
  calls `parseRawAppDiff` + renders `RawAppDiffStack`. The reusable parser + stack
  built here make this a contained addition.

## Risks / notes
- Phase 3 (sidebar nesting + keyboard-nav extension) is the most delicate part of the
  existing `ForkDiffDrawer`; keep its changes isolated.
- `lineBudget` (~1500) and exact sidebar sort are final-tuned during implementation.
