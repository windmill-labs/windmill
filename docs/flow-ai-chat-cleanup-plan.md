# Flow AI Chat Cleanup Plan

## Goal

Make the flow AI chat code simple, DRY, and predictable by removing the split between real flow nodes and pseudo flow modules in mutation and review paths.

The main problem today is not that accept/reject or `set_module_code` are inherently complex. The problem is that different parts of the codebase use different traversal and lookup models:

- some paths resolve real stored nodes
- some paths resolve pseudo module wrappers
- some paths know about `aiagent.tools`
- some paths still only understand loops and branches

That makes small changes hard to review because it is not obvious whether a lookup returns the canonical object or a projection.

## Design Principles

- There must be exactly one canonical way to resolve a flow node by id.
- The canonical resolver must always return the real stored node, never a pseudo wrapper.
- Mutation code must only use canonical tree helpers.
- Pseudo modules are allowed only for display, search, or compatibility layers.
- `aiagent.tools` must be treated as a real nested container, not as a special fake-module case.
- Tree operations should be based on direct container access, not reconstructed paths and special cases.
- Core logic should live in pure TypeScript helpers with runnable `*.test.ts` coverage.

## Target Architecture

Introduce one small tree module, likely `frontend/src/lib/components/flows/flowTree.ts`, that becomes the only source of truth for real flow structure.

Recommended API:

- `findNode(flowValue, id)`
- `visitNodes(flowValue, visitor)`
- `getChildContainers(node)`
- `replaceNode(flowValue, id, nextNode)`
- `removeNode(flowValue, id)`
- `insertNode(flowValue, parentRef, index, node)`

`findNode` should return enough information to make all write operations trivial:

- the real stored node
- the owning container array
- the index inside that container
- parent metadata
- a small kind or location descriptor when useful

This is the key simplification. Once the caller has the real node and its real container, modify, accept, reject, add, remove, and move all become simple direct array operations.

## Scope Boundaries

The cleanup should focus first on tree semantics, not UI polish.

In scope:

- canonical node lookup
- canonical traversal
- canonical insert, replace, and remove operations
- flow chat mutation paths
- diff manager accept and reject paths
- delete and revert helpers
- test coverage for all container types

Out of scope for the first refactor pass:

- graph selection redesign
- broader AI tool UX cleanup
- search UX changes
- visual refactors

## Refactor Stages

### Stage 1: Extract canonical tree helpers

Create `flowTree.ts` and move the real structural knowledge there.

It should support:

- root modules
- `forloopflow`
- `whileloopflow`
- `branchone`
- `branchall`
- `aiagent.tools`
- `preprocessor_module`
- `failure_module`

The implementation should reuse the existing structural logic that already works in `flowDiff.ts`, especially around parent finding and nested container handling.

Deliverables:

- new `flowTree.ts`
- unit tests for lookup and container resolution
- no behavior change yet outside the new helper

### Stage 2: Migrate all write paths to the canonical tree

Replace mutation-time lookups in the following areas:

- flow chat helpers
- `set_module_code`
- diff manager accept and reject
- delete and revert helpers
- any helper that removes or replaces nodes by id

After this stage:

- mutation code must not use `previousResults.ts`
- mutation code must not use `flowExplorer.ts`
- any function named `find*ById` in a mutation path must resolve the real stored node

Deliverables:

- mutation logic uses only `flowTree.ts`
- add and remove work for AI agent tools through the same helpers as every other nested container

### Stage 3: Separate pseudo projections from real nodes

Keep `flowExplorer.ts` only for projection-oriented use cases such as search, listing, and display.

Make this boundary explicit:

- rename projection helpers so they sound read-only
- stop constructing ad hoc pseudo `FlowModule` objects in random call sites
- if a projection is needed, convert through a named adapter helper

At the end of this stage, reviewers should be able to tell immediately whether a given API returns a real node or a projection.

Deliverables:

- projection helpers renamed or documented as read-only
- mutation and diff code no longer rely on projection helpers

### Stage 4: Collapse duplicated lookup utilities

Remove or reduce duplicate helpers such as:

- `previousResults.dfs`
- `flows/dfs.ts` for generic node lookup use cases
- `chat/shared.findModuleById`
- `chat/flow/utils.getModuleById`
- `flowDiffManager.getModuleFromFlow`

Some helpers may remain for specialized needs, but there should be one canonical implementation and every generic lookup should delegate to it.

Deliverables:

- duplicate lookup code removed or turned into thin wrappers
- simpler call graph
- easier review and maintenance

### Stage 5: Simplify flow chat integration points

Once the tree model is unified, clean up the remaining chat-facing helper layer:

- keep `FlowAIChat` orchestration thin
- keep inline-script helpers focused on inline-script concerns only
- keep `previousResults.ts` focused on execution context and prior step semantics only
- keep diff manager focused on diff orchestration, not tree traversal

This stage should mostly be deletion and consolidation rather than new logic.

## Hard Rules After Refactor

- `previousResults.ts` must not be used for mutation.
- `flowExplorer.ts` must not be used for mutation.
- Any canonical lookup must return the real stored node.
- Any projection helper must be clearly named or documented as read-only.
- Add, remove, replace, and move must operate on a returned real container array.
- `aiagent.tools` must be handled by the same generic tree APIs as other nested containers.

## Testing Strategy

Keep the important logic in pure `*.test.ts` files rather than relying on `.svelte.test.ts`.

Required coverage:

- modify a root rawscript
- modify a loop child
- modify a branch child
- modify an AI agent tool
- remove an AI agent tool
- add an AI agent tool
- move a module across containers
- move an AI agent tool if supported
- accept and reject for each of the above where applicable

The acceptance rule for the refactor is simple:

- the same canonical helper API must power add, modify, remove, review, and revert for every container type

## Non-Goals

- Do not make pseudo modules the primary model.
- Do not combine tree cleanup with graph selection redesign.
- Do not mix this refactor with broad visual or UX changes.
- Do not introduce a heavy abstraction layer if a small set of pure helpers is enough.

## Recommended PR Sequence

1. Extract `flowTree.ts` with tests.
2. Migrate write paths and diff-manager mutation paths to `flowTree.ts`.
3. Mark projection helpers as read-only and remove them from mutation code.
4. Collapse duplicate `findModuleById` and traversal helpers.
5. Do any remaining UI or selection cleanup after the tree model is stable.

## Expected Outcome

If this plan is followed, the code should become easier to review because every reviewer can answer these questions immediately:

- Is this the real stored node or a projection?
- Which helper owns lookup semantics?
- Which helper owns replace and remove semantics?
- Does this code path treat AI tools the same way as other nested containers?

That is the main simplification target. The logic itself is not complicated once the codebase stops using multiple incompatible tree models.
