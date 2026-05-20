# Global Mode AI Chat and UserDraft Requirements

## Purpose

This document enumerates the hard requirements that global mode AI chat places
on the frontend `UserDraft` service. It is intentionally limited to required
behavior and does not prescribe API names, data structures, or implementation
strategy.

The requirements are based on the current base branch state of `UserDraft`:

- drafts are scoped by workspace, item kind, and storage path;
- persisted drafts are stored in localStorage under `userdraft/w/...`;
- live editor handles may hold draft state that is not present in
  localStorage;
- draft values are wrapped with freshness metadata used by editor staleness
  checks;
- existing editor flows rely on `UserDraft` for autosave, restore, metadata,
  removal, discard, and live-handle behavior;
- the service does not provide a public way to enumerate all draft entries;
- the service does not identify whether a draft was written by an editor or by
  an external actor.

## Requirements

### R1. Existing Editor Behavior Must Be Preserved

All existing editor behavior that depends on `UserDraft` must remain unchanged.

Opening an editor, hydrating current backend state, autosaving local edits,
restoring local edits, checking staleness, removing local drafts, and discarding
to a fallback value must keep the same observable behavior.

Global mode must not cause a clean editor baseline to become deployable AI work.

### R2. Global Mode Writes Must Be Distinguishable From Editor Writes

When global mode creates or updates a frontend local draft, later code must be
able to distinguish that draft from drafts created by normal editor autosave or
editor hydration.

This distinction must survive page reloads.

This distinction must survive freshness metadata updates.

Legacy drafts with no such distinction must not be treated as global mode AI
drafts.

### R3. External Draft Writes Must Be Visible Immediately

After global mode writes a frontend local draft, the written value must be
visible immediately to:

- any live editor handle for the same workspace, item kind, and storage path;
- any localStorage-based lookup for that draft;
- any global mode read, list, deploy, or delete flow that runs after the write.

This must hold even when a live editor handle already exists before the write.

### R4. Freshness Metadata Must Not Be Lost

Global mode writes and any later metadata updates must preserve the freshness
metadata needed by existing editor staleness checks.

Freshness metadata changes must not erase the marker that distinguishes global
mode drafts from editor drafts.

The marker that distinguishes global mode drafts from editor drafts must not
erase freshness metadata.

### R5. Draft Enumeration Must Be Possible

Global mode must be able to enumerate frontend local drafts for a workspace and
a bounded set of item kinds.

Enumeration must include persisted-only drafts.

Enumeration must include live-only draft values.

Enumeration must include draft values that exist both in localStorage and in a
live editor handle.

Enumeration must not return duplicate logical entries for the same workspace,
item kind, and storage path.

### R6. Enumeration Must Expose Storage Identity

For each enumerated draft, global mode must be able to identify the exact
workspace, item kind, and storage path used by `UserDraft`.

This is required because global mode operates on workspace item paths, while
`UserDraft` storage paths are not always equal to item paths.

New-item editor drafts with an empty storage path must remain representable.

### R7. Enumeration Must Expose Persistence State

For each enumerated draft, global mode must be able to determine whether the
value exists in localStorage, in a live editor handle, or in both.

Global mode must not need to infer persistence state by reading private
`UserDraft` internals.

### R8. Enumeration Must Expose Draft Origin

For each enumerated draft, global mode must be able to determine whether the
draft is global-mode-originated or editor-originated.

Global mode must not need to infer origin from the draft value shape, item path,
item kind, or localStorage key.

### R9. Live-Only Values Must Not Be Deployable By Global Mode

A frontend local draft must not be deployable by global mode unless it is known
to be global-mode-originated and persisted.

Live-only values may be read as current frontend-visible context, but they must
not be treated as deployable AI drafts.

### R10. Current Context And Deployable Drafts Must Be Separatable

Global mode must be able to distinguish:

- current frontend-visible local state, which may include clean editor
  baselines, editor-originated drafts, live-only values, and global mode drafts;
- deployable global mode draft state, which must include only persisted
  global-mode-originated drafts.

This distinction must be available without inspecting private `UserDraft`
internals.

### R11. Clearing A Global Mode Draft Must Clear Persisted And Live State

When global mode clears a frontend local draft, the draft must be removed from
localStorage and from any live editor handle for the same workspace, item kind,
and storage path.

The cleared live state must not be re-persisted automatically as a side effect
of clearing.

Clearing a global mode draft must not clear unrelated editor drafts or unrelated
workspace items.

### R12. Existing Discard Semantics Must Remain Available

Editor flows must retain the ability to discard local draft state while
restoring a provided fallback value in memory.

Global mode clearing requirements must not remove or weaken that editor
discard behavior.

### R13. Enumeration Must Be Robust To Editor Runtime Values

Draft enumeration must not fail because a live editor draft contains
runtime-only values that cannot be serialized into localStorage.

Enumeration may omit runtime-only details, but it must preserve the draft data
needed for global mode to identify, read, list, and clear the draft.

### R14. UserDraft Must Remain Domain-Agnostic

`UserDraft` must remain independent of global mode workspace item semantics.

It must not need to know about scripts, flows, apps, schedules, triggers,
resources, variables, deployment behavior, or chat tools.

Global mode-specific interpretation of item kinds and draft values must remain
outside `UserDraft`.

### R15. Backend Drafts Must Remain Separate From UserDraft

Backend draft-only scripts, flows, and apps are not frontend `UserDraft`
entries.

Global mode must not require `UserDraft` to represent backend DB drafts.

Global mode must be able to combine backend draft-aware reads with frontend
`UserDraft` state without treating one storage system as the other.

### R16. Cross-Workspace Isolation Must Be Preserved

Draft reads, writes, enumeration, and clearing must remain scoped to the target
workspace.

Global mode must not see, modify, deploy, or clear drafts from another
workspace.

### R17. Item-Kind Boundaries Must Be Preserved

Draft reads, writes, enumeration, and clearing must remain scoped to the target
item kind.

Global mode must not treat a draft of one item kind as a draft of another item
kind.

### R18. Regression Coverage Is Required

The UserDraft-only change must include tests covering:

- preservation of existing editor behavior;
- distinction between editor-originated and global-mode-originated drafts;
- persisted-only draft enumeration;
- live-only draft enumeration;
- combined persisted and live draft enumeration without duplicates;
- storage path exposure when storage path differs from item path;
- global-mode-originated writes becoming visible to live handles and
  localStorage;
- clearing removing both persisted and live state;
- freshness metadata preservation;
- cross-workspace isolation;
- item-kind isolation;
- enumeration robustness when live draft values contain runtime-only data.
