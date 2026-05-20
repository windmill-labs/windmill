# Global Mode AI Chat and UserDraft Requirements

## Purpose

Global mode AI chat needs to inspect and modify workspace items without
immediately deploying them. For frontend-only draft state, it should use the
same `UserDraft` service as editors, but with a stricter contract:

- editor state must remain safe from accidental deletion or deployment;
- AI-written drafts must be discoverable and deployable;
- live editor handles and persisted localStorage entries must stay consistent;
- backend draft-only items must not be confused with frontend `UserDraft`
  entries.

This document defines the requirements for that contract. It is intended to
guide a small UserDraft-only PR before wiring global mode AI chat to those
methods.

## Terms

### UserDraft

The frontend local draft service backed by localStorage and optional live
Svelte handles. It is scoped by `(workspace, itemKind, storagePath)`.

### Storage path

The path used in the localStorage key and live-handle map. For existing items
this is normally the item path. For new-item editors, this can be an empty
string while the draft value already contains its final workspace path.

### Item path

The workspace path inside the draft value, for example
`f/scripts/my_script`. Global mode must operate by item path.

### Entry source

Where the current `UserDraft` value came from:

- `persisted`: localStorage only;
- `live`: live editor handle only;
- `both`: live handle plus localStorage entry.

This is not the same as draft ownership.

### Draft origin

Who wrote the value:

- editor-originated draft: editor autosave or editor baseline hydration;
- external-originated draft: global mode AI chat, or any future external actor
  that writes into UserDraft outside the editor's normal mutation flow.

Only external-originated drafts are deployable by global AI mode.

## Requirements

### R1. Preserve the existing editor contract

Existing editor usage of `UserDraft.save`, `UserDraft.saveIfChanged`,
`UserDraft.use`, `UserDraft.useMany`, `setDraftAndMeta`, and `setMeta` must keep
working without global mode installed.

Opening an editor and hydrating its current backend state must not create a
deployable AI draft. A clean open editor baseline is useful read context, but it
is not unsaved AI work.

### R2. External writes must be explicit

Global mode must not call the normal editor-oriented `UserDraft.save` when it
creates or updates an AI draft. It needs an explicit method such as:

```ts
UserDraft.saveExternal(itemKind, storagePath, value, opts)
```

That method must:

- mark metadata so the entry can later be recognized as external-originated;
- preserve existing freshness metadata such as `remoteRev` and
  `remoteDraftRev`;
- update any live handle for the same key immediately;
- persist to localStorage immediately, even if a live handle exists.

Immediate persistence matters because global mode listing, deployment, and page
refresh recovery must see the AI-written draft without waiting for an editor
autosave effect.

### R3. Normal editor saves must remain neutral

`UserDraft.save` and `UserDraft.saveIfChanged` must not mark drafts as external.
Absence of external-origin metadata must be treated as editor-originated or
legacy state.

This keeps existing localStorage entries backward-compatible and prevents old
editor drafts from becoming deployable by global mode.

### R4. Listing must include enough metadata for safe filtering

Global mode needs a generic listing API, for example:

```ts
UserDraft.list({ workspace, itemKinds })
```

Each entry must include:

- `workspace`;
- `itemKind`;
- `storagePath`;
- best-effort `path` for the storage key;
- `source`: `persisted`, `live`, or `both`;
- `draftOrigin` or equivalent metadata;
- cloned `value`.

The listing API must merge persisted and live entries for the same key. It must
not expose the same draft twice.

### R5. Listing must tolerate live editor runtime values

Some live editor values contain functions, class instances, or other
non-structured-cloneable fields. `UserDraft.list` must still return the usable
draft fields instead of crashing.

A safe implementation can try `$state.snapshot` and `structuredClone`, then fall
back to a shallow/object-preserving clone when a runtime-only field cannot be
cloned.

### R6. Storage path and item path must stay separate

Global mode works by workspace item path, but UserDraft storage is keyed by
storage path. UserDraft must expose storage path in list entries so callers can
later delete or overwrite the exact stored draft.

This is required for new-item editors where:

- storage path can be empty;
- draft value path can be `u/admin/new_script`;
- global mode must find the item by value path;
- delete/clear must target the original storage path.

### R7. External drafts must be deployable only when persisted

Global mode should consider an entry deployable only if:

- it is external-originated; and
- it is persisted (`source` is `persisted` or `both`).

