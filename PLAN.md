# Plan 1: Navigator Mode — Add Workspace Script & Flow Search Tool

## Problem

Navigator mode currently has 5 tools (`get_triggerable_components`, `trigger_component`, `get_documentation`, `get_current_page_name`, `get_available_resources`), but **none of them can search or inspect workspace scripts and flows**. When a user asks "What scripts do I have for sending emails?" or "Show me the Stripe flows", the navigator has no way to answer.

## Goal

Add a `search_workspace` tool to navigator mode that lets the AI search for scripts and flows by query, returning summaries and input schemas so the AI can help users discover and understand their building blocks.

## Current Architecture

- **File**: `frontend/src/lib/components/copilot/chat/navigator/core.ts`
- **Existing tools**: 5 tools (page scanning, triggering, docs, resources)
- **No search infrastructure**: Navigator has no fuzzy search, no script/flow listing

## Implementation

### 1. Add `WorkspaceSearch` class to `navigator/core.ts`

Reuse the same pattern as `WorkspaceRunnablesSearch` from `app/core.ts` (which already searches both scripts and flows with uFuzzy). Either:
- **Option A (Recommended)**: Extract `WorkspaceRunnablesSearch` into `shared.ts` as a shared utility, then import it in both `app/core.ts` and `navigator/core.ts`.
- **Option B**: Duplicate the class in `navigator/core.ts` (simpler but violates DRY).

```typescript
// shared.ts — new shared class
import uFuzzy from '@leeoniya/ufuzzy'
import { ScriptService, FlowService, type Script, type Flow } from '$lib/gen'
import { emptyString } from '$lib/utils'

export class WorkspaceRunnablesSearch {
  private uf: uFuzzy
  private workspace: string | undefined = undefined
  private scripts: Script[] | undefined = undefined
  private flows: Flow[] | undefined = undefined

  constructor() { this.uf = new uFuzzy() }
  // ... existing implementation from app/core.ts
}
```

### 2. Add `get_runnable_details` tool

After a user searches and finds a script/flow, they may want to know its inputs. Add a second tool:

```typescript
const GET_RUNNABLE_DETAILS_TOOL: ChatCompletionTool = {
  type: 'function',
  function: {
    name: 'get_runnable_details',
    description: 'Get details (summary, description, inputs schema) of a specific script or flow by path',
    parameters: {
      type: 'object',
      properties: {
        path: { type: 'string', description: 'The path of the script or flow (e.g. "f/marketing/send_email")' },
        type: { type: 'string', enum: ['script', 'flow'], description: 'Whether this is a script or a flow' }
      },
      required: ['path', 'type']
    }
  }
}
```

Implementation calls `ScriptService.getScriptByPath()` or `FlowService.getFlowByPath()` and returns:
- `summary`, `description`
- `schema` (input parameters)
- `language` (for scripts)
- `path`
- For flows: number of modules, module summaries

### 3. Add `search_workspace` tool

```typescript
const SEARCH_WORKSPACE_TOOL: ChatCompletionTool = {
  type: 'function',
  function: {
    name: 'search_workspace',
    description: 'Search for scripts and flows in the workspace. Use this when a user asks about existing building blocks, wants to find a script/flow, or asks "what do I have for X".',
    parameters: {
      type: 'object',
      properties: {
        query: { type: 'string', description: 'Search query (e.g. "stripe", "send email", "ETL")' },
        type: { type: 'string', enum: ['all', 'scripts', 'flows'], description: 'Filter by type. Defaults to "all".' }
      },
      required: ['query']
    }
  }
}
```

### 4. Update system prompt

Add to the navigator system prompt:

```
You have access to these tools:
...
5. Search for scripts and flows in the workspace (search_workspace)
6. Get detailed information about a specific script or flow (get_runnable_details)

- When users ask about existing scripts, flows, or building blocks in their workspace, use search_workspace to find them.
- When users want to understand a specific script or flow (inputs, description, what it does), use get_runnable_details.
- When users ask you to navigate to a specific script or flow, first search for it, then use the navigation tools to go to it.
```

### 5. Register tools

Add the two new tools to the `navigatorTools` array:

```typescript
export const navigatorTools: Tool<{}>[] = [
  getTriggerableComponentsTool,
  triggerComponentTool,
  getDocumentationTool,
  getCurrentPageNameTool,
  getAvailableResourcesTool,
  searchWorkspaceTool,      // NEW
  getRunnableDetailsTool    // NEW
]
```

## Files to Modify

| File | Change |
|------|--------|
| `frontend/src/lib/components/copilot/chat/shared.ts` | Extract `WorkspaceRunnablesSearch` here |
| `frontend/src/lib/components/copilot/chat/navigator/core.ts` | Add 2 new tools, update system prompt, import search class |
| `frontend/src/lib/components/copilot/chat/app/core.ts` | Import shared `WorkspaceRunnablesSearch` instead of local one |

## Testing

1. Open Windmill in navigator mode
2. Ask "What scripts do I have for sending emails?"
3. AI should use `search_workspace` and return results
4. Ask "Tell me more about f/marketing/send_email"
5. AI should use `get_runnable_details` and return schema + description
6. Ask "Navigate to my Stripe integration flow"
7. AI should search, find the flow, then use navigation tools to go to it
