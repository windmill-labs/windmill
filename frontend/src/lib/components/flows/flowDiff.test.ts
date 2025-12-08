import { describe, it, expect } from 'vitest'
import { buildFlowTimeline, hasInputSchemaChanged } from './flowDiff'
import type { FlowValue, RawScript, ForloopFlow, WhileloopFlow, BranchOne, BranchAll } from '$lib/gen'
import {
	createRawScriptModule,
	createIdentityModule,
	createForloopModule,
	createWhileloopModule,
	createBranchOneModule,
	createBranchAllModule,
	createFlow,
	createFlowWithSpecialModules,
	expectModuleOrder
} from './flowDiff.testUtils'

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

	describe('special modules', () => {
		it('detects added failure_module', () => {
			const moduleA = createRawScriptModule('a', 'main step')
			const failureModule = createRawScriptModule('failure', 'handle failure')

			const beforeFlow = createFlowWithSpecialModules({ modules: [moduleA] })
			const afterFlow = createFlowWithSpecialModules({
				modules: [moduleA],
				failure_module: failureModule
			})

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.afterActions['failure']).toEqual({ action: 'added', pending: false })
			expect(result.beforeActions['failure']).toBeUndefined()

			// Verify mergedFlow contains the failure_module
			expect(result.mergedFlow.failure_module?.id).toBe('failure')
		})

		it('detects removed failure_module', () => {
			const moduleA = createRawScriptModule('a', 'main step')
			const failureModule = createRawScriptModule('failure', 'handle failure')

			const beforeFlow = createFlowWithSpecialModules({
				modules: [moduleA],
				failure_module: failureModule
			})
			const afterFlow = createFlowWithSpecialModules({ modules: [moduleA] })

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.beforeActions['failure']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['failure']).toEqual({ action: 'removed', pending: false })

			// Verify mergedFlow contains the removed failure_module
			expect(result.mergedFlow.failure_module?.id).toBe('failure')
		})

		it('detects modified failure_module', () => {
			const moduleA = createRawScriptModule('a', 'main step')
			const failureModuleBefore = createRawScriptModule('failure', 'handle failure v1')
			const failureModuleAfter = createRawScriptModule('failure', 'handle failure v2')

			const beforeFlow = createFlowWithSpecialModules({
				modules: [moduleA],
				failure_module: failureModuleBefore
			})
			const afterFlow = createFlowWithSpecialModules({
				modules: [moduleA],
				failure_module: failureModuleAfter
			})

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.beforeActions['failure']).toEqual({ action: 'modified', pending: false })
			expect(result.afterActions['failure']).toEqual({ action: 'modified', pending: false })

			// Verify mergedFlow contains the modified failure_module with new content
			expect(result.mergedFlow.failure_module?.id).toBe('failure')
			expect((result.mergedFlow.failure_module?.value as RawScript).content).toBe(
				'handle failure v2'
			)
		})

		it('detects added preprocessor_module', () => {
			const moduleA = createRawScriptModule('a', 'main step')
			const preprocessorModule = createRawScriptModule('preprocessor', 'preprocess input')

			const beforeFlow = createFlowWithSpecialModules({ modules: [moduleA] })
			const afterFlow = createFlowWithSpecialModules({
				modules: [moduleA],
				preprocessor_module: preprocessorModule
			})

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.afterActions['preprocessor']).toEqual({ action: 'added', pending: false })
			expect(result.beforeActions['preprocessor']).toBeUndefined()

			// Verify mergedFlow contains the preprocessor_module
			expect(result.mergedFlow.preprocessor_module?.id).toBe('preprocessor')
		})

		it('detects removed preprocessor_module', () => {
			const moduleA = createRawScriptModule('a', 'main step')
			const preprocessorModule = createRawScriptModule('preprocessor', 'preprocess input')

			const beforeFlow = createFlowWithSpecialModules({
				modules: [moduleA],
				preprocessor_module: preprocessorModule
			})
			const afterFlow = createFlowWithSpecialModules({ modules: [moduleA] })

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.beforeActions['preprocessor']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['preprocessor']).toEqual({ action: 'removed', pending: false })

			// Verify mergedFlow contains the removed preprocessor_module
			expect(result.mergedFlow.preprocessor_module?.id).toBe('preprocessor')
		})

		it('detects changes to both failure and preprocessor modules simultaneously', () => {
			const moduleA = createRawScriptModule('a', 'main step')
			const failureModuleBefore = createRawScriptModule('failure', 'handle failure v1')
			const failureModuleAfter = createRawScriptModule('failure', 'handle failure v2')
			const preprocessorModule = createRawScriptModule('preprocessor', 'preprocess input')

			const beforeFlow = createFlowWithSpecialModules({
				modules: [moduleA],
				failure_module: failureModuleBefore,
				preprocessor_module: preprocessorModule
			})
			const afterFlow = createFlowWithSpecialModules({
				modules: [moduleA],
				failure_module: failureModuleAfter
				// preprocessor_module removed
			})

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// failure_module was modified
			expect(result.beforeActions['failure']).toEqual({ action: 'modified', pending: false })
			expect(result.afterActions['failure']).toEqual({ action: 'modified', pending: false })

			// preprocessor_module was removed
			expect(result.beforeActions['preprocessor']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['preprocessor']).toEqual({ action: 'removed', pending: false })

			// Verify mergedFlow structure
			expect(result.mergedFlow.failure_module?.id).toBe('failure')
			expect(result.mergedFlow.preprocessor_module?.id).toBe('preprocessor')
		})
	})

	describe('branchall entire branch removal', () => {
		it('detects removal of entire parallel branch from branchall', () => {
			const parallel1A = createRawScriptModule('p1_a', 'parallel 1 a')
			const parallel1B = createRawScriptModule('p1_b', 'parallel 1 b')
			const parallel2A = createRawScriptModule('p2_a', 'parallel 2 a')

			const beforeBranchAll = createBranchAllModule('branchall1', [
				{ modules: [parallel1A, parallel1B] },
				{ modules: [parallel2A] } // entire branch removed
			])
			const afterBranchAll = createBranchAllModule('branchall1', [
				{ modules: [parallel1A, parallel1B] }
			])

			const beforeFlow = createFlow([beforeBranchAll])
			const afterFlow = createFlow([afterBranchAll])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// Module from removed branch should be marked as removed
			expect(result.beforeActions['p2_a']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['p2_a']).toEqual({ action: 'removed', pending: false })

			// Modules from preserved branch should be unchanged
			expect(result.beforeActions['p1_a']).toBeUndefined()
			expect(result.beforeActions['p1_b']).toBeUndefined()

			// Verify mergedFlow structure - should have 2 branches with removed modules restored
			expectModuleOrder(result.mergedFlow.modules ?? [], ['branchall1'])
			const mergedBranchAll = result.mergedFlow.modules?.find((m) => m.id === 'branchall1')
			const branches = (mergedBranchAll?.value as BranchAll).branches

			// First branch unchanged
			expectModuleOrder(branches[0].modules, ['p1_a', 'p1_b'])

			// Second branch should be restored with its module
			expect(branches).toHaveLength(2)
			expectModuleOrder(branches[1].modules, ['p2_a'])
		})

		it('detects removal of multiple parallel branches from branchall', () => {
			const p1_a = createRawScriptModule('p1_a', 'parallel 1')
			const p2_a = createRawScriptModule('p2_a', 'parallel 2')
			const p3_a = createRawScriptModule('p3_a', 'parallel 3')

			const beforeBranchAll = createBranchAllModule('branchall1', [
				{ modules: [p1_a] },
				{ modules: [p2_a] }, // removed
				{ modules: [p3_a] } // removed
			])
			const afterBranchAll = createBranchAllModule('branchall1', [{ modules: [p1_a] }])

			const beforeFlow = createFlow([beforeBranchAll])
			const afterFlow = createFlow([afterBranchAll])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// Modules from removed branches should be marked as removed
			expect(result.beforeActions['p2_a']).toEqual({ action: 'removed', pending: false })
			expect(result.beforeActions['p3_a']).toEqual({ action: 'removed', pending: false })

			// Verify mergedFlow structure - should have 3 branches restored
			const mergedBranchAll = result.mergedFlow.modules?.find((m) => m.id === 'branchall1')
			const branches = (mergedBranchAll?.value as BranchAll).branches

			expect(branches).toHaveLength(3)
			expectModuleOrder(branches[0].modules, ['p1_a'])
			expectModuleOrder(branches[1].modules, ['p2_a'])
			expectModuleOrder(branches[2].modules, ['p3_a'])
		})
	})

	describe('hasInputSchemaChanged', () => {
		it('returns false for identical schemas', () => {
			const schema = {
				type: 'object',
				properties: {
					name: { type: 'string' },
					age: { type: 'number' }
				}
			}
			const beforeFlow = { schema }
			const afterFlow = { schema: JSON.parse(JSON.stringify(schema)) }

			expect(hasInputSchemaChanged(beforeFlow, afterFlow)).toBe(false)
		})

		it('returns true for different schemas', () => {
			const beforeFlow = {
				schema: {
					type: 'object',
					properties: {
						name: { type: 'string' }
					}
				}
			}
			const afterFlow = {
				schema: {
					type: 'object',
					properties: {
						name: { type: 'string' },
						email: { type: 'string' }
					}
				}
			}

			expect(hasInputSchemaChanged(beforeFlow, afterFlow)).toBe(true)
		})

		it('returns false for undefined flows', () => {
			expect(hasInputSchemaChanged(undefined, undefined)).toBe(false)
			expect(hasInputSchemaChanged(undefined, { schema: {} })).toBe(false)
			expect(hasInputSchemaChanged({ schema: {} }, undefined)).toBe(false)
		})

		it('returns true when schema is added', () => {
			const beforeFlow = {}
			const afterFlow = { schema: { type: 'object' } }

			expect(hasInputSchemaChanged(beforeFlow, afterFlow)).toBe(true)
		})

		it('returns true when schema is removed', () => {
			const beforeFlow = { schema: { type: 'object' } }
			const afterFlow = {}

			expect(hasInputSchemaChanged(beforeFlow, afterFlow)).toBe(true)
		})

		it('returns false for both empty schemas', () => {
			const beforeFlow = { schema: {} }
			const afterFlow = { schema: {} }

			expect(hasInputSchemaChanged(beforeFlow, afterFlow)).toBe(false)
		})
	})

	describe('multiple removed modules ordering', () => {
		it('restores multiple removed modules in correct order', () => {
			const moduleA = createRawScriptModule('a', 'first')
			const moduleB = createRawScriptModule('b', 'second')
			const moduleC = createRawScriptModule('c', 'third')
			const moduleD = createRawScriptModule('d', 'fourth')
			const moduleE = createRawScriptModule('e', 'fifth')

			// Remove modules at beginning, middle, and end
			const beforeFlow = createFlow([moduleA, moduleB, moduleC, moduleD, moduleE])
			const afterFlow = createFlow([moduleB, moduleD]) // Remove a, c, e

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// All removed modules should be marked as removed
			expect(result.beforeActions['a']).toEqual({ action: 'removed', pending: false })
			expect(result.beforeActions['c']).toEqual({ action: 'removed', pending: false })
			expect(result.beforeActions['e']).toEqual({ action: 'removed', pending: false })

			// Verify mergedFlow restores all modules in the original order
			expectModuleOrder(result.mergedFlow.modules ?? [], ['a', 'b', 'c', 'd', 'e'])
		})

		it('restores multiple removed modules inside a loop in correct order', () => {
			const innerA = createRawScriptModule('inner_a', 'first')
			const innerB = createRawScriptModule('inner_b', 'second')
			const innerC = createRawScriptModule('inner_c', 'third')
			const innerD = createRawScriptModule('inner_d', 'fourth')

			const beforeLoop = createForloopModule('loop1', [innerA, innerB, innerC, innerD])
			const afterLoop = createForloopModule('loop1', [innerB]) // Remove a, c, d

			const beforeFlow = createFlow([beforeLoop])
			const afterFlow = createFlow([afterLoop])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// All removed modules should be marked as removed
			expect(result.beforeActions['inner_a']).toEqual({ action: 'removed', pending: false })
			expect(result.beforeActions['inner_c']).toEqual({ action: 'removed', pending: false })
			expect(result.beforeActions['inner_d']).toEqual({ action: 'removed', pending: false })

			// Verify mergedFlow restores all modules in the original order inside the loop
			const mergedLoop = result.mergedFlow.modules?.find((m) => m.id === 'loop1')
			const loopModules = (mergedLoop?.value as ForloopFlow).modules
			expectModuleOrder(loopModules, ['inner_a', 'inner_b', 'inner_c', 'inner_d'])
		})
	})

	describe('insert position tests', () => {
		it('handles module added at beginning of list', () => {
			const moduleA = createRawScriptModule('a', 'existing first')
			const moduleB = createRawScriptModule('b', 'existing second')
			const moduleNew = createRawScriptModule('new', 'new at beginning')

			const beforeFlow = createFlow([moduleA, moduleB])
			const afterFlow = createFlow([moduleNew, moduleA, moduleB])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.afterActions['new']).toEqual({ action: 'added', pending: false })

			// Verify mergedFlow maintains correct order with new module at beginning
			expectModuleOrder(result.mergedFlow.modules ?? [], ['new', 'a', 'b'])
		})

		it('handles module added in middle of list', () => {
			const moduleA = createRawScriptModule('a', 'first')
			const moduleB = createRawScriptModule('b', 'second')
			const moduleC = createRawScriptModule('c', 'third')
			const moduleNew = createRawScriptModule('new', 'new in middle')

			const beforeFlow = createFlow([moduleA, moduleB, moduleC])
			const afterFlow = createFlow([moduleA, moduleNew, moduleB, moduleC])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.afterActions['new']).toEqual({ action: 'added', pending: false })

			// Verify mergedFlow maintains correct order with new module in middle
			expectModuleOrder(result.mergedFlow.modules ?? [], ['a', 'new', 'b', 'c'])
		})

		it('handles module added at beginning inside a loop', () => {
			const innerA = createRawScriptModule('inner_a', 'existing first')
			const innerB = createRawScriptModule('inner_b', 'existing second')
			const innerNew = createRawScriptModule('inner_new', 'new at beginning')

			const beforeLoop = createForloopModule('loop1', [innerA, innerB])
			const afterLoop = createForloopModule('loop1', [innerNew, innerA, innerB])

			const beforeFlow = createFlow([beforeLoop])
			const afterFlow = createFlow([afterLoop])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.afterActions['inner_new']).toEqual({ action: 'added', pending: false })

			// Verify correct order inside the loop
			const mergedLoop = result.mergedFlow.modules?.find((m) => m.id === 'loop1')
			const loopModules = (mergedLoop?.value as ForloopFlow).modules
			expectModuleOrder(loopModules, ['inner_new', 'inner_a', 'inner_b'])
		})

		it('handles multiple modules added at different positions', () => {
			const moduleA = createRawScriptModule('a', 'original a')
			const moduleB = createRawScriptModule('b', 'original b')
			const moduleNew1 = createRawScriptModule('new1', 'new at start')
			const moduleNew2 = createRawScriptModule('new2', 'new in middle')
			const moduleNew3 = createRawScriptModule('new3', 'new at end')

			const beforeFlow = createFlow([moduleA, moduleB])
			const afterFlow = createFlow([moduleNew1, moduleA, moduleNew2, moduleB, moduleNew3])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.afterActions['new1']).toEqual({ action: 'added', pending: false })
			expect(result.afterActions['new2']).toEqual({ action: 'added', pending: false })
			expect(result.afterActions['new3']).toEqual({ action: 'added', pending: false })

			// Verify mergedFlow maintains correct order
			expectModuleOrder(result.mergedFlow.modules ?? [], ['new1', 'a', 'new2', 'b', 'new3'])
		})
	})

	describe('empty containers', () => {
		it('handles adding empty forloop', () => {
			const moduleA = createRawScriptModule('a', 'step a')
			const emptyLoop = createForloopModule('empty_loop', [])

			const beforeFlow = createFlow([moduleA])
			const afterFlow = createFlow([moduleA, emptyLoop])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.afterActions['empty_loop']).toEqual({ action: 'added', pending: false })
			expectModuleOrder(result.mergedFlow.modules ?? [], ['a', 'empty_loop'])

			// Verify the empty loop has no modules
			const mergedLoop = result.mergedFlow.modules?.find((m) => m.id === 'empty_loop')
			expect((mergedLoop?.value as ForloopFlow).modules).toHaveLength(0)
		})

		it('handles removing empty forloop', () => {
			const moduleA = createRawScriptModule('a', 'step a')
			const emptyLoop = createForloopModule('empty_loop', [])

			const beforeFlow = createFlow([moduleA, emptyLoop])
			const afterFlow = createFlow([moduleA])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.beforeActions['empty_loop']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['empty_loop']).toEqual({ action: 'removed', pending: false })

			// Verify the removed empty loop is in mergedFlow
			expectModuleOrder(result.mergedFlow.modules ?? [], ['a', 'empty_loop'])
		})

		it('handles adding empty branchone', () => {
			const moduleA = createRawScriptModule('a', 'step a')
			const emptyBranch = createBranchOneModule('empty_branch', [], [])

			const beforeFlow = createFlow([moduleA])
			const afterFlow = createFlow([moduleA, emptyBranch])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.afterActions['empty_branch']).toEqual({ action: 'added', pending: false })
			expectModuleOrder(result.mergedFlow.modules ?? [], ['a', 'empty_branch'])

			// Verify the empty branch has no modules in default or branches
			const mergedBranch = result.mergedFlow.modules?.find((m) => m.id === 'empty_branch')
			expect((mergedBranch?.value as BranchOne).default).toHaveLength(0)
			expect((mergedBranch?.value as BranchOne).branches).toHaveLength(0)
		})

		it('handles removing empty branchone with empty default and branches', () => {
			const moduleA = createRawScriptModule('a', 'step a')
			const emptyBranch = createBranchOneModule('empty_branch', [], [{ expr: 'true', modules: [] }])

			const beforeFlow = createFlow([moduleA, emptyBranch])
			const afterFlow = createFlow([moduleA])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.beforeActions['empty_branch']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['empty_branch']).toEqual({ action: 'removed', pending: false })

			// Verify the removed empty branch is in mergedFlow with its structure
			expectModuleOrder(result.mergedFlow.modules ?? [], ['a', 'empty_branch'])
			const mergedBranch = result.mergedFlow.modules?.find((m) => m.id === 'empty_branch')
			expect((mergedBranch?.value as BranchOne).default).toHaveLength(0)
			expect((mergedBranch?.value as BranchOne).branches).toHaveLength(1)
		})
	})

	describe('container type changes', () => {
		it('treats forloop to whileloop change as removed + added', () => {
			const innerModule = createRawScriptModule('inner', 'inner step')

			const forLoop = createForloopModule('loop1', [innerModule])
			const whileLoop = createWhileloopModule('loop1', [innerModule])

			const beforeFlow = createFlow([forLoop])
			const afterFlow = createFlow([whileLoop])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// The container itself should be treated as removed + added (type changed)
			expect(result.beforeActions['loop1']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['loop1']).toEqual({ action: 'added', pending: false })

			// Inner module's location type changes (forloop -> whileloop), so it's also treated as moved
			expect(result.beforeActions['inner']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['inner']).toEqual({ action: 'added', pending: false })

			// The old inner module should appear with prefix in the merged flow
			const mergedLoop = result.mergedFlow.modules?.find((m) => m.id === 'loop1')
			expect(mergedLoop).toBeDefined()
			const loopModules = (mergedLoop?.value as WhileloopFlow).modules
			// The whileloop contains 'inner' (new) and the old one gets prefixed
			expect(loopModules.some((m) => m.id === 'inner')).toBe(true)
		})

		it('treats branchone to branchall change as removed + added', () => {
			const innerModule = createRawScriptModule('inner', 'inner step')

			const branchOne = createBranchOneModule('branch1', [innerModule], [])
			const branchAll = createBranchAllModule('branch1', [{ modules: [innerModule] }])

			const beforeFlow = createFlow([branchOne])
			const afterFlow = createFlow([branchAll])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// The container itself should be treated as removed + added (type changed)
			expect(result.beforeActions['branch1']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['branch1']).toEqual({ action: 'added', pending: false })

			// Inner module's location type changes (branchone-default -> branchall-branch), so it's also treated as moved
			expect(result.beforeActions['inner']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['inner']).toEqual({ action: 'added', pending: false })

			// The new inner module should appear in the branchall
			const mergedBranch = result.mergedFlow.modules?.find((m) => m.id === 'branch1')
			expect(mergedBranch).toBeDefined()
			const branchModules = (mergedBranch?.value as BranchAll).branches[0].modules
			expect(branchModules.some((m) => m.id === 'inner')).toBe(true)
		})
	})

	describe('module movement', () => {
		it('detects module moved from root to inside a loop', () => {
			const moduleA = createRawScriptModule('a', 'step a')
			const moduleB = createRawScriptModule('b', 'step b')
			const emptyLoop = createForloopModule('loop1', [])
			const loopWithB = createForloopModule('loop1', [moduleB])

			// Before: a, b, loop1(empty)
			// After: a, loop1(b)
			const beforeFlow = createFlow([moduleA, moduleB, emptyLoop])
			const afterFlow = createFlow([moduleA, loopWithB])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// Module b is in both flows but at different locations
			// The implementation doesn't track "movement" - it sees b in both places
			// The loop itself is modified because its modules changed
			expect(result.beforeActions['loop1']).toEqual({ action: 'modified', pending: false })
			expect(result.afterActions['loop1']).toEqual({ action: 'modified', pending: false })

			// b is removed from root and added inside loop - implementation sees it as existing in both
			// but since it's the same module ID, no action is recorded
			// The mergedFlow should contain both the old position (at root) and the new position (in loop)
			// This is handled by the duplicate ID logic - old__b prefix
			const rootModuleIds = result.mergedFlow.modules?.map((m) => m.id) ?? []
			expect(rootModuleIds).toContain('a')
			expect(rootModuleIds).toContain('loop1')
			// The old 'b' at root level gets prefixed with 'old__'
			expect(rootModuleIds).toContain('old__b')
		})

		it('detects module moved from one branch to another', () => {
			const moduleA = createRawScriptModule('a', 'step a')

			// Before: branch with 'a' in first conditional branch
			const beforeBranch = createBranchOneModule('branch1', [], [
				{ expr: 'x > 0', modules: [moduleA] },
				{ expr: 'x < 0', modules: [] }
			])
			// After: branch with 'a' in second conditional branch
			const afterBranch = createBranchOneModule('branch1', [], [
				{ expr: 'x > 0', modules: [] },
				{ expr: 'x < 0', modules: [moduleA] }
			])

			const beforeFlow = createFlow([beforeBranch])
			const afterFlow = createFlow([afterBranch])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// The branch container is modified
			expect(result.beforeActions['branch1']).toEqual({ action: 'modified', pending: false })
			expect(result.afterActions['branch1']).toEqual({ action: 'modified', pending: false })

			// Module 'a' moved from branchIndex 0 to branchIndex 1, so it's treated as removed + added
			expect(result.beforeActions['a']).toEqual({ action: 'removed', pending: false })
			expect(result.afterActions['a']).toEqual({ action: 'added', pending: false })

			// The old 'a' should appear with prefix in the first branch of mergedFlow
			const mergedBranch = result.mergedFlow.modules?.find((m) => m.id === 'branch1')
			expect(mergedBranch).toBeDefined()
			const firstBranchModules = (mergedBranch?.value as BranchOne).branches[0].modules
			expect(firstBranchModules.some((m) => m.id === 'old__a')).toBe(true)

			// The new 'a' should appear in the second branch
			const secondBranchModules = (mergedBranch?.value as BranchOne).branches[1].modules
			expect(secondBranchModules.some((m) => m.id === 'a')).toBe(true)
		})

		it('detects module moved from loop to root', () => {
			const moduleA = createRawScriptModule('a', 'step a')
			const moduleB = createRawScriptModule('b', 'step b')

			const loopWithA = createForloopModule('loop1', [moduleA])
			const emptyLoop = createForloopModule('loop1', [])

			// Before: b, loop1(a)
			// After: b, loop1(empty), a
			const beforeFlow = createFlow([moduleB, loopWithA])
			const afterFlow = createFlow([moduleB, emptyLoop, moduleA])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// The loop is modified because its modules changed
			expect(result.beforeActions['loop1']).toEqual({ action: 'modified', pending: false })
			expect(result.afterActions['loop1']).toEqual({ action: 'modified', pending: false })

			// Module 'a' exists in both flows, so no action recorded for it
			// The mergedFlow should contain the old 'a' inside the loop (prefixed) and new 'a' at root
			const rootModuleIds = result.mergedFlow.modules?.map((m) => m.id) ?? []
			expect(rootModuleIds).toContain('b')
			expect(rootModuleIds).toContain('loop1')
			expect(rootModuleIds).toContain('a')

			// The old 'a' inside the loop gets prefixed
			const mergedLoop = result.mergedFlow.modules?.find((m) => m.id === 'loop1')
			const loopModuleIds = (mergedLoop?.value as ForloopFlow).modules.map((m) => m.id)
			expect(loopModuleIds).toContain('old__a')
		})
	})

	describe('module reordering', () => {
		it('detects modules reordered at root level', () => {
			const moduleA = createRawScriptModule('a', 'step a')
			const moduleB = createRawScriptModule('b', 'step b')
			const moduleC = createRawScriptModule('c', 'step c')

			// Before: a, b, c
			// After: c, a, b
			const beforeFlow = createFlow([moduleA, moduleB, moduleC])
			const afterFlow = createFlow([moduleC, moduleA, moduleB])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// No modules were added, removed, or modified - just reordered
			// The implementation doesn't detect reordering as a change
			expect(result.beforeActions['a']).toBeUndefined()
			expect(result.beforeActions['b']).toBeUndefined()
			expect(result.beforeActions['c']).toBeUndefined()
			expect(result.afterActions['a']).toBeUndefined()
			expect(result.afterActions['b']).toBeUndefined()
			expect(result.afterActions['c']).toBeUndefined()

			// mergedFlow should reflect the afterFlow order
			expectModuleOrder(result.mergedFlow.modules ?? [], ['c', 'a', 'b'])
		})

		it('detects modules reordered inside a loop', () => {
			const innerA = createRawScriptModule('inner_a', 'inner a')
			const innerB = createRawScriptModule('inner_b', 'inner b')
			const innerC = createRawScriptModule('inner_c', 'inner c')

			// Before: loop with [a, b, c]
			// After: loop with [c, b, a]
			const beforeLoop = createForloopModule('loop1', [innerA, innerB, innerC])
			const afterLoop = createForloopModule('loop1', [innerC, innerB, innerA])

			const beforeFlow = createFlow([beforeLoop])
			const afterFlow = createFlow([afterLoop])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			// The loop is modified because its internal structure changed
			// (deepEqual compares the modules array which includes order)
			expect(result.beforeActions['loop1']).toEqual({ action: 'modified', pending: false })
			expect(result.afterActions['loop1']).toEqual({ action: 'modified', pending: false })

			// Individual modules are not marked as changed
			expect(result.beforeActions['inner_a']).toBeUndefined()
			expect(result.beforeActions['inner_b']).toBeUndefined()
			expect(result.beforeActions['inner_c']).toBeUndefined()

			// mergedFlow should reflect the afterFlow order inside the loop
			const mergedLoop = result.mergedFlow.modules?.find((m) => m.id === 'loop1')
			const loopModules = (mergedLoop?.value as ForloopFlow).modules
			expectModuleOrder(loopModules, ['inner_c', 'inner_b', 'inner_a'])
		})
	})
})
