# Global AI Chat — Context Optimization for Raw Apps

How the global-mode AI chat manages context today, why it fills up fast on big
raw-app projects, and a prioritized plan to fix it. Techniques are borrowed from
how Claude Code keeps a long agentic session bounded.

## Where the relevant code lives

- Agentic loop: `frontend/src/lib/components/copilot/chat/chatLoop.ts`
- Chat state / compaction / token accounting:
  `frontend/src/lib/components/copilot/chat/AIChatManager.svelte.ts`
- Global tools (scripts, flows, apps, triggers, …) and raw-app file tools:
  `frontend/src/lib/components/copilot/chat/global/core.ts`
- Raw-app client-side bundler bridge:
  `frontend/src/lib/components/copilot/chat/global/rawAppBundlerBridge.ts`

## What it already does well

The chat is tool-based and mostly *lazy*, which is the right foundation:

- `read_workspace_item type=app` returns **metadata only** — frontend file paths +
  sizes and the runnable list, no contents (`global/core.ts` `summarizeAppValue`).
- Contents are pulled on demand, one file at a time, via `read_app_file`.
- Edits are surgical (`patch_app_file`) rather than full rewrites.
- Flow reads use a *compact view*: inline script bodies are replaced by an
  `inline_script.<moduleId>` placeholder so they don't bloat tool I/O.
- Logs/runs tools are count-capped (`get_app_runtime_logs` ≤100, `list_app_runs` ≤100).
- A compaction safety net exists: drop-oldest at 80%→70% of the context window
  (`COMPACTION_TRIGGER_RATIO` / `COMPACTION_TARGET_RATIO`, `compactOldestMessages`).

## Why context still fills up fast on big projects

The loop re-sends the **entire** message history every iteration
(`chatLoop.ts`, `messageParams = [systemMessage, ...messages]`) and nothing caps
or dedupes what accumulates:

1. **`read_app_file` is uncapped.** It returns the whole file verbatim (no
   `offset`/`limit`). One large component or generated file lands in history in
   full and stays for the rest of the session. *Biggest single lever.*
2. **Re-reads duplicate.** Over a long edit session the model re-reads the same
   file repeatedly; every copy persists at full size.
3. **No tool-result truncation.** `read_app_file` output (and, less so, logs/runs)
   accumulates at full size — no per-result cap.
4. **Compaction is lossy and counter-productive.** Drop-oldest deletes the
   *oldest* messages — exactly the early file reads and plan the model relies on —
   so it then re-reads those files and re-bloats. It also busts the Anthropic
   prompt cache (the cached prefix changes), so every post-compaction turn pays
   full input cost.
5. **~28 tool schemas are always present** (`globalTools`), even in a pure
   raw-app session where the script/flow/trigger/resource/variable/schedule tools
   are dead weight re-sent every turn.

For reference, Claude Code keeps context bounded with: Read capped at ~2000
lines / ~25K tokens with `offset`/`limit`; an "unchanged file → refer to earlier
read" stub; large tool results persisted out-of-context behind a reference;
summarization-based compaction that re-injects only the ~5 most-recent file
reads; native `clear_tool_uses` microcompaction (keeps the last ~40K tokens of
tool I/O); and deferred tool-schema loading.

## Prioritized plan

### A. Cap and range-limit `read_app_file` *(highest leverage, cache-safe)*
Add `offset`/`limit` and a default cap (~1500 lines / ~50K chars). Large files
return a head slice plus a "truncated — call again with offset N" annotation.
Bounds per-read cost at the source so history grows slower and compaction fires
less often. Because it caps at write time it never rewrites history → the prompt
cache stays valid.

### B. Dedupe re-reads *(built, benchmarked, then dropped)*
Tracked, per conversation, the file content already returned, returning a short
stub on a repeat read of an unchanged file still in context (keyed off the
originating tool-call id so it self-heals after compaction). **Removed before
shipping:** verified across sonnet/opus/gpt-5.5/haiku that no model re-reads a
file within a conversation — they read each file once and keep it in context — so
the dedupe never fired in any real run (the micro-benchmark forces a re-read to
prove the mechanism works). It was a correct but unused guard, so it was dropped
to keep the change focused; only the cap (A) ships.

### C. Replace drop-oldest with tool-result pruning *(later)*
Instead of deleting whole oldest messages, stale the *contents* of old
`read_app_file` results in place (`[content omitted — re-read if needed]`) while
keeping the message/tool structure. Recovers the most tokens (file bodies
dominate) with the least information loss, and avoids the re-read→re-bloat loop.
Mirrors Claude Code's `clear_tool_uses`. Keep true drop-oldest only as a last
resort.

### D. Trim the tool surface per session *(later)*
Gate the toolset (or defer-load schemas) so a raw-app session doesn't carry the
flow/trigger/resource/variable tools. Lower headline impact but it's fixed
per-turn overhead and helps keep the cache prefix stable.

### E. `search_app` grep tool *(recommended — natural third piece of the reading triad)*
For big projects, locating where a symbol/string lives currently forces the model
to read many whole files. A grep-style tool that searches across all frontend
files + inline backend runnable contents of one app and returns only matching
`file:line` rows (head-capped, e.g. ≤100 matches) is extremely
context-efficient. It completes the **list → search → ranged-read** loop:
`read_workspace_item` to list files, `search_app` to locate, `read_app_file`
(with `offset`/`limit`) to inspect. Entirely client-side (the draft value is
already in memory), so no backend work. Strongly recommended as the immediate
follow-up to A + B.

### F. Subagent for read-heavy exploration *(largest change)*
Delegate "find/understand X across the app" to a sub-agent whose intermediate
reads never touch the parent context; only its summary returns. Worth it only if
A–E aren't enough.

## What shipped

**A only** (`read_app_file` cap + `offset`/`limit` line paging +
`char_offset`/`char_limit` char paging) in `global/core.ts`, plus a global raw-app
eval harness (large `analytics_dashboard` fixture + cases `global-test29/30/31/32`)
and a deterministic micro-benchmark. **B was built and dropped** (see B above).

## Measured results (ai_evals global mode)

The cap helps only when a model would read a large file in full — which is
**model- and run-dependent**, so the LLM token deltas are noisy. The deterministic
micro-benchmark (forces the read pattern, no model noise) is the source of truth:
it shows the cap cuts returned bytes >50% on a realistic large-read pattern.

| Model | test31 (large-file inspect) | Note |
|---|---|---|
| haiku | 376k → 199k (**−47%**) | always reads the big file whole → biggest win |
| sonnet | 262k → 231k (−12%; −24% another run) | partial |
| gpt-5.5 | −15% (read full) … +28% (self-limited) | swing is read-*choice* variance, not the cap |
| opus | flat | reads selectively → cap inert |

**Key finding.** On small-file tasks the model reads a few hundred lines once
(~4k tokens), so file content is ≈2% of the ~200k-token context; the rest is
**fixed per-iteration overhead** (system prompt + ~28 tool schemas re-sent every
turn). The cap only moves the needle when a model would otherwise dump a large
file into context — a worst-case guardrail, not an everyday win. It never
regresses correctness on any model.

**Implication for next work.** The dominant lever for typical tasks is
**D (trim/defer the ~28 tool schemas + system prompt)**, not reads. A separate
investigation also found a **write-path bloat** (the app-draft write echoing the
full app value per edit) that looks like a bigger source of context growth than
reads. C (tool-result pruning) and E (`search_app`) remain options for
read-heavy/big-project flows — though a standalone `search_app` A/B showed it did
not improve token efficiency.
