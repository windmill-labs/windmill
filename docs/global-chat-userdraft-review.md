# UserDraft-backed Global Draft Review

The patch introduces functional issues in the UserDraft-backed global draft integration: live editor deploys can wipe the open editor, raw-app draft reads bypass the metadata-only contract, and prefix-limited listing can include unrelated local drafts.

## Findings

### [P2] Preserve live editor state after chat deploy

File: `frontend/src/lib/components/copilot/chat/global/core.ts:2809`

When the draft being deployed is also open in a script or flow editor, this call resolves the draft to the editor's UserDraft slot and then `deleteGlobalDraft` uses `UserDraft.clear(..., undefined)`, which resets the live handle. The script routes only render `ScriptBuilder` while `scriptHandle.draft` exists, and flow routes fall back to an empty flow, so a successful chat deploy can blank the open editor instead of leaving the deployed content or navigating away.

### [P2] Summarize raw-app drafts in read_workspace_item

File: `frontend/src/lib/components/copilot/chat/global/core.ts:1372-1377`

When a raw app has a UserDraft-backed local draft, this early return serializes the full `AppDraftValue` rather than the metadata summary used for deployed apps below. That contradicts the new raw-app tool contract that `read_workspace_item` returns metadata only and can dump every draft file/runnable into chat; app drafts should be summarized here and file contents left to `read_app_file`.

### [P2] Apply path_prefix to local drafts

File: `frontend/src/lib/components/copilot/chat/global/core.ts:1341-1346`

When callers pass `path_prefix`, backend items are filtered by `listWorkspaceItems`, but UserDraft-backed local drafts are merged unconditionally here. Because this now includes persisted/open editor drafts, `list_workspace_items({ path_prefix: 'f/foo/' })` can return unrelated drafts outside the requested prefix and consume the result limit before matching items.
