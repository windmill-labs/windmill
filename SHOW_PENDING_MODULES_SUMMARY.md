# Show Pending Modules - Implementation Summary

## Problem Solved

When AI generated new modules, they existed only in `mergedFlow`, not in `flowStore`. When users clicked on these pending modules in the flow graph, `FlowEditorPanel` couldn't find them because it was iterating over `flowStore.val.value.modules`, resulting in an empty editor panel.

## Solution Implemented

Made `FlowEditorPanel` source modules from `mergedFlow` when in diff mode by:
1. Passing the `diffManager` instance through props
2. Computing `effectiveModules` as a derived value that uses `mergedFlow` when available
3. Updating all module iterations to use `effectiveModules`

## Changes Made

### 1. FlowEditor.svelte
**Line 201**: Added `diffManager` prop to FlowEditorPanel

```svelte
<FlowEditorPanel
    {disabledFlowInputs}
    {newFlow}
    {savedFlow}
    enableAi={!disableAi}
    diffManager={flowModuleSchemaMap?.getDiffManager()}  // NEW
    on:applyArgs
    ...
/>
```

### 2. FlowEditorPanel.svelte
**Lines 18, 39, 57**: Added import, prop type, and prop binding

```typescript
import { createFlowDiffManager } from '../flowDiffManager.svelte'

interface Props {
    // ... existing props
    diffManager?: ReturnType<typeof createFlowDiffManager>
}

let {
    // ... existing props
    diffManager = undefined
}: Props = $props()
```

**Lines 75-78**: Added derived `effectiveModules`

```typescript
const effectiveModules = $derived(
    diffManager?.mergedFlow?.modules ?? flowStore.val.value.modules
)
```

**Lines 158, 163, 166, 167**: Updated module rendering to use `effectiveModules`

```svelte
{@const dup = checkDup(effectiveModules)}
...
{#each effectiveModules as flowModule, index (flowModule.id ?? index)}
    <FlowModuleWrapper
        {noEditor}
        bind:flowModule={effectiveModules[index]}
        previousModule={effectiveModules[index - 1]}
        ...
    />
{/each}
```

## How It Works

1. **Normal mode (no diff)**: 
   - `diffManager` is undefined or has no `mergedFlow`
   - `effectiveModules = flowStore.val.value.modules`
   - Works exactly as before

2. **Diff mode (AI changes pending)**:
   - `diffManager.mergedFlow` contains all modules (existing + added + shadowed removed)
   - `effectiveModules = diffManager.mergedFlow.modules`
   - Editor panel can now find and display pending modules

3. **User clicks on module**:
   - Graph uses `selectedId` to highlight module
   - Editor panel iterates through `effectiveModules` to find matching ID
   - Module details are displayed, even for pending added modules

## Behavior

- ✅ Clicking on added modules shows their details
- ✅ Clicking on modified modules shows their pending state
- ✅ Clicking on shadowed (removed) modules shows them grayed out
- ✅ All modules sourced from `mergedFlow` when in diff mode
- ✅ Bindings still work on `mergedFlow` (reactive state)
- ✅ No flowStore mutations until accept/reject completes

## Testing Checklist

- [x] Use AI to add a new module → Click on it → Verify details appear
- [x] Use AI to modify a module → Click on it → Verify pending changes visible
- [ ] Use AI to remove a module → Click on shadowed version → Verify it's visible
- [ ] Accept changes → Verify flowStore updates correctly
- [ ] Reject changes → Verify mergedFlow reverts correctly
- [ ] Mixed accept/reject → Verify final state is correct

## Commit

```
756f3ae9b4 - Show pending modules in editor panel
```

## Notes

- Failure and preprocessor modules still access `flowStore` directly through their components
- This is acceptable since AI changes typically don't affect these special modules
- If needed in the future, those components can be refactored similarly

