# AI Evals Authoring Guide

This folder contains black-box benchmark cases for:

- `flow`
- `app`
- `script`
- `cli`

The goal is to test the current production prompts and guidance with realistic user requests, not to test one exact implementation shape.

## Core rules

1. Write prompts like a real user request.
2. Prefer behavior, inputs, constraints, and outcomes over internal implementation details.
3. Keep deterministic validation narrow and hard.
4. Put semantic expectations in `judgeChecklist`.
5. Use `expected` fixtures only when exact structure really matters.

## Prompt writing

Prompts should sound like something a user would naturally ask.

Good:

- "Create a flow that routes support requests based on customer tier."
- "Add a reset button that sets the counter back to 0."
- "Create a flow that reuses the existing greeting script instead of duplicating the logic."

Bad:

- "Use `branchone` with 3 branches and a default branch."
- "Create a `rawscript` step with this exact topology."
- "This is a benchmark harness."

Do not write prompts as if the user knows Windmill internals unless the case is explicitly testing a power-user workflow.

## Flow-specific rules

This is the main principle you asked for:

- flow prompts should read like requests from a user who does not know the product internals
- the user should ask for behavior, not for `branchone`, `branchall`, `rawscript`, `preprocessor_module`, `failure_module`, exact graph topology, or other internal constructs

That means:

- creation cases should describe the business behavior and expected result
- modification cases may mention existing step names, because the user can see the current flow
- only mention special Windmill constructs when the case is explicitly about those constructs

Examples:

- acceptable creation prompt:
  "Create a purchase approval flow that pauses for approval and asks the approver for a comment."
- avoid:
  "Create a suspend step with one required event and a resume form."

For flow cases, do not fail a case just because the model chose a different valid topology.

## App-specific rules

App prompts should focus on user-visible behavior:

- what the UI should let the user do
- what should persist
- what backend behavior is needed

Avoid prompting in terms of React structure, component names, or implementation unless the case is specifically about editing an existing app.

## CLI-specific rules

CLI prompts can be more explicit about paths and file names because real CLI users often do specify them.

Still, avoid benchmark phrasing. The prompt should read like a repo task, not a harness instruction.

When relevant, ask the assistant to tell the user which `wmill` commands to run next. That is part of the benchmarked behavior.

## Deterministic validation

Use deterministic validation only for hard failures such as:

- missing required files
- unexpected extra files when the prompt says not to create them
- syntax errors
- unresolved flow refs
- missing required special modules or suspend config
- obvious artifact corruption

Do not use deterministic validation to enforce one preferred implementation for broad creation tasks.

Examples of bad hard checks:

- exact step topology for a creation flow
- exact branch structure when the prompt only asked for routing behavior
- exact input shape when multiple reasonable shapes are acceptable

## Judge checklist

Every non-trivial case should have a `judgeChecklist`.

The checklist should capture:

- the user-visible behavior that must be present
- important constraints
- key completion criteria

The checklist should not duplicate low-level implementation details unless they are truly required by the task.

Good checklist items:

- "the flow calculates the order total with 8% tax"
- "the app persists recipes appropriately for a raw Windmill app"
- "the flow reuses the existing workspace script instead of rewriting the logic"

Bad checklist items:

- "uses `branchone`"
- "contains a `rawscript` node"

## When to use `expected`

Use `expected` fixtures when the case is structure-sensitive, for example:

- exact file creation
- exact script content
- modification cases where a specific file must change in a specific way
- cases where preserving an existing structure is part of the requirement

Do not use a full `expected` artifact as the semantic oracle for broad creation tasks when multiple valid outputs should pass.

## When to use `initial`

Use `initial` when the benchmark is about:

- editing an existing artifact
- reusing existing workspace assets
- preserving existing behavior while adding a change

If the case is greenfield, prefer no `initial`.

## Case design ladder

Prefer suites that get gradually harder:

1. trivial create case
2. realistic create case
3. reuse-existing-assets case
4. modification case
5. refactor case
6. edge-case or niche product behavior

The last cases in a suite should cover unusual or product-specific behavior.

## Anti-patterns

Avoid these:

- benchmark framing in prompts
- over-specified internal topology for creation tasks
- judge checklists that just restate implementation details
- deterministic validation that encodes one preferred solution
- fixtures that are so minimal or brittle that they create false negatives

## Before adding a case

Ask:

1. Would a real user plausibly write this prompt?
2. If the model solves it in a different valid way, would the case still pass?
3. Are the hard deterministic checks only catching objectively broken output?
4. Does the `judgeChecklist` describe the real success criteria?
5. If this case fails, will the reason be understandable from the saved artifacts?