A live-only value can be read as current context, but deploying a live-only value
is risky because it may be an editor baseline or a transient state that would be
lost on refresh.

### R8. Reads must distinguish current context from deployable drafts

Global mode needs two read concepts:

- current frontend-visible item: includes clean live editor baselines and
  editor drafts, and is useful as context;
- deployable AI draft: external-originated and persisted.

The UserDraft API should provide enough metadata for a caller to implement both
views without guessing from the item value alone.

### R9. Clear must reset persisted and live state

Global mode delete/clear needs a method such as:

```ts
UserDraft.clear(itemKind, storagePath, opts)
```

The method must:

- remove the localStorage entry;
- reset any live handle for that key immediately;
- avoid re-persisting the cleared value via the live handle's autosave effect.

The existing `remove` method is not enough for global mode because it clears
localStorage only and can leave stale live state visible.

### R10. Discard with fallback must remain available for editors

Editors still need the ability to discard local changes and restore a deployed
baseline in memory:

```ts
UserDraft.discard(itemKind, storagePath, fallback, opts)
```

`clear` should be a convenience wrapper for `discard(..., undefined)`, not a
replacement for editor discard behavior.

### R11. Metadata updates must preserve ownership metadata

`saveMeta` and handle-level `setMeta` must merge metadata instead of replacing
the entire metadata object. Updating `remoteRev` or `remoteDraftRev` must not
drop the external-origin marker.

Similarly, `saveExternal` must not drop existing remote freshness metadata.

### R12. UserDraft must stay domain-generic

UserDraft should not know about `WorkspaceItem`, scripts, flows, triggers, apps,
or global AI chat tools. It should expose generic draft entries and metadata.

Mapping `UserDraftItemKind` to global mode workspace item types should live in a
global-mode adapter layer.

### R13. Backend DB drafts are outside UserDraft

Backend draft-only scripts, flows, and apps are not frontend UserDraft entries.
Global mode read/list tools must use backend draft-aware endpoints for those.

UserDraft should only represent local frontend drafts. The global-mode adapter
is responsible for merging:

- backend deployed items;
- backend DB draft-only items;
- local UserDraft current context;
- local UserDraft external drafts.

### R14. Tests must cover live/persisted edge cases

The UserDraft-only PR should include unit tests for:

- `saveExternal` marks origin and persists without a live handle;
- `saveExternal` updates and persists when a live handle exists;
- normal `save` does not mark origin;
- `list` reports `storagePath`, `source`, origin, and cloned value;
- `list` handles persisted-only, live-only, and both states;
- `clear` removes localStorage and clears live state;
- `discard` still restores a fallback baseline;
- metadata updates preserve origin;
- uncloneable live values do not make `list` throw;
- empty storage path plus value-level item path remains representable.

Global mode integration tests should live in the later global-mode PR, not in
the UserDraft-only PR.

## Proposed UserDraft-only PR Scope

The first PR should be limited to UserDraft service primitives and tests.

### In scope

- Add explicit external-write API.
- Add origin metadata to stored draft metadata.
- Enrich `UserDraft.list` entries with `storagePath`, `source`, and origin.
- Make `UserDraft.list` merge persisted and live entries safely.
- Add `UserDraft.clear`.
- Ensure metadata merge behavior preserves origin.
- Add focused `frontend/src/lib/userDraft.test.ts` coverage.

### Out of scope

- No global chat tool wiring.
- No changes to global AI prompts.
- No global draft inspector route.
- No deploy behavior changes.
- No script/flow/app-specific adapter logic.
- No backend API changes.

## Later Global Mode PR Scope

The follow-up global-mode PR can then:

- introduce a thin adapter that maps UserDraft entries to global
  `WorkspaceItem`s;
- use external writes for global mode `write_*` tools;
- list external persisted drafts as deployable;
- read local current context without marking clean editor baselines as drafts;
- clear AI drafts through `UserDraft.clear`;
- merge backend items, backend draft-only items, editor context, and AI drafts
  deterministically;
- add global chat unit and browser tests for read/list/write/deploy flows.

## Acceptance Criteria

The UserDraft-only PR is acceptable when:

- existing editor behavior is unchanged;
- external-origin drafts can be identified without inspecting item shape;
- deployable vs current-context decisions can be made from list metadata;
- live handles and localStorage cannot diverge after external writes or clears;
- no global mode code needs private access to UserDraft internals.
