---
name: ai-evals
description: Author and run black-box benchmark cases for the Windmill AI generation modes (flow/app/script/cli/global) in ai_evals/. Use when adding or changing eval cases, or when running before/after benchmarks for AI chat / copilot changes.
---

# AI evals — authoring and running benchmark cases

`ai_evals/` is a black-box benchmark runner for the Windmill AI generation modes:
`flow`, `app`, `script`, `cli`, `global`. It always tests the **current** production
prompts, tools, and guidance in this checkout. Each attempt runs the real production
path, deterministic validation, then LLM judging.

The goal is to test current production guidance with realistic user requests — **not**
to pin one exact implementation shape.

## Running benchmarks

```bash
cd ai_evals
bun install                       # first time; frontend modes also need `cd frontend && bun install`
bun run cli -- models             # list model aliases
bun run cli -- cases global       # list cases for a mode
bun run cli -- run global global-test1-script-create --model sonnet
```

Frontend modes (`flow`/`script`/`app`/`global`) route model calls through a Windmill
backend's `/api/w/<ws>/ai/proxy`, so you need **any** reachable backend:

```bash
WMILL_AI_EVAL_BACKEND_URL=http://127.0.0.1:<port> WMILL_AI_EVAL_BACKEND_WORKSPACE=integration-tests \
  bun run cli -- run global <caseIds...> --models sonnet,gpt-5.5,gemini-3.1-pro-preview
```

- **Reuse an existing workspace.** CE builds cap workspaces, so temp-workspace
  creation 400s ("reached workspace limit"). Always set
  `WMILL_AI_EVAL_BACKEND_WORKSPACE=integration-tests` (or any existing workspace) to
  reuse one. The only side effect of a run is upserting an `f/evals/ai/<provider>`
  resource there.
- Provider keys live in `ai_evals/.env` and are auto-loaded by bun. The judge is a
  separate Anthropic call (default `claude-sonnet-4-6`) regardless of the model under
  test.

## Authoring core rules

1. Write prompts like a real user request.
2. Prefer behavior, inputs, constraints, and outcomes over internal implementation.
3. Keep deterministic validation narrow and hard.
4. Put semantic expectations in `judgeChecklist`.
5. Use `expected` fixtures only when exact structure really matters.

### Prompt writing

Prompts should sound like something a user would naturally ask. Do not write prompts
as if the user knows Windmill internals unless the case explicitly tests a power-user
workflow.

Good:
- "Create a flow that routes support requests based on customer tier."
- "Add a reset button that sets the counter back to 0."
- "Create a flow that reuses the existing greeting script instead of duplicating the logic."

Bad:
- "Use `branchone` with 3 branches and a default branch."
- "Create a `rawscript` step with this exact topology."
- "This is a benchmark harness."

### Deterministic validation

Use deterministic checks only for hard failures: missing required files; unexpected
extra files when the prompt says not to create them; syntax errors; unresolved flow
refs; missing required special modules or suspend config; obvious corruption.

Do **not** encode one preferred implementation. Bad hard checks: exact step topology
for a creation flow; exact branch structure when the prompt only asked for routing;
exact input shape when multiple reasonable shapes are acceptable.

### Judge checklist

Every non-trivial case should have a `judgeChecklist` capturing user-visible behavior
that must be present, important constraints, and key completion criteria — not
low-level implementation details unless truly required.

Good: "the flow calculates the order total with 8% tax"; "the flow reuses the existing
workspace script instead of rewriting the logic". Bad: "uses `branchone`"; "contains a
`rawscript` node".

See `ai_evals/README.md` for the full case format, fields, and fixture details.