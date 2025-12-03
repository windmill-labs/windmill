import { describe, it, expect } from 'vitest'
import { buildFlowTimeline } from './flowDiff'
import type {
	FlowValue,
	FlowModule,
	RawScript,
	Identity,
	ForloopFlow,
	WhileloopFlow,
	BranchOne,
	BranchAll
} from '$lib/gen'

// Helper to create a minimal RawScript module
function createRawScriptModule(id: string, content: string): FlowModule {
	return {
		id,
		value: {
			type: 'rawscript',
			content,
			language: 'bun',
			input_transforms: {}
		} as RawScript
	}
}

// Helper to create a minimal Identity module (for type-change tests)
function createIdentityModule(id: string): FlowModule {
	return {
		id,
		value: {
			type: 'identity'
		} as Identity
	}
}

// Helper to create a ForloopFlow module with nested modules
function createForloopModule(id: string, nestedModules: FlowModule[]): FlowModule {
	return {
		id,
		value: {
			type: 'forloopflow',
			modules: nestedModules,
			iterator: { type: 'javascript', expr: '[1,2,3]' },
			skip_failures: false
		} as ForloopFlow
	}
}

// Helper to create a WhileloopFlow module with nested modules
function createWhileloopModule(id: string, nestedModules: FlowModule[]): FlowModule {
	return {
		id,
		value: {
			type: 'whileloopflow',
			modules: nestedModules,
			skip_failures: false
		} as WhileloopFlow
	}
}

// Helper to create a BranchOne module with default and conditional branches
function createBranchOneModule(
	id: string,
	defaultModules: FlowModule[],
	branches: { expr: string; modules: FlowModule[] }[]
): FlowModule {
	return {
		id,
		value: {
			type: 'branchone',
			default: defaultModules,
			branches: branches.map((b) => ({ expr: b.expr, modules: b.modules }))
		} as BranchOne
	}
}

// Helper to create a BranchAll module with parallel branches
function createBranchAllModule(id: string, branches: { modules: FlowModule[] }[]): FlowModule {
	return {
		id,
		value: {
			type: 'branchall',
			branches: branches.map((b) => ({ modules: b.modules }))
		} as BranchAll
	}
}

// Helper to create a FlowValue with modules
function createFlow(modules: FlowModule[]): FlowValue {
	return { modules }
}

/**
 * Asserts that modules appear in the exact order specified.
 * Also implicitly verifies the count of modules.
 */
function expectModuleOrder(modules: FlowModule[], expectedIds: string[]) {
	expect(modules.map((m) => m.id)).toEqual(expectedIds)
}

describe('buildFlowTimeline', () => {
	describe('basic detection', () => {
		it('returns empty actions for identical flows', () => {
			const moduleA = createRawScriptModule('a', 'console.log("hello")')
			const beforeFlow = createFlow([moduleA])
			const afterFlow = createFlow([moduleA])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(Object.keys(result.beforeActions)).toHaveLength(0)
			expect(Object.keys(result.afterActions)).toHaveLength(0)
		})

		it('detects added module', () => {
			const moduleA = createRawScriptModule('a', 'console.log("a")')
			const moduleB = createRawScriptModule('b', 'console.log("b")')
			const beforeFlow = createFlow([moduleA])
			const afterFlow = createFlow([moduleA, moduleB])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.beforeActions).toEqual({})
			expect(result.afterActions).toEqual({
				b: { action: 'added', pending: false }
			})
		})

		it('detects removed module', () => {
			const moduleA = createRawScriptModule('a', 'console.log("a")')
			const moduleB = createRawScriptModule('b', 'console.log("b")')
			const beforeFlow = createFlow([moduleA, moduleB])
			const afterFlow = createFlow([moduleA])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.beforeActions).toEqual({
				b: { action: 'removed', pending: false }
			})
			// By default markRemovedAsShadowed is false, so removed modules show as 'removed' in afterActions
			expect(result.afterActions).toEqual({
				b: { action: 'removed', pending: false }
			})
		})

		it('detects modified module (same type, different content)', () => {
			const moduleBeforeA = createRawScriptModule('a', 'console.log("before")')
			const moduleAfterA = createRawScriptModule('a', 'console.log("after")')
			const beforeFlow = createFlow([moduleBeforeA])
			const afterFlow = createFlow([moduleAfterA])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.beforeActions).toEqual({
				a: { action: 'modified', pending: false }
			})
			expect(result.afterActions).toEqual({
				a: { action: 'modified', pending: false }
			})
		})

		it('treats type change as removed + added (not modified)', () => {
			const moduleBeforeA = createRawScriptModule('a', 'console.log("script")')
			const moduleAfterA = createIdentityModule('a')
			const beforeFlow = createFlow([moduleBeforeA])
			const afterFlow = createFlow([moduleAfterA])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.beforeActions).toEqual({
				a: { action: 'removed', pending: false }
			})
			// The new module is 'added', and the old module is kept in mergedFlow
			// with a prefixed ID 'old__a' and marked as 'removed'
			expect(result.afterActions['a']).toEqual({ action: 'added', pending: false })
			expect(result.afterActions['old__a']).toEqual({ action: 'removed', pending: false })
		})
	})

	describe('options', () => {
		it('markAsPending sets pending: true on all actions', () => {
			const moduleA = createRawScriptModule('a', 'console.log("a")')
			const moduleB = createRawScriptModule('b', 'console.log("b")')
			const beforeFlow = createFlow([moduleA])
			const afterFlow = createFlow([moduleB])

			const result = buildFlowTimeline(beforeFlow, afterFlow, {
				markAsPending: true,
				markRemovedAsShadowed: false
			})

			expect(result.beforeActions).toEqual({
				a: { action: 'removed', pending: true }
			})
			expect(result.afterActions).toEqual({
				a: { action: 'removed', pending: true },
				b: { action: 'added', pending: true }
			})
		})

		it('markRemovedAsShadowed: true shows removed as shadowed in afterActions', () => {
			const moduleA = createRawScriptModule('a', 'console.log("a")')
			const moduleB = createRawScriptModule('b', 'console.log("b")')
			const beforeFlow = createFlow([moduleA, moduleB])
			const afterFlow = createFlow([moduleA])

			const result = buildFlowTimeline(beforeFlow, afterFlow, {
				markAsPending: false,
				markRemovedAsShadowed: true
			})

			expect(result.beforeActions).toEqual({
				b: { action: 'removed', pending: false }
			})
			expect(result.afterActions).toEqual({
				b: { action: 'shadowed', pending: false }
			})
		})

		it('markRemovedAsShadowed: false shows removed as removed in afterActions', () => {
			const moduleA = createRawScriptModule('a', 'console.log("a")')
			const moduleB = createRawScriptModule('b', 'console.log("b")')
			const beforeFlow = createFlow([moduleA, moduleB])
			const afterFlow = createFlow([moduleA])

			const result = buildFlowTimeline(beforeFlow, afterFlow, {
				markAsPending: false,
				markRemovedAsShadowed: false
			})

			expect(result.beforeActions).toEqual({
				b: { action: 'removed', pending: false }
			})
			expect(result.afterActions).toEqual({
				b: { action: 'removed', pending: false }
			})
		})
	})

	describe('mergedFlow', () => {
		it('mergedFlow contains all modules from afterFlow', () => {
			const moduleA = createRawScriptModule('a', 'console.log("a")')
			const moduleB = createRawScriptModule('b', 'console.log("b")')
			const beforeFlow = createFlow([moduleA])
			const afterFlow = createFlow([moduleA, moduleB])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expectModuleOrder(result.mergedFlow.modules ?? [], ['a', 'b'])
		})

		it('mergedFlow includes removed modules from beforeFlow', () => {
			const moduleA = createRawScriptModule('a', 'console.log("a")')
			const moduleB = createRawScriptModule('b', 'console.log("b")')
			const beforeFlow = createFlow([moduleA, moduleB])
			const afterFlow = createFlow([moduleA])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expectModuleOrder(result.mergedFlow.modules ?? [], ['a', 'b'])
		})
	})

	describe('edge cases', () => {
		it('handles empty flows', () => {
			const beforeFlow = createFlow([])
			const afterFlow = createFlow([])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(Object.keys(result.beforeActions)).toHaveLength(0)
			expect(Object.keys(result.afterActions)).toHaveLength(0)
			expect(result.mergedFlow.modules).toHaveLength(0)
		})

		it('handles flow with undefined modules', () => {
			const beforeFlow = {}
			const afterFlow = {}

			const result = buildFlowTimeline(beforeFlow as FlowValue, afterFlow as FlowValue)

			expect(Object.keys(result.beforeActions)).toHaveLength(0)
			expect(Object.keys(result.afterActions)).toHaveLength(0)
		})

		it('handles multiple changes at once', () => {
			const moduleA = createRawScriptModule('a', 'original')
			const moduleB = createRawScriptModule('b', 'to be removed')
			const moduleAModified = createRawScriptModule('a', 'modified')
			const moduleC = createRawScriptModule('c', 'newly added')

			const beforeFlow = createFlow([moduleA, moduleB])
			const afterFlow = createFlow([moduleAModified, moduleC])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// Module 'a' was modified
			expect(result.beforeActions['a']).toEqual({ action: 'modified', pending: false })
			expect(result.afterActions['a']).toEqual({ action: 'modified', pending: false })

			// Module 'b' was removed
			expect(result.beforeActions['b']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['b']).toEqual({ action: 'removed', pending: false })

			// Module 'c' was added
			expect(result.afterActions['c']).toEqual({ action: 'added', pending: false })
		})
	})

	describe('forloop operations', () => {
		it('detects added module inside forloop', () => {
			const innerA = createRawScriptModule('inner_a', 'step a')
			const innerB = createRawScriptModule('inner_b', 'step b')

			const beforeLoop = createForloopModule('loop1', [innerA])
			const afterLoop = createForloopModule('loop1', [innerA, innerB])

			const beforeFlow = createFlow([beforeLoop])
			const afterFlow = createFlow([afterLoop])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.afterActions['inner_b']).toEqual({ action: 'added', pending: false })
			expect(result.beforeActions['inner_b']).toBeUndefined()

			// Verify mergedFlow contains the loop with both modules in correct order
			expectModuleOrder(result.mergedFlow.modules ?? [], ['loop1'])
			const mergedLoop = result.mergedFlow.modules?.find((m) => m.id === 'loop1')
			expect(mergedLoop?.value.type).toBe('forloopflow')
			const loopModules = (mergedLoop?.value as ForloopFlow).modules
			expectModuleOrder(loopModules, ['inner_a', 'inner_b'])
		})

		it('detects removed module from inside forloop', () => {
			const innerA = createRawScriptModule('inner_a', 'step a')
			const innerB = createRawScriptModule('inner_b', 'step b')

			const beforeLoop = createForloopModule('loop1', [innerA, innerB])
			const afterLoop = createForloopModule('loop1', [innerA])

			const beforeFlow = createFlow([beforeLoop])
			const afterFlow = createFlow([afterLoop])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.beforeActions['inner_b']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['inner_b']).toEqual({ action: 'removed', pending: false })
			// Verify removed module is in mergedFlow inside the loop at correct position
			const mergedLoop = result.mergedFlow.modules?.find((m) => m.id === 'loop1')
			expect(mergedLoop?.value.type).toBe('forloopflow')
			const loopModules = (mergedLoop?.value as ForloopFlow).modules
			expectModuleOrder(loopModules, ['inner_a', 'inner_b'])
		})

		it('detects modified module inside forloop', () => {
			const innerBefore = createRawScriptModule('inner_a', 'original code')
			const innerAfter = createRawScriptModule('inner_a', 'modified code')

			const beforeLoop = createForloopModule('loop1', [innerBefore])
			const afterLoop = createForloopModule('loop1', [innerAfter])

			const beforeFlow = createFlow([beforeLoop])
			const afterFlow = createFlow([afterLoop])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.beforeActions['inner_a']).toEqual({ action: 'modified', pending: false })
			expect(result.afterActions['inner_a']).toEqual({ action: 'modified', pending: false })

			// Verify mergedFlow contains the loop with the modified module
			expectModuleOrder(result.mergedFlow.modules ?? [], ['loop1'])
			const mergedLoop = result.mergedFlow.modules?.find((m) => m.id === 'loop1')
			const loopModules = (mergedLoop?.value as ForloopFlow).modules
			expectModuleOrder(loopModules, ['inner_a'])
			// Verify the modified content is present (from afterFlow)
			const innerModule = loopModules.find((m) => m.id === 'inner_a')
			expect((innerModule?.value as RawScript).content).toBe('modified code')
		})

		it('detects removal of entire forloop with nested children', () => {
			const innerA = createRawScriptModule('inner_a', 'step a')
			const innerB = createRawScriptModule('inner_b', 'step b')
			const loop = createForloopModule('loop1', [innerA, innerB])
			const moduleC = createRawScriptModule('c', 'step c')

			const beforeFlow = createFlow([loop, moduleC])
			const afterFlow = createFlow([moduleC])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// The loop itself and all nested modules should be marked as removed
			expect(result.beforeActions['loop1']).toEqual({ action: 'removed', pending: false })
			expect(result.beforeActions['inner_a']).toEqual({ action: 'removed', pending: false })
			expect(result.beforeActions['inner_b']).toEqual({ action: 'removed', pending: false })

			// Verify mergedFlow contains the removed loop with all nested children at correct positions
			expectModuleOrder(result.mergedFlow.modules ?? [], ['loop1', 'c'])
			const mergedLoop = result.mergedFlow.modules?.find((m) => m.id === 'loop1')
			expect(mergedLoop?.value.type).toBe('forloopflow')
			const loopModules = (mergedLoop?.value as ForloopFlow).modules
			expectModuleOrder(loopModules, ['inner_a', 'inner_b'])
		})

		it('handles nested forloop inside forloop with changes at inner level', () => {
			const deepInnerA = createRawScriptModule('deep_a', 'deep step')
			const deepInnerB = createRawScriptModule('deep_b', 'new deep step')

			const innerLoopBefore = createForloopModule('inner_loop', [deepInnerA])
			const innerLoopAfter = createForloopModule('inner_loop', [deepInnerA, deepInnerB])

			const outerLoopBefore = createForloopModule('outer_loop', [innerLoopBefore])
			const outerLoopAfter = createForloopModule('outer_loop', [innerLoopAfter])

			const beforeFlow = createFlow([outerLoopBefore])
			const afterFlow = createFlow([outerLoopAfter])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// The new nested module is detected as added
			expect(result.afterActions['deep_b']).toEqual({ action: 'added', pending: false })
			// Note: Container modules are marked as 'modified' because their nested content changed
			// (deepEqual comparison includes nested modules array)
			expect(result.beforeActions['outer_loop']).toEqual({ action: 'modified', pending: false })
			expect(result.beforeActions['inner_loop']).toEqual({ action: 'modified', pending: false })

			// Verify mergedFlow preserves the outer_loop → inner_loop → [deep_a, deep_b] structure
			expectModuleOrder(result.mergedFlow.modules ?? [], ['outer_loop'])
			const mergedOuterLoop = result.mergedFlow.modules?.find((m) => m.id === 'outer_loop')
			expect(mergedOuterLoop?.value.type).toBe('forloopflow')

			const outerLoopModules = (mergedOuterLoop?.value as ForloopFlow).modules
			expectModuleOrder(outerLoopModules, ['inner_loop'])
			const mergedInnerLoop = outerLoopModules.find((m) => m.id === 'inner_loop')
			expect(mergedInnerLoop?.value.type).toBe('forloopflow')

			const innerLoopModules = (mergedInnerLoop?.value as ForloopFlow).modules
			expectModuleOrder(innerLoopModules, ['deep_a', 'deep_b'])
		})
	})

	describe('whileloop operations', () => {
		it('detects changes inside whileloop', () => {
			const innerA = createRawScriptModule('while_inner_a', 'step a')
			const innerB = createRawScriptModule('while_inner_b', 'step b')

			const beforeLoop = createWhileloopModule('while1', [innerA])
			const afterLoop = createWhileloopModule('while1', [innerA, innerB])

			const beforeFlow = createFlow([beforeLoop])
			const afterFlow = createFlow([afterLoop])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.afterActions['while_inner_b']).toEqual({ action: 'added', pending: false })

			// Verify mergedFlow contains the whileloop with both modules in correct order
			expectModuleOrder(result.mergedFlow.modules ?? [], ['while1'])
			const mergedLoop = result.mergedFlow.modules?.find((m) => m.id === 'while1')
			expect(mergedLoop?.value.type).toBe('whileloopflow')
			const loopModules = (mergedLoop?.value as WhileloopFlow).modules
			expectModuleOrder(loopModules, ['while_inner_a', 'while_inner_b'])
		})

		it('handles removal of module from whileloop and restores in mergedFlow', () => {
			const innerA = createRawScriptModule('while_a', 'step a')
			const innerB = createRawScriptModule('while_b', 'step b')

			const beforeLoop = createWhileloopModule('while1', [innerA, innerB])
			const afterLoop = createWhileloopModule('while1', [innerA])

			const beforeFlow = createFlow([beforeLoop])
			const afterFlow = createFlow([afterLoop])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.beforeActions['while_b']).toEqual({ action: 'removed', pending: false })
			// Verify mergedFlow contains the removed module at correct position
			const mergedLoop = result.mergedFlow.modules?.find((m) => m.id === 'while1')
			const loopModules = (mergedLoop?.value as WhileloopFlow).modules
			expectModuleOrder(loopModules, ['while_a', 'while_b'])
		})
	})

	describe('branchone operations', () => {
		it('detects added module in default branch', () => {
			const defaultA = createRawScriptModule('default_a', 'default step a')
			const defaultB = createRawScriptModule('default_b', 'default step b')

			const beforeBranch = createBranchOneModule('branch1', [defaultA], [])
			const afterBranch = createBranchOneModule('branch1', [defaultA, defaultB], [])

			const beforeFlow = createFlow([beforeBranch])
			const afterFlow = createFlow([afterBranch])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.afterActions['default_b']).toEqual({ action: 'added', pending: false })

			// Verify mergedFlow.branch1.default contains both modules in correct order
			expectModuleOrder(result.mergedFlow.modules ?? [], ['branch1'])
			const mergedBranch = result.mergedFlow.modules?.find((m) => m.id === 'branch1')
			expect(mergedBranch?.value.type).toBe('branchone')
			const defaultModules = (mergedBranch?.value as BranchOne).default
			expectModuleOrder(defaultModules, ['default_a', 'default_b'])
		})

		it('detects removed module from conditional branch', () => {
			const branchModuleA = createRawScriptModule('branch_a', 'branch step a')
			const branchModuleB = createRawScriptModule('branch_b', 'branch step b')

			const beforeBranch = createBranchOneModule('branch1', [], [
				{ expr: 'true', modules: [branchModuleA, branchModuleB] }
			])
			const afterBranch = createBranchOneModule('branch1', [], [
				{ expr: 'true', modules: [branchModuleA] }
			])

			const beforeFlow = createFlow([beforeBranch])
			const afterFlow = createFlow([afterBranch])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.beforeActions['branch_b']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['branch_b']).toEqual({ action: 'removed', pending: false })

			// Verify mergedFlow.branch1.branches[0].modules contains the removed branch_b at correct position
			const mergedBranch = result.mergedFlow.modules?.find((m) => m.id === 'branch1')
			const branches = (mergedBranch?.value as BranchOne).branches
			expect(branches).toHaveLength(1)
			expectModuleOrder(branches[0].modules, ['branch_a', 'branch_b'])
		})

		it('detects removal of entire conditional branch', () => {
			const defaultModule = createRawScriptModule('default_mod', 'default')
			const branch1ModuleA = createRawScriptModule('b1_a', 'branch 1 a')
			const branch2ModuleA = createRawScriptModule('b2_a', 'branch 2 a')
			const branch2ModuleB = createRawScriptModule('b2_b', 'branch 2 b')

			const beforeBranch = createBranchOneModule('branch1', [defaultModule], [
				{ expr: 'x > 0', modules: [branch1ModuleA] },
				{ expr: 'x < 0', modules: [branch2ModuleA, branch2ModuleB] } // entire branch removed
			])
			const afterBranch = createBranchOneModule('branch1', [defaultModule], [
				{ expr: 'x > 0', modules: [branch1ModuleA] }
			])

			const beforeFlow = createFlow([beforeBranch])
			const afterFlow = createFlow([afterBranch])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// All modules from removed branch should be marked as removed
			expect(result.beforeActions['b2_a']).toEqual({ action: 'removed', pending: false })
			expect(result.beforeActions['b2_b']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['b2_a']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['b2_b']).toEqual({ action: 'removed', pending: false })

			// Verify mergedFlow structure - should have 2 branches with removed modules restored
			expectModuleOrder(result.mergedFlow.modules ?? [], ['branch1'])
			const mergedBranch = result.mergedFlow.modules?.find((m) => m.id === 'branch1')
			const branchValue = mergedBranch?.value as BranchOne

			// Default branch unchanged
			expectModuleOrder(branchValue.default, ['default_mod'])

			// First branch unchanged
			expectModuleOrder(branchValue.branches[0].modules, ['b1_a'])

			// Second branch should be restored with its modules
			expect(branchValue.branches).toHaveLength(2)
			expectModuleOrder(branchValue.branches[1].modules, ['b2_a', 'b2_b'])
		})

		it('detects changes across multiple branches simultaneously', () => {
			const defaultModule = createRawScriptModule('default_mod', 'default')
			const branch1ModuleA = createRawScriptModule('b1_a', 'branch 1 a')
			const branch1ModuleB = createRawScriptModule('b1_b', 'branch 1 b - new')
			const branch2ModuleA = createRawScriptModule('b2_a', 'branch 2 a - original')
			const branch2ModuleAModified = createRawScriptModule('b2_a', 'branch 2 a - modified')

			const beforeBranch = createBranchOneModule('branch1', [defaultModule], [
				{ expr: 'x > 0', modules: [branch1ModuleA] },
				{ expr: 'x < 0', modules: [branch2ModuleA] }
			])
			const afterBranch = createBranchOneModule('branch1', [defaultModule], [
				{ expr: 'x > 0', modules: [branch1ModuleA, branch1ModuleB] },
				{ expr: 'x < 0', modules: [branch2ModuleAModified] }
			])

			const beforeFlow = createFlow([beforeBranch])
			const afterFlow = createFlow([afterBranch])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// branch1ModuleB was added
			expect(result.afterActions['b1_b']).toEqual({ action: 'added', pending: false })
			// branch2ModuleA was modified
			expect(result.beforeActions['b2_a']).toEqual({ action: 'modified', pending: false })
			expect(result.afterActions['b2_a']).toEqual({ action: 'modified', pending: false })

			// Verify mergedFlow preserves structure with correct ordering
			expectModuleOrder(result.mergedFlow.modules ?? [], ['branch1'])
			const mergedBranch = result.mergedFlow.modules?.find((m) => m.id === 'branch1')
			const branchValue = mergedBranch?.value as BranchOne

			// Check default branch
			expectModuleOrder(branchValue.default, ['default_mod'])

			// Check first conditional branch (should have b1_a and b1_b in order)
			expectModuleOrder(branchValue.branches[0].modules, ['b1_a', 'b1_b'])

			// Check second conditional branch (should have modified b2_a)
			expectModuleOrder(branchValue.branches[1].modules, ['b2_a'])
			const b2aModule = branchValue.branches[1].modules.find((m) => m.id === 'b2_a')
			expect((b2aModule?.value as RawScript).content).toBe('branch 2 a - modified')
		})

		it('handles nested branch inside loop with changes', () => {
			const branchInnerA = createRawScriptModule('nested_branch_a', 'nested a')
			const branchInnerB = createRawScriptModule('nested_branch_b', 'nested b')

			const beforeBranch = createBranchOneModule('inner_branch', [branchInnerA], [])
			const afterBranch = createBranchOneModule('inner_branch', [branchInnerA, branchInnerB], [])

			const beforeLoop = createForloopModule('outer_loop', [beforeBranch])
			const afterLoop = createForloopModule('outer_loop', [afterBranch])

			const beforeFlow = createFlow([beforeLoop])
			const afterFlow = createFlow([afterLoop])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.afterActions['nested_branch_b']).toEqual({ action: 'added', pending: false })

			// Verify mergedFlow.outer_loop.inner_branch.default contains added nested_branch_b at correct position
			expectModuleOrder(result.mergedFlow.modules ?? [], ['outer_loop'])
			const mergedLoop = result.mergedFlow.modules?.find((m) => m.id === 'outer_loop')
			const loopModules = (mergedLoop?.value as ForloopFlow).modules
			expectModuleOrder(loopModules, ['inner_branch'])
			const mergedBranch = loopModules.find((m) => m.id === 'inner_branch')
			const branchDefault = (mergedBranch?.value as BranchOne).default
			expectModuleOrder(branchDefault, ['nested_branch_a', 'nested_branch_b'])
		})
	})

	describe('branchall operations', () => {
		it('detects changes in parallel branches', () => {
			const parallel1A = createRawScriptModule('p1_a', 'parallel 1 a')
			const parallel2A = createRawScriptModule('p2_a', 'parallel 2 a')
			const parallel2B = createRawScriptModule('p2_b', 'parallel 2 b - new')

			const beforeBranchAll = createBranchAllModule('branchall1', [
				{ modules: [parallel1A] },
				{ modules: [parallel2A] }
			])
			const afterBranchAll = createBranchAllModule('branchall1', [
				{ modules: [parallel1A] },
				{ modules: [parallel2A, parallel2B] }
			])

			const beforeFlow = createFlow([beforeBranchAll])
			const afterFlow = createFlow([afterBranchAll])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.afterActions['p2_b']).toEqual({ action: 'added', pending: false })

			// Verify mergedFlow.branchall1.branches have correct module order
			expectModuleOrder(result.mergedFlow.modules ?? [], ['branchall1'])
			const mergedBranchAll = result.mergedFlow.modules?.find((m) => m.id === 'branchall1')
			expect(mergedBranchAll?.value.type).toBe('branchall')
			const branches = (mergedBranchAll?.value as BranchAll).branches
			expect(branches).toHaveLength(2)
			// First branch unchanged
			expectModuleOrder(branches[0].modules, ['p1_a'])
			// Second branch has added module in correct order
			expectModuleOrder(branches[1].modules, ['p2_a', 'p2_b'])
		})

		it('detects removal of entire branchall with all nested modules', () => {
			const parallel1A = createRawScriptModule('p1_a', 'parallel 1')
			const parallel2A = createRawScriptModule('p2_a', 'parallel 2')
			const branchAll = createBranchAllModule('branchall1', [
				{ modules: [parallel1A] },
				{ modules: [parallel2A] }
			])
			const moduleC = createRawScriptModule('c', 'step c')

			const beforeFlow = createFlow([branchAll, moduleC])
			const afterFlow = createFlow([moduleC])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.beforeActions['branchall1']).toEqual({ action: 'removed', pending: false })
			expect(result.beforeActions['p1_a']).toEqual({ action: 'removed', pending: false })
			expect(result.beforeActions['p2_a']).toEqual({ action: 'removed', pending: false })

			// Verify mergedFlow contains the removed branchall with correct structure
			expectModuleOrder(result.mergedFlow.modules ?? [], ['branchall1', 'c'])
			const mergedBranchAll = result.mergedFlow.modules?.find((m) => m.id === 'branchall1')
			expect(mergedBranchAll?.value.type).toBe('branchall')
			const branches = (mergedBranchAll?.value as BranchAll).branches
			expect(branches).toHaveLength(2)
			expectModuleOrder(branches[0].modules, ['p1_a'])
			expectModuleOrder(branches[1].modules, ['p2_a'])
		})

		it('handles removal from one parallel branch while adding to another', () => {
			const parallel1A = createRawScriptModule('p1_a', 'parallel 1 a')
			const parallel1B = createRawScriptModule('p1_b', 'parallel 1 b - to remove')
			const parallel2A = createRawScriptModule('p2_a', 'parallel 2 a')
			const parallel2B = createRawScriptModule('p2_b', 'parallel 2 b - new')

			const beforeBranchAll = createBranchAllModule('branchall1', [
				{ modules: [parallel1A, parallel1B] },
				{ modules: [parallel2A] }
			])
			const afterBranchAll = createBranchAllModule('branchall1', [
				{ modules: [parallel1A] },
				{ modules: [parallel2A, parallel2B] }
			])

			const beforeFlow = createFlow([beforeBranchAll])
			const afterFlow = createFlow([afterBranchAll])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.beforeActions['p1_b']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['p2_b']).toEqual({ action: 'added', pending: false })

			// Verify mergedFlow.branchall1 has correct structure with all modules in order
			expectModuleOrder(result.mergedFlow.modules ?? [], ['branchall1'])
			const mergedBranchAll = result.mergedFlow.modules?.find((m) => m.id === 'branchall1')
			const branches = (mergedBranchAll?.value as BranchAll).branches
			expect(branches).toHaveLength(2)

			// First branch should have both p1_a and removed p1_b restored in order
			expectModuleOrder(branches[0].modules, ['p1_a', 'p1_b'])

			// Second branch should have p2_a and added p2_b in order
			expectModuleOrder(branches[1].modules, ['p2_a', 'p2_b'])
		})
	})

	describe('deep nesting', () => {
		it('handles loop inside branch inside loop with modifications at each level', () => {
			// Structure: outerLoop -> branch -> innerLoop -> scripts
			const deepScript1 = createRawScriptModule('deep1', 'deep script 1')
			const deepScript2 = createRawScriptModule('deep2', 'deep script 2')
			const deepScript3 = createRawScriptModule('deep3', 'deep script 3 - new')

			const innerLoopBefore = createForloopModule('inner_loop', [deepScript1, deepScript2])
			const innerLoopAfter = createForloopModule('inner_loop', [deepScript1, deepScript3])

			const branchBefore = createBranchOneModule('mid_branch', [innerLoopBefore], [])
			const branchAfter = createBranchOneModule('mid_branch', [innerLoopAfter], [])

			const outerLoopBefore = createForloopModule('outer_loop', [branchBefore])
			const outerLoopAfter = createForloopModule('outer_loop', [branchAfter])

			const beforeFlow = createFlow([outerLoopBefore])
			const afterFlow = createFlow([outerLoopAfter])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// deep2 was removed
			expect(result.beforeActions['deep2']).toEqual({ action: 'removed', pending: false })
			// deep3 was added
			expect(result.afterActions['deep3']).toEqual({ action: 'added', pending: false })
			// Note: Container modules are marked as 'modified' because their nested content changed
			// (deepEqual comparison includes nested modules array)
			expect(result.beforeActions['outer_loop']).toEqual({ action: 'modified', pending: false })
			expect(result.beforeActions['mid_branch']).toEqual({ action: 'modified', pending: false })
			expect(result.beforeActions['inner_loop']).toEqual({ action: 'modified', pending: false })

			// Verify full structure with correct ordering at each level
			expectModuleOrder(result.mergedFlow.modules ?? [], ['outer_loop'])
			const mergedOuterLoop = result.mergedFlow.modules?.find((m) => m.id === 'outer_loop')
			expect(mergedOuterLoop?.value.type).toBe('forloopflow')

			const outerLoopModules = (mergedOuterLoop?.value as ForloopFlow).modules
			expectModuleOrder(outerLoopModules, ['mid_branch'])
			const mergedMidBranch = outerLoopModules.find((m) => m.id === 'mid_branch')
			expect(mergedMidBranch?.value.type).toBe('branchone')

			const midBranchDefault = (mergedMidBranch?.value as BranchOne).default
			expectModuleOrder(midBranchDefault, ['inner_loop'])
			const mergedInnerLoop = midBranchDefault.find((m) => m.id === 'inner_loop')
			expect(mergedInnerLoop?.value.type).toBe('forloopflow')

			const innerLoopModules = (mergedInnerLoop?.value as ForloopFlow).modules
			// deep1 is unchanged, deep2 was removed (but restored in mergedFlow), deep3 was added
			expectModuleOrder(innerLoopModules, ['deep1', 'deep2', 'deep3'])
		})

		it('handles complex scenario with multiple nested structures and simultaneous changes', () => {
			// Before: loop1 -> [scriptA, branch1 -> [scriptB, scriptC]]
			// After:  loop1 -> [scriptA_modified, branch1 -> [scriptB, scriptD]]
			const scriptA = createRawScriptModule('a', 'script a original')
			const scriptAModified = createRawScriptModule('a', 'script a modified')
			const scriptB = createRawScriptModule('b', 'script b')
			const scriptC = createRawScriptModule('c', 'script c - to remove')
			const scriptD = createRawScriptModule('d', 'script d - new')

			const branchBefore = createBranchOneModule('branch1', [scriptB, scriptC], [])
			const branchAfter = createBranchOneModule('branch1', [scriptB, scriptD], [])

			const loopBefore = createForloopModule('loop1', [scriptA, branchBefore])
			const loopAfter = createForloopModule('loop1', [scriptAModified, branchAfter])

			const beforeFlow = createFlow([loopBefore])
			const afterFlow = createFlow([loopAfter])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// scriptA was modified
			expect(result.beforeActions['a']).toEqual({ action: 'modified', pending: false })
			expect(result.afterActions['a']).toEqual({ action: 'modified', pending: false })
			// scriptC was removed
			expect(result.beforeActions['c']).toEqual({ action: 'removed', pending: false })
			// scriptD was added
			expect(result.afterActions['d']).toEqual({ action: 'added', pending: false })
			// scriptB unchanged
			expect(result.beforeActions['b']).toBeUndefined()
			expect(result.afterActions['b']).toBeUndefined()

			// Verify mergedFlow structure with correct ordering
			expectModuleOrder(result.mergedFlow.modules ?? [], ['loop1'])
			const mergedLoop = result.mergedFlow.modules?.find((m) => m.id === 'loop1')
			const loopModules = (mergedLoop?.value as ForloopFlow).modules
			expectModuleOrder(loopModules, ['a', 'branch1'])

			// Modified scriptA should be in mergedFlow with updated content
			const mergedA = loopModules.find((m) => m.id === 'a')
			expect((mergedA?.value as RawScript).content).toBe('script a modified')

			// Branch1 should contain scriptB, removed c, and added d in correct order
			const mergedBranch = loopModules.find((m) => m.id === 'branch1')
			const branchDefault = (mergedBranch?.value as BranchOne).default
			expectModuleOrder(branchDefault, ['b', 'c', 'd'])
		})

		it('preserves correct structure in mergedFlow for deeply nested removals', () => {
			// Remove a script from inside a loop that is inside a branch
			const deepScript = createRawScriptModule('deep', 'deep script to remove')
			const keepScript = createRawScriptModule('keep', 'script to keep')

			const innerLoopBefore = createForloopModule('inner_loop', [deepScript, keepScript])
			const innerLoopAfter = createForloopModule('inner_loop', [keepScript])

			const branchBefore = createBranchOneModule('branch', [innerLoopBefore], [])
			const branchAfter = createBranchOneModule('branch', [innerLoopAfter], [])

			const beforeFlow = createFlow([branchBefore])
			const afterFlow = createFlow([branchAfter])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// Navigate into the merged flow structure and verify correct ordering
			expectModuleOrder(result.mergedFlow.modules ?? [], ['branch'])
			const mergedBranch = result.mergedFlow.modules?.find((m) => m.id === 'branch')

			const branchDefault = (mergedBranch?.value as BranchOne).default
			expectModuleOrder(branchDefault, ['inner_loop'])
			const mergedInnerLoop = branchDefault.find((m) => m.id === 'inner_loop')

			const innerLoopModules = (mergedInnerLoop?.value as ForloopFlow).modules
			// Both modules should be present in the merged flow in correct order
			expectModuleOrder(innerLoopModules, ['deep', 'keep'])
		})

		it('handles branchall inside branchone with nested changes', () => {
			const scriptInParallel1 = createRawScriptModule('par1', 'parallel 1')
			const scriptInParallel2 = createRawScriptModule('par2', 'parallel 2')
			const scriptInParallel3 = createRawScriptModule('par3', 'parallel 3 - new')

			const branchAllBefore = createBranchAllModule('parallel_section', [
				{ modules: [scriptInParallel1] },
				{ modules: [scriptInParallel2] }
			])
			const branchAllAfter = createBranchAllModule('parallel_section', [
				{ modules: [scriptInParallel1] },
				{ modules: [scriptInParallel2, scriptInParallel3] }
			])

			const branchOneBefore = createBranchOneModule('outer_branch', [branchAllBefore], [
				{ expr: 'x > 0', modules: [] }
			])
			const branchOneAfter = createBranchOneModule('outer_branch', [branchAllAfter], [
				{ expr: 'x > 0', modules: [] }
			])

			const beforeFlow = createFlow([branchOneBefore])
			const afterFlow = createFlow([branchOneAfter])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.afterActions['par3']).toEqual({ action: 'added', pending: false })
			// Other modules unchanged
			expect(result.beforeActions['par1']).toBeUndefined()
			expect(result.beforeActions['par2']).toBeUndefined()

			// Verify full nested structure is preserved in mergedFlow with correct ordering
			expectModuleOrder(result.mergedFlow.modules ?? [], ['outer_branch'])
			const mergedBranchOne = result.mergedFlow.modules?.find((m) => m.id === 'outer_branch')
			expect(mergedBranchOne?.value.type).toBe('branchone')

			const branchOneDefault = (mergedBranchOne?.value as BranchOne).default
			expectModuleOrder(branchOneDefault, ['parallel_section'])
			const mergedBranchAll = branchOneDefault.find((m) => m.id === 'parallel_section')
			expect(mergedBranchAll?.value.type).toBe('branchall')

			const parallelBranches = (mergedBranchAll?.value as BranchAll).branches
			expect(parallelBranches).toHaveLength(2)
			// First parallel branch has par1
			expectModuleOrder(parallelBranches[0].modules, ['par1'])
			// Second parallel branch has par2 and added par3 in correct order
			expectModuleOrder(parallelBranches[1].modules, ['par2', 'par3'])
		})
	})
})
