# UserDraft Path Key Investigation Prompt

Investigate whether we can stop using empty-path UserDraft keys for new-item editors and instead bind UserDraft to the actual editor path as soon as that path is known.

Context:

- Today some editors use keys like `userdraft/w/{workspace}/{kind}/` for new items.
- The actual auto-assigned path may live inside editor state or the draft value.
- This makes non-editor consumers like global AI chat hard to implement because `UserDraft.list()` sees the empty storage path instead of the user-facing workspace path.
- Preferred direction: editors should call or rebind to `UserDraft.use(kind, actualPath)` as soon as the actual path is available, instead of relying on UserDraft or global chat to infer paths from draft values.

Please investigate complexity and risk for every `UserDraftItemKind`:

- `script`
- `flow`
- `app`
- `raw_app`
- `resource`
- `variable`
- `trigger_schedule`
- `trigger_http`
- `trigger_websocket`
- `trigger_kafka`
- `trigger_nats`
- `trigger_postgres`
- `trigger_mqtt`
- `trigger_sqs`
- `trigger_gcp`
- `trigger_azure`

Questions to answer:

1. Which editors currently call `UserDraft.use(..., '')` or otherwise use an empty/new-item path?
2. For each kind, where does the actual path become known?
3. Can that path be known before `UserDraft.use(...)` is called?
4. If not, can the editor safely rebind from `''` to `actualPath` after the path becomes known?
5. What happens to the old empty-path draft during rebind? Should it be moved, copied, discarded, or left until save?
6. How do discard/reset flows currently work, especially through `LocalDraftStaleModal`, save buttons, deploy buttons, and route navigation?
7. Would changing the key break reload restoration for new-item pages?
8. Would it break editor-open live state, debounced persistence, or staleness metadata?
9. Does any item kind need a different design because its path is not known until save, lives outside the draft value, or can change via a rename field?
10. What tests should be added before attempting the change?

Please produce:

- A table by item kind with current key behavior, where the actual path lives, feasibility, risk level, and required code changes.
- A recommended migration strategy for existing empty-path drafts.
- A minimal staged implementation plan, starting with the safest item kind.
- Any cases where this direction should not be applied.
