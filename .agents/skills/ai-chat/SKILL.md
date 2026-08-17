---
name: ai-chat
description: Guidance for improving the Windmill AI chat (copilot), especially global mode — tools, prompts, and context-window discipline. Use when editing chat tools, system prompts, or tool-result shapes under frontend/src/lib/components/copilot/chat, or when changing how the chat manages its context window.
---

## Always benchmark before and after

No context or behavior change ships without an `ai_evals` A/B on the affected mode.
Add or adjust cases for exactly what you changed — see the `ai-evals` skill for
authoring and the full run reference.

Run the affected mode **before** your change and **after**, same model(s), same cases.

## Measure the window first, and cumulative second

Optimize **`finalContextTokens`** (window occupancy — what drives overflow and
compaction), then cumulative prompt tokens.

## Context discipline

The dominant fixed cost is per-iteration overhead: the system prompt **plus every
tool schema** is re-sent on every loop iteration. So:

- **Every tool and every parameter is a permanent tax.** Justify each one and measure
  it; an extra "locate" round-trip can cost more than the reads it saves. Strip dead
  params rather than leaving them in the schema.
- **Tool results return the minimum.** Never echo content the model already has. The
  canonical mistake: a write tool that returns the whole edited artifact right after
  the model authored it — return `{ success, message }` instead. When you touch a
  *shared* write helper (e.g. `finishAppDraftWrite` in `global/core.ts`), re-check
  this invariant for **all** the write tools routing through it — the echo has
  regressed before via a shared refactor.

## Prompts and tool descriptions are part of the surface

The system prompt and tool descriptions steer behavior as much as the tools
themselves, and are benchmarkable the same way. A description that advertises
truncation makes the model self-limit; the path-conventions block changes where
drafts land. Treat prompt/description edits as real changes and A/B them — a
pure-prompt change is a legitimate, measurable improvement.