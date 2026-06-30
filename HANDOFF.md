# Handoff: AI-chat thinking-blocks fix (PR #9841)

Branch: `glm/fix-copilot-thinking-blocks` · base `main` · file touched: `frontend/src/lib/components/copilot/chat/anthropic.ts`

## The bug
Global AI chat (`/sessions`, dev-gated) 400s when an assistant turn uses the native
`web_search` tool while thinking is on AND ends in a tool call:

```
400 invalid_request_error
messages.N.content.M: `thinking` or `redacted_thinking` blocks in the latest assistant
message cannot be modified. These blocks must remain as they were in the original response.
```

Root cause: the turn was reconstructed keeping only thinking blocks, reordering them to the
front and dropping the `server_tool_use` / `web_search_tool_result` blocks. With interleaved
thinking (a 2nd thinking block after the web-search result) the 2nd block's signature no
longer validates.

## What's already done (1 commit)
`frontend/src/lib/components/copilot/chat/anthropic.ts`:
- `parseAnthropicCompletion`: store the full turn as `_anthropicContent = finalMessage.content`
  (replaces the thinking-only `_anthropicThinkingBlocks` extraction).
- `convertOpenAIToAnthropicMessages`: if `_anthropicContent` is present, emit it **verbatim**;
  skip the redundant standalone text message for that turn; the old `_anthropicThinkingBlocks`
  path is kept as a fallback for sessions persisted before this change.
- `npm run check:fast` passes. Do NOT commit `frontend/package-lock.json` churn (revert if the
  install/check rewrites it).

## Verify (still TODO)
1. Run this worktree's frontend against the EE backend: from `frontend/`,
   `REMOTE=http://localhost:8000 npm run dev`; open it in a browser.
2. Unlock global chat: DevTools console -> `localStorage.setItem('wm_dev_global_ai','1')` -> reload.
3. Workspace **demo-orange** -> AI Sessions -> new session (Claude model, effort `high`).
4. Prompt: "Search the web for the current NIST SP 800-63B minimum password length. After you
   get the number, in a follow-up step list the resources in my workspace. Think step by step
   between each tool call."
5. Expected: Thinking -> web search -> Thinking -> list resources, and the auto-continuation
   no longer 400s (on `main` it does). Capture the body to
   `/api/w/demo-orange/ai/proxy/v1/messages`: the replayed assistant content should keep
   `thinking`, `server_tool_use`, `web_search_tool_result` in original order (not
   `["thinking","thinking","tool_use"]`).

Only NEW sessions are fixed — sessions already persisted in the old shape will still fail on
continue; start a fresh session to test.

## Remaining tasks
- Confirm the repro is fixed in-app; attach before/after screenshots to PR #9841.
- Add a unit test for `convertOpenAIToAnthropicMessages` (near `AIChatManager.test.ts` /
  `global/core.test.ts`): a turn with `_anthropicContent =
  [thinking, server_tool_use, web_search_tool_result, thinking, tool_use]` round-trips verbatim,
  and the preceding standalone text message is skipped.
- Sanity-check a plain text turn (no tools) and an old-style session (only
  `_anthropicThinkingBlocks`) still convert via the fallback.
- `npm run check` (full) before marking PR ready.
- Remove this HANDOFF.md before the PR is marked ready for review.

## Deploying to orange-dev preview
Frontend-only fix — cherry-pick the commit onto the preview's frontend branch and rebuild the
frontend bundle (no backend/DB change). Already-broken sessions stay broken; verify in a new chat.
