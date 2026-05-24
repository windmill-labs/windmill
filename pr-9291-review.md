# PR #9291 Review — feat: plug global chat drafts into userdraft

## Code review

cc @centdix

Mergeable, but should ideally address nits: duplicate `ResourceDraftState`/`VariableDraftState` types in `core.ts` and `userDraftAdapter.ts`, unused exports `setGlobalDraft`/`getGlobalDraftSlot`/`GlobalDraftSlot` in the adapter, duplicate `DEFAULT_APP_DATA` literal in `userDraftAdapter.ts` when `DEFAULT_DATA` is already imported elsewhere, lost reactivity on the `/global_drafts` dev page after switching from `$state` store to UserDraft.

Found 4 issues:

1. **[P2]** `ResourceDraftState` and `VariableDraftState` are defined verbatim in two files (`userDraftAdapter.ts` lines 66-83 and `core.ts` lines 1713-1729). The two types must stay byte-for-byte identical for the round-trip (`createXToDraftState` in `core.ts` → `xItemToDraft` in `userDraftAdapter.ts`) to work; nothing in the type system enforces that today. Export the types from one place and import from `core.ts`.
   - `frontend/src/lib/components/copilot/chat/global/core.ts:1713`
   - `frontend/src/lib/components/copilot/chat/global/userDraftAdapter.ts:66`

2. **[P2]** `userDraftAdapter.ts` defines `setGlobalDraft` (line 460), `getGlobalDraftSlot` (line 413) and `GlobalDraftSlot` (line 85) as exports, but none are imported anywhere outside the file. Either delete the dead exports or inline `getGlobalDraftSlot` into `getGlobalDraft` / `listGlobalDrafts`.
   - `frontend/src/lib/components/copilot/chat/global/userDraftAdapter.ts:413`
   - `frontend/src/lib/components/copilot/chat/global/userDraftAdapter.ts:460`

3. **[P2]** `DEFAULT_APP_DATA = { tables: [], datatable: undefined, schema: undefined }` is hand-rolled in `userDraftAdapter.ts:64`, while `core.ts:42` already imports the canonical `DEFAULT_DATA` from `$lib/components/raw_apps/dataTableRefUtils`. The two will drift the next time the data shape grows a field. Import `DEFAULT_DATA` in the adapter instead.
   - `frontend/src/lib/components/copilot/chat/global/userDraftAdapter.ts:64`

4. **[P2]** `/global_drafts/+page.svelte` previously read from a `$state`-backed store, so writes from elsewhere updated the inspector reactively. After this PR, `let drafts = $derived(listGlobalDrafts($workspaceStore))` reads through `UserDraft.list()` which iterates a plain `Map` plus `localStorage` and explicitly `untrack`s the per-entry runes (`userDraft.svelte.ts:528`). The derived only refires when `$workspaceStore` changes, so drafts created post-mount won't appear until reload. Dev-only route so not blocking, but a visible regression — worth polling, watching `storage` events, or exposing a reactive list helper on `UserDraft`.
   - `frontend/src/routes/(root)/(logged)/global_drafts/+page.svelte:25`

### Test coverage

- **Frontend logic tests**: substantial coverage added to `core.test.ts` (+631 lines) exercising every `write_*` tool against the new UserDraft-backed adapter, live-editor-draft routing for scripts/flows/raw apps, and discard semantics. `userDraft.test.ts` adds tests for the new live-editor registry (workspace isolation, per-storage-path clear) and `snapshotDraftValue` behavior of `UserDraft.get`. Old `draftStore.test.ts` appropriately deleted. Coverage proportionate to what changed.
- **CI / backend**: no backend changes. The `cli/src/commands/sync/sync.ts` one-liner in the diff is the same `false → true` change already shipped in `486e5f947b` (#9289); will be a no-op when rebased on `main`.
- **Manual verification still worth running**: open AI chat, have it create a script and a flow draft, then open `/scripts/edit/<path>` and `/flows/edit/<path>` and confirm the editor restores the AI's draft. Then open a script editor first, ask chat to `edit_script` on the editor's *effective* path (not the URL path — e.g. on `/scripts/add` URL path is empty but editor has a typed path), and verify the change shows up in the open editor without creating a separate draft. Finally, run `discard_local_draft` on an item with an active live editor and confirm the editor's local autosave is cleared.

## Simplification / architecture opportunities

- **Collapse the six `writeXDraft` functions into one** — `writeScriptDraft` (core.ts:1856), `writeFlowDraft` (1920), `writeScheduleDraft` (1989), `writeTriggerDraft` (2019), `writeResourceDraft` (2050), `writeVariableDraft` (2083) all repeat the same skeleton: resolve storagePath, look up existingDraft, conditionally fetch backend existing, merge, call `UserDraft.save` or `setDraftAndMeta`, then `finishDraftWrite`. A per-kind strategy table `{ exists, fetch, merge, metaFromBackend }` plus one generic `writeDraft<K>(...)` would cut ~250 LoC and remove the three independently-evolving styles (NewScript-shaped, `mergeDraftConfig`-shaped, `xToDraftState`-shaped). Today schedules/triggers skip `setDraftAndMeta` (no remoteRev plumbed) while resources/variables do plumb `existing.edited_at` — no obvious reason.

- **Merge the two trigger-kind dictionaries into one source of truth** — `TRIGGER_DRAFT_KIND_BY_TRIGGER_KIND` (userDraftAdapter.ts:23) and `TRIGGER_KIND_BY_DRAFT_KIND` (userDraftAdapter.ts:33) are inverse maps maintained by hand. Keep only the forward map and derive the reverse at module load: `Object.fromEntries(Object.entries(FWD).map(([k, v]) => [v, k]))`.

- **Drop the duplicate `ResourceDraftState`/`VariableDraftState` types** (see finding 1). Once unified, `createResourceToDraftState` / `resourceToDraftState` in `core.ts` and `resourceItemToDraft` / `resourceDraftToWorkspaceItem` in `userDraftAdapter.ts` do the same `(payload ↔ draft-state)` round-trip in opposite directions; consider keeping the conversion pair only in `userDraftAdapter.ts` and having `core.ts` use them.

- **Inline `saveAppDraft` (core.ts:836)** — it's a one-line wrapper around `saveGlobalAppDraft`. Call `saveGlobalAppDraft` directly at the 6 call sites, or rename the adapter export and delete the wrapper.

- **Inline `triggerKindToUserDraftKind` (userDraftAdapter.ts:128)** — one-line wrapper over `TRIGGER_DRAFT_KIND_BY_TRIGGER_KIND[kind]`, used at one site (`writeTriggerDraft`, core.ts:2026). Export the map or inline the lookup.

- **`appDraftMeta` (core.ts:873) is called once** in `loadAppDraftValue` (line 902). Inlining saves the named function and the `UserDraftMeta` import.

- **`getGlobalDraftStoragePath` is called only by write helpers** in core.ts. With the per-kind unification above, `resolveDraftStoragePath` could move into the generic `writeDraft` and the wrapper goes away.

- **`UserDraft.clear` is `UserDraft.discard(...)` aliased** (userDraft.svelte.ts:494). Pre-existing, but adapter consistently calls `UserDraft.clear` — pick one direction in a follow-up.

- **`discard_local_draft`'s schema is structurally a subset of `delete_workspace_item`'s schema** (both take `{ type, path, trigger_kind? }`). Consider whether `delete_workspace_item` could take a `scope: 'workspace' | 'local_draft'` discriminator and remove the second tool — fewer tools for the model, confirmation flow already exists for both.
