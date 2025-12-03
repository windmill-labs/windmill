# Flow Diff Analysis and Test Coverage

## Overview

This document captures the analysis of `flowDiff.ts` functionality, edge cases, and the relationship between diff logic and accept/reject behavior.

## Current Test Coverage (63 tests)

### Implemented Tests
- ✅ Basic detection (identical, added, removed, modified, type change)
- ✅ Options (markAsPending, markRemovedAsShadowed)
- ✅ mergedFlow structure
- ✅ Edge cases (empty flows, undefined modules, multiple changes)
- ✅ Forloop operations (add/remove/modify inside, nested loops, entire loop removal)
- ✅ Whileloop operations
- ✅ Branchone operations (default branch, conditional branches, entire branch removal)
- ✅ Branchall operations (parallel branches, entire branchall removal)
- ✅ Deep nesting scenarios
- ✅ Special modules (failure_module, preprocessor_module)
- ✅ hasInputSchemaChanged function
- ✅ Multiple removed modules ordering
- ✅ Insert position tests (beginning/middle)
- ✅ Empty containers (empty loops, empty branches)
- ✅ Container type changes (forloop→whileloop, branchone→branchall)
- ✅ Module reordering

### Failing Tests (2)
These tests reveal a gap in the implementation:

1. **`detects module moved from root to inside a loop`**
   - Before: `[a, b, loop(empty)]`
   - After: `[a, loop(b)]`
   - Expected: `old__b` at root level in mergedFlow
   - Actual: `b` only appears inside the loop

2. **`detects module moved from loop to root`**
   - Before: `[b, loop(a)]`
   - After: `[b, loop(empty), a]`
   - Expected: `old__a` inside the loop in mergedFlow
   - Actual: loop is empty, `a` only at root

## The Module "Movement" Problem

### Key Insight
A module appearing at a different location with the same ID should NOT be treated as a "move". It should be treated as two independent operations:
- **Removal** from the old location
- **Addition** at the new location

The user may have simply removed a module and added a new one with the same ID elsewhere, without conceptualizing it as a "move".

### Current Implementation Gap

The current `computeFlowModuleDiff` function compares modules by ID across the entire flow:
```typescript
const beforeModules = getAllModulesMap(beforeFlow)  // Flat map of all modules by ID
const afterModules = getAllModulesMap(afterFlow)

// If module ID exists in both, it's "modified" or unchanged
// If only in before, it's "removed"
// If only in after, it's "added"
```

This approach loses **location information**. A module with the same ID at a different location is seen as "existing in both" (unchanged or modified), not as removed+added.

### What Should Happen

For the scenario `[a, b, loop(empty)]` → `[a, loop(b)]`:

1. **Diff detection should recognize:**
   - `b` at root level: **removed**
   - `b` inside loop: **added**

2. **mergedFlow should contain both:**
   - `old__b` at root (marked as removed)
   - `b` inside loop (marked as added)

3. **Accept/Reject should work independently:**
   - User can accept the removal at root → `b` disappears from root
   - User can accept the addition in loop → `b` stays in loop
   - User can reject the removal at root → `b` restored at root
   - User can reject the addition in loop → `b` removed from loop

4. **Conflict handling:**
   - If user accepts both removal and addition → valid, module "moved"
   - If user rejects both → valid, module stays at original location
   - If user accepts removal but rejects addition → module deleted entirely
   - If user rejects removal but accepts addition → duplicate! Must be handled

### Duplicate ID Handling

The tricky case: user rejects removal (keep at old location) AND accepts addition (keep at new location) = duplicate module ID.

Options:
1. **Prevent in UI** - Don't allow this combination
2. **Validation error** - Show error when user tries to apply
3. **Auto-resolve** - If same content, treat as user error and pick one

## Accept/Reject Logic Investigation

**TODO: We need to investigate the accept/reject implementation to understand:**
- How does it currently handle the actions from `buildFlowTimeline`?
- What happens when accepting/rejecting a "removed" module?
- What happens when accepting/rejecting an "added" module?
- How does it handle the `old__` prefixed modules?
- What validation exists for conflicts?

## Next Steps

1. **Explore the accept/reject logic** - Find where this is implemented and understand the current behavior
2. **Write tests for accept/reject** - Cover the scenarios described above
3. **Identify gaps in accept/reject** - Based on the module movement scenario
4. **Fix the diff implementation** - Make it location-aware for same-ID modules
5. **Update accept/reject if needed** - Handle the duplicate ID scenario

## Files to Investigate

- `flowDiff.ts` - Diff computation (already well understood)
- Accept/reject implementation - **TO BE FOUND**
- Flow editor components that use the diff - **TO BE FOUND**
