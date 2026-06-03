# Windmill — Drafts & Fork Compare

Domain language for unsaved/uncommitted changes to runnables (scripts, flows, apps) and the fork↔parent comparison flow. Created while reworking the draft-count architecture; extend as terms are resolved.

## Language

### Drafts

**Workspace Drafts**:
The set of deployable **Draft Items** in a workspace — the single answer to "which drafts exist here?". Lists once; the **Draft Count** is its length.
_Avoid_: draft list, pending changes.

**Draft Item**:
A runnable that has a pending draft (`has_draft`) or exists only as a draft (`draft_only`). Carries `{ kind, path, summary, draft_only, raw_app }`.
_Avoid_: draft entry, change.

**Server Draft**:
A draft persisted server-side (the `draft` table, workspace-scoped, plus `draft_only` runnables). What the compare page and the session draft bar read.
_Avoid_: bare "draft" when the local/server distinction matters.

**Local Draft (UserDraft)**:
A draft the AI chat keeps in the browser (`UserDraft`, localStorage). The AI proposes these and deploys them directly; they are **not** Server Drafts and are invisible to the Draft Count.
_Avoid_: bare "draft".

**Draft Count**:
The number of Draft Items in a workspace. Defined as the length of the Workspace Drafts list — never a separate query.
_Avoid_: draft total, pending count.

**`draft_only`** / **`has_draft`**:
States of a runnable: `draft_only` = exists only as a draft, never deployed; `has_draft` = a deployed runnable with a pending draft.

**Deploy a draft**:
Promote a draft to the deployed version of the runnable, in the same workspace (`draft_only` → first deploy, `has_draft` → update). Distinct from merging fork→parent.
_Avoid_: publish, save, merge.

**Discard a draft**:
Remove the draft. For a `draft_only` runnable this deletes the runnable; for `has_draft` it drops the draft and leaves the deployed version.
_Avoid_: delete, revert.

### Fork compare

**Fork** / **parent**:
A fork (`wm-fork-*`) is a workspace cloned from a parent's *deployed* state. Drafts are per-workspace; a fork does not see the parent's drafts.

**Deploy to parent** / **Update current**:
The two directions of a fork↔parent merge — push the fork's deployed changes up to the parent, or pull the parent's down into the fork. Distinct from "Deploy a draft".
