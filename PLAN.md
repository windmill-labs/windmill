# Plan 4: Flow Mode — Add Flow Search Tool with Auto-Search Behavior

## Problem

Flow mode can search for **scripts** (via `search_scripts`) but cannot search for **flows**. When building a new flow, users can't discover existing flows they could reference as sub-flows. The AI should proactively search for existing flows after any user request, just like it already searches for scripts.

## Goal

Add a `search_flows` tool to flow mode and instruct the AI to proactively search for existing flows and scripts after any request, enabling discovery and reuse of flow building blocks.

## Current Architecture

- **File**: `frontend/src/lib/components/copilot/chat/flow/core.ts`
- **Existing tools**: `search_scripts`, `search_hub_scripts`, `resource_type`, `get_instructions_for_code_generation`, `get_db_schema`, `set_flow_json`, `inspect_inline_script`, `set_module_code`, `test_run_flow`, `test_run_step`, `get_lint_errors`
- **`search_scripts`** uses `WorkspaceScriptsSearch` (uFuzzy over `ScriptService.listScripts()`)
- **No flow search** — the AI cannot find existing flows to reuse as sub-flow modules

## Implementation

### 1. Add `search_flows` tool

```typescript
const searchFlowsSchema = z.object({
  query: z
    .string()
    .describe('The query to search for, e.g. "process invoices", "send notification", etc.')
})

const searchFlowsToolDef = createToolDef(
  searchFlowsSchema,
  'search_flows',
  'Search for flows in the workspace. Returns array of {path, summary} objects. Use this to find existing flows that can be referenced as sub-flow modules.'
)
```

### 2. Add `WorkspaceFlowsSearch` class (or reuse shared class)

Either reuse the shared `WorkspaceRunnablesSearch` from Plan 1, or add a dedicated flow search class:

```typescript
class WorkspaceFlowsSearch {
  private uf: uFuzzy
  private workspace: string | undefined = undefined
  private flows: Flow[] | undefined = undefined

  constructor() { this.uf = new uFuzzy() }

  private async init(workspace: string) {
    this.flows = await FlowService.listFlows({ workspace })
    this.workspace = workspace
  }

  async search(query: string, workspace: string) {
    if (this.flows === undefined || this.workspace !== workspace) {
      await this.init(workspace)
    }
    const flows = this.flows
    if (!flows) throw new Error('Failed to load flows')

    const results = this.uf.search(
      flows.map((f) => (emptyString(f.summary) ? f.path : f.summary + ' (' + f.path + ')')),
      query.trim()
    )
    return results[2]?.map((id) => ({
      path: flows[id].path,
      summary: flows[id].summary
    })) ?? []
  }
}
```

### 3. Add `get_flow_details` tool (to inspect found flows)

```typescript
const getFlowDetailsSchema = z.object({
  path: z.string().describe('The path of the flow to inspect')
})

const getFlowDetailsToolDef = createToolDef(
  getFlowDetailsSchema,
  'get_flow_details',
  'Get details of a workspace flow including its modules structure, input schema, and module summaries. Use after search_flows to understand a flow before referencing it.'
)
```

Implementation calls `FlowService.getFlowByPath()` and returns:
- Flow summary, description
- Input schema
- Module structure (ids, summaries, types, paths for script references)

### 4. Update system prompt — Proactive Search Instruction

This is the key difference from script mode. The system prompt should instruct the AI to **always search for existing flows** when creating or modifying a flow:

Update the "Creating Flows" section in `prepareFlowSystemMessage()`:

```
### Creating Flows

1. **Search for existing flows and scripts first** (unless user explicitly asks to write from scratch):
   - First: `search_flows` to find existing flows that could be reused as sub-flow modules
   - Then: `search_scripts` to find workspace scripts
   - Then: `search_hub_scripts` (only consider highly relevant results)
   - Only create raw scripts if no suitable script or flow is found

2. **When referencing an existing flow as a sub-module:**
   - Use `get_flow_details` to understand the flow's input schema
   - Use `type: "flow"` with `path` in the module value
   - Define `input_transforms` matching the referenced flow's input schema
```

### 5. Register tools

Add to the `flowTools` array:

```typescript
export const flowTools: Tool<FlowAIChatHelpers>[] = [
  // ... existing tools
  {
    def: searchFlowsToolDef,
    fn: async ({ args, workspace, toolId, toolCallbacks }) => {
      toolCallbacks.setToolStatus(toolId, {
        content: 'Searching for workspace flows related to "' + args.query + '"...'
      })
      const parsedArgs = searchFlowsSchema.parse(args)
      const flowResults = await workspaceFlowsSearch.search(parsedArgs.query, workspace)
      toolCallbacks.setToolStatus(toolId, {
        content: 'Found ' + flowResults.length + ' flows in the workspace related to "' + args.query + '"'
      })
      return JSON.stringify(flowResults)
    }
  },
  {
    def: getFlowDetailsToolDef,
    fn: async ({ args, workspace, toolId, toolCallbacks }) => {
      // ... implementation
    }
  }
]
```

## Files to Modify

| File | Change |
|------|--------|
| `frontend/src/lib/components/copilot/chat/flow/core.ts` | Add `search_flows` and `get_flow_details` tools, update system prompt, add search class, register tools |
| `frontend/src/lib/components/copilot/chat/shared.ts` | Ensure shared flow search utility if extracted |

## Testing

1. Open a flow in flow mode
2. Ask "Create a flow that sends a notification after processing an invoice"
3. AI should proactively search for existing flows related to "notification" and "invoice"
4. AI should search for existing scripts too
5. If matches are found, AI should propose reusing them
6. Ask "I have a flow for Stripe webhook handling, can I add it as a sub-flow here?"
7. AI should find the flow, inspect its inputs, and add it as a `type: "flow"` module
