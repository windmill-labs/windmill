import { describe, it, expect } from 'vitest'
import { flushSync } from 'svelte'
import { createFlowDiffManager } from './flowDiffManager.svelte'
import { DUPLICATE_MODULE_PREFIX, NEW_MODULE_PREFIX } from './flowDiff'
import type { FlowValue, RawScript, ForloopFlow, BranchOne, BranchAll } from '$lib/gen'
import { SPECIAL_MODULE_IDS } from '../copilot/chat/shared'
import {
	createRawScriptModule,
	createForloopModule,
	createBranchOneModule,
	createBranchAllModule,
	createExtendedOpenFlow,
	createFlowStore,
	deepClone,
	expectModuleOrder,
	createIdentityModule
} from './flowDiff.testUtils'

describe('FlowDiffManager', () => {
	describe('effect auto-computation', () => {
		it('auto-computes moduleActions when flows differ', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'before')
				const beforeFlow = createExtendedOpenFlow({ modules: [moduleA] })

				const moduleB = createRawScriptModule('b', 'new')
				const afterFlow: FlowValue = { modules: [moduleA, moduleB] }

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)

				flushSync()

				expect(manager.moduleActions).toHaveProperty('b')
				expect(manager.moduleActions['b']).toEqual({ action: 'added', pending: true })
			})
			cleanup()
		})
	})

	describe('acceptModule', () => {
		it('accepts added module - inserts into beforeFlow', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const beforeFlow = createExtendedOpenFlow({ modules: [moduleA] })
				const afterFlow: FlowValue = { modules: [moduleA, moduleB] }

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				// Verify 'b' is detected as added
				expect(manager.moduleActions['b']).toEqual({ action: 'added', pending: true })

				// Accept the added module
				manager.acceptModule('b')
				flushSync()

				// After accepting, beforeFlow should now contain module 'b'
				const beforeModules = manager.beforeFlow?.value.modules ?? []
				expect(beforeModules.map((m) => m.id)).toContain('b')
				// The action should be cleared since beforeFlow now matches currentFlow
				expect(manager.moduleActions['b']).toBeUndefined()
			})
			cleanup()
		})

		it('accepts removed module - deletes from beforeFlow', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const beforeFlow = createExtendedOpenFlow({ modules: [moduleA, moduleB] })
				const afterFlow: FlowValue = { modules: [moduleA] }

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				// Verify 'b' is detected as removed
				expect(manager.moduleActions['b']).toEqual({ action: 'removed', pending: true })

				// Accept the removal
				manager.acceptModule('b')
				flushSync()

				// After accepting, beforeFlow should no longer contain module 'b'
				const beforeModules = manager.beforeFlow?.value.modules ?? []
				expect(beforeModules.map((m) => m.id)).not.toContain('b')
				expect(manager.moduleActions['b']).toBeUndefined()
			})
			cleanup()
		})

		it('accepts modified module - updates beforeFlow content', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleBeforeA = createRawScriptModule('a', 'original-content')
				const moduleAfterA = createRawScriptModule('a', 'modified-content')
				const beforeFlow = createExtendedOpenFlow({ modules: [moduleBeforeA] })
				const afterFlow: FlowValue = { modules: [moduleAfterA] }

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				// Verify 'a' is detected as modified
				expect(manager.moduleActions['a']).toEqual({ action: 'modified', pending: true })

				// Accept the modification
				manager.acceptModule('a')
				flushSync()

				// After accepting, beforeFlow module should have the new content
				const beforeModules = manager.beforeFlow?.value.modules ?? []
				const moduleA = beforeModules.find((m) => m.id === 'a')
				expect((moduleA?.value as RawScript).content).toBe('modified-content')
				expect(manager.moduleActions['a']).toBeUndefined()
			})
			cleanup()
		})

		it('accepts added nested module - parent accepted as skeleton first', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				// beforeFlow: empty
				const beforeFlow = createExtendedOpenFlow({ modules: [] })

				// afterFlow: forloop containing a nested module
				const nestedModule = createRawScriptModule('nested', 'nested-content')
				const forloop = createForloopModule('loop', [nestedModule])
				const afterFlow: FlowValue = { modules: [forloop] }

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				// Both 'loop' and 'nested' should be detected as added
				expect(manager.moduleActions['loop']).toEqual({ action: 'added', pending: true })
				expect(manager.moduleActions['nested']).toEqual({ action: 'added', pending: true })

				// Accept the nested module first - should auto-accept parent as skeleton
				manager.acceptModule('nested')
				flushSync()

				// After accepting nested, the loop should also be in beforeFlow (as skeleton initially)
				const beforeModules = manager.beforeFlow?.value.modules ?? []
				expect(beforeModules.map((m) => m.id)).toContain('loop')

				// The nested module should be in the loop
				const loopModule = beforeModules.find((m) => m.id === 'loop')
				const loopValue = loopModule?.value as ForloopFlow
				expect(loopValue.modules.map((m) => m.id)).toContain('nested')
			})
			cleanup()
		})

		it('accepts input schema changes', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const beforeSchema = {
					properties: { x: { type: 'string' } }
				}
				const afterSchema = {
					properties: { x: { type: 'string' }, y: { type: 'number' } }
				}
				const beforeFlow = createExtendedOpenFlow({ modules: [] }, beforeSchema)
				const afterFlow: FlowValue = { modules: [] }

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				manager.setCurrentInputSchema(afterSchema)
				flushSync()

				// Input schema change should be detected
				expect(manager.moduleActions[SPECIAL_MODULE_IDS.INPUT]).toEqual({
					action: 'modified',
					pending: true
				})

				// Accept the input schema change
				manager.acceptModule(SPECIAL_MODULE_IDS.INPUT)
				flushSync()

				// beforeFlow schema should now match currentInputSchema
				expect(manager.beforeFlow?.schema).toEqual(afterSchema)
			})
			cleanup()
		})

		it('handles duplicate ID prefix (old__) for type changes', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })
	   
				// Create a real type change: rawscript -> identity
				const moduleBeforeA = createRawScriptModule('a', 'content')
				const moduleAfterA = createIdentityModule('a')
				const beforeFlow = createExtendedOpenFlow({ modules: [moduleBeforeA] })
				const afterFlow: FlowValue = { modules: [moduleAfterA] }
	   
				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()
	   
				// Type change produces: 'a' (added) and 'old__a' (removed)
				expect(manager.moduleActions['a']).toEqual({ action: 'added', pending: true })
				expect(manager.moduleActions[`${DUPLICATE_MODULE_PREFIX}a`]).toEqual({ action: 'removed', pending: true })
	   
				// Accept the removal (old__a) - should remove original module from beforeFlow
				manager.acceptModule(`${DUPLICATE_MODULE_PREFIX}a`)
				flushSync()
	   
				// The module 'a' (original rawscript) should be removed from beforeFlow
				const beforeModules = manager.beforeFlow?.value.modules ?? []
				expect(beforeModules.map((m) => m.id)).not.toContain('a')
			})
			cleanup()
		})
	})

	describe('rejectModule', () => {
		it('rejects added module - removes from flowStore', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const beforeFlow = createExtendedOpenFlow({ modules: [deepClone(moduleA)] })

				const currentFlowValue: FlowValue = {
					modules: [deepClone(moduleA), deepClone(moduleB)]
				}
				const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(flowStore.val.value)
				flushSync()

				expect(manager.moduleActions['b']).toEqual({ action: 'added', pending: true })

				// Reject the added module
				manager.rejectModule('b', flowStore)
				flushSync()

				// After rejecting, flowStore should no longer contain module 'b'
				const currentModules = flowStore.val.value.modules
				expect(currentModules.map((m) => m.id)).not.toContain('b')
				expect(currentModules.map((m) => m.id)).toContain('a')
			})
			cleanup()
		})

		it('rejects removed module - restores to flowStore', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const beforeFlow = createExtendedOpenFlow({
					modules: [deepClone(moduleA), deepClone(moduleB)]
				})

				const currentFlowValue: FlowValue = { modules: [deepClone(moduleA)] }
				const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(flowStore.val.value)
				flushSync()

				expect(manager.moduleActions['b']).toEqual({ action: 'removed', pending: true })

				// Reject the removal - should restore module 'b'
				manager.rejectModule('b', flowStore)
				flushSync()

				// After rejecting, flowStore should contain module 'b' again
				const currentModules = flowStore.val.value.modules
				expect(currentModules.map((m) => m.id)).toContain('b')
			})
			cleanup()
		})

		it('rejects modified module - reverts flowStore to beforeFlow state', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleBeforeA = createRawScriptModule('a', 'original-content')
				const moduleAfterA = createRawScriptModule('a', 'modified-content')
				const beforeFlow = createExtendedOpenFlow({ modules: [deepClone(moduleBeforeA)] })

				const currentFlowValue: FlowValue = { modules: [deepClone(moduleAfterA)] }
				const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(flowStore.val.value)
				flushSync()

				expect(manager.moduleActions['a']).toEqual({ action: 'modified', pending: true })

				// Reject the modification
				manager.rejectModule('a', flowStore)
				flushSync()

				// After rejecting, flowStore should have the original content
				const moduleA = flowStore.val.value.modules.find((m) => m.id === 'a')
				expect((moduleA?.value as RawScript).content).toBe('original-content')
			})
			cleanup()
		})

		it('rejects input schema changes - reverts flowStore schema', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const beforeSchema = { properties: { x: { type: 'string' } } }
				const afterSchema = { properties: { x: { type: 'string' }, y: { type: 'number' } } }
				const beforeFlow = createExtendedOpenFlow({ modules: [] }, beforeSchema)

				const flowStore = createFlowStore(createExtendedOpenFlow({ modules: [] }, afterSchema))

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(flowStore.val.value)
				manager.setCurrentInputSchema(afterSchema)
				flushSync()

				expect(manager.moduleActions[SPECIAL_MODULE_IDS.INPUT]).toEqual({
					action: 'modified',
					pending: true
				})

				// Reject the schema change
				manager.rejectModule(SPECIAL_MODULE_IDS.INPUT, flowStore)
				flushSync()

				// After rejecting, flowStore schema should match beforeFlow
				expect(flowStore.val.schema).toEqual(beforeSchema)
			})
			cleanup()
		})

		it('does not crash without flowStore', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content')
				const moduleB = createRawScriptModule('b', 'content')
				const beforeFlow = createExtendedOpenFlow({ modules: [moduleA] })
				const afterFlow: FlowValue = { modules: [moduleA, moduleB] }

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				// Should not throw when flowStore is not provided
				expect(() => manager.rejectModule('b')).not.toThrow()
			})
			cleanup()
		})

		it('does nothing for module not in actions', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content')
				const beforeFlow = createExtendedOpenFlow({ modules: [moduleA] })
				const afterFlow: FlowValue = { modules: [moduleA] }

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				// No actions should exist
				expect(Object.keys(manager.moduleActions)).toHaveLength(0)

				// Should not throw for non-existent module
				expect(() => manager.rejectModule('nonexistent')).not.toThrow()
			})
			cleanup()
		})

		it('rejects duplicate ID prefix (old__) for type changes - restores original and renames new', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				// Create a real type change: rawscript -> identity
				const moduleBeforeA = createRawScriptModule('a', 'original-content')
				const moduleAfterA = createIdentityModule('a')
				const beforeFlow = createExtendedOpenFlow({ modules: [deepClone(moduleBeforeA)] })

				const currentFlowValue: FlowValue = { modules: [deepClone(moduleAfterA)] }
				const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(flowStore.val.value)
				flushSync()

				// Type change produces: 'a' (added) and 'old__a' (removed)
				expect(manager.moduleActions['a']).toEqual({ action: 'added', pending: true })
				expect(manager.moduleActions[`${DUPLICATE_MODULE_PREFIX}a`]).toEqual({
					action: 'removed',
					pending: true
				})

				// Reject the removal (old__a) - should restore original and rename the new one
				manager.rejectModule(`${DUPLICATE_MODULE_PREFIX}a`, flowStore)
				flushSync()

				const currentModules = flowStore.val.value.modules

				// The original module 'a' (rawscript) should be restored
				const restoredOriginal = currentModules.find((m) => m.id === 'a')
				expect(restoredOriginal).toBeDefined()
				expect(restoredOriginal?.value.type).toBe('rawscript')

				// The new module should be renamed to 'new__a' so user can still accept/reject it
				const renamedNew = currentModules.find((m) => m.id === `${NEW_MODULE_PREFIX}a`)
				expect(renamedNew).toBeDefined()
				expect(renamedNew?.value.type).toBe('identity')
			})
			cleanup()
		})
	})

	describe('batch operations', () => {
		it('acceptAll accepts all pending modules', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const moduleC = createRawScriptModule('c', 'content-c')
				const beforeFlow = createExtendedOpenFlow({ modules: [moduleA] })
				const afterFlow: FlowValue = { modules: [moduleA, moduleB, moduleC] }

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				expect(manager.moduleActions['b']).toBeDefined()
				expect(manager.moduleActions['c']).toBeDefined()

				// Accept all
				manager.acceptAll()
				flushSync()

				// All modules should now be in beforeFlow
				const beforeModules = manager.beforeFlow?.value.modules ?? []
				expect(beforeModules.map((m) => m.id)).toContain('b')
				expect(beforeModules.map((m) => m.id)).toContain('c')
			})
			cleanup()
		})

		it('acceptAll skips non-pending actions', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content')
				const moduleB = createRawScriptModule('b', 'content')
				const beforeFlow = createExtendedOpenFlow({ modules: [moduleA] })
				const afterFlow: FlowValue = { modules: [moduleA, moduleB] }

				// Set editMode to false so actions are not pending
				manager.setEditMode(false)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				// Action should exist but not be pending
				expect(manager.moduleActions['b']?.pending).toBe(false)

				// acceptAll should skip non-pending
				manager.acceptAll()
				flushSync()

				// 'b' should still not be in beforeFlow
				const beforeModules = manager.beforeFlow?.value.modules ?? []
				expect(beforeModules.map((m) => m.id)).not.toContain('b')
			})
			cleanup()
		})

		it('rejectAll rejects all pending modules', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const moduleC = createRawScriptModule('c', 'content-c')
				const beforeFlow = createExtendedOpenFlow({ modules: [deepClone(moduleA)] })

				const currentFlowValue: FlowValue = {
					modules: [deepClone(moduleA), deepClone(moduleB), deepClone(moduleC)]
				}
				const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(flowStore.val.value)
				flushSync()

				expect(manager.moduleActions['b']).toBeDefined()
				expect(manager.moduleActions['c']).toBeDefined()

				// Reject all
				manager.rejectAll(flowStore)
				flushSync()

				// Both added modules should be removed from flowStore
				const currentModules = flowStore.val.value.modules
				expect(currentModules.map((m) => m.id)).not.toContain('b')
				expect(currentModules.map((m) => m.id)).not.toContain('c')
			})
			cleanup()
		})
	})

	describe('edge cases', () => {
		it('clearSnapshot clears all state', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content')
				const moduleB = createRawScriptModule('b', 'content')
				const beforeFlow = createExtendedOpenFlow({ modules: [moduleA] })
				const afterFlow: FlowValue = { modules: [moduleA, moduleB] }

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				expect(manager.beforeFlow).toBeDefined()
				expect(manager.currentFlow).toBeDefined()
				expect(Object.keys(manager.moduleActions).length).toBeGreaterThan(0)

				manager.clearSnapshot()
				flushSync()

				expect(manager.beforeFlow).toBeUndefined()
				expect(manager.currentFlow).toBeUndefined()
				expect(Object.keys(manager.moduleActions)).toHaveLength(0)
			})
			cleanup()
		})

		it('revertToSnapshot restores entire flow', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'original')
				const moduleB = createRawScriptModule('b', 'modified')
				const beforeFlow = createExtendedOpenFlow({ modules: [deepClone(moduleA)] })

				const currentFlowValue: FlowValue = { modules: [deepClone(moduleB)] }
				const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(flowStore.val.value)
				flushSync()

				// Revert to snapshot
				manager.revertToSnapshot(flowStore)
				flushSync()

				// flowStore should now be the beforeFlow
				expect(flowStore.val.value.modules.map((m) => m.id)).toEqual(['a'])
				expect((flowStore.val.value.modules[0].value as RawScript).content).toBe('original')
			})
			cleanup()
		})
	})

	describe('module positioning', () => {
		it('accept added module at beginning - inserts at correct position', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const moduleNew = createRawScriptModule('new', 'new at beginning')

				// beforeFlow: [a, b]
				// afterFlow: [new, a, b]
				const beforeFlow = createExtendedOpenFlow({
					modules: [deepClone(moduleA), deepClone(moduleB)]
				})
				const afterFlow: FlowValue = {
					modules: [deepClone(moduleNew), deepClone(moduleA), deepClone(moduleB)]
				}

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				expect(manager.moduleActions['new']).toEqual({ action: 'added', pending: true })

				// Accept the added module
				manager.acceptModule('new')
				flushSync()

				// Verify 'new' is inserted at the beginning
				const beforeModules = manager.beforeFlow?.value.modules ?? []
				expectModuleOrder(beforeModules, ['new', 'a', 'b'])
			})
			cleanup()
		})

		it('accept added module in middle - inserts at correct position', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const moduleC = createRawScriptModule('c', 'content-c')
				const moduleNew = createRawScriptModule('new', 'new in middle')

				// beforeFlow: [a, b, c]
				// afterFlow: [a, new, b, c]
				const beforeFlow = createExtendedOpenFlow({
					modules: [deepClone(moduleA), deepClone(moduleB), deepClone(moduleC)]
				})
				const afterFlow: FlowValue = {
					modules: [
						deepClone(moduleA),
						deepClone(moduleNew),
						deepClone(moduleB),
						deepClone(moduleC)
					]
				}

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				expect(manager.moduleActions['new']).toEqual({ action: 'added', pending: true })

				// Accept the added module
				manager.acceptModule('new')
				flushSync()

				// Verify 'new' is inserted at the correct middle position
				const beforeModules = manager.beforeFlow?.value.modules ?? []
				expectModuleOrder(beforeModules, ['a', 'new', 'b', 'c'])
			})
			cleanup()
		})

		it('reject removed module - restores at correct position', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const moduleC = createRawScriptModule('c', 'content-c')

				// beforeFlow: [a, b, c]
				// afterFlow (currentFlow): [a, c] - 'b' removed
				const beforeFlow = createExtendedOpenFlow({
					modules: [deepClone(moduleA), deepClone(moduleB), deepClone(moduleC)]
				})

				const currentFlowValue: FlowValue = {
					modules: [deepClone(moduleA), deepClone(moduleC)]
				}
				const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(flowStore.val.value)
				flushSync()

				expect(manager.moduleActions['b']).toEqual({ action: 'removed', pending: true })

				// Reject the removal - should restore 'b' at original position
				manager.rejectModule('b', flowStore)
				flushSync()

				// Verify 'b' is restored at the middle position
				const currentModules = flowStore.val.value.modules
				expectModuleOrder(currentModules, ['a', 'b', 'c'])
			})
			cleanup()
		})

		it('accept added module inside loop - inserts at correct nested position', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const innerA = createRawScriptModule('inner_a', 'inner a')
				const innerB = createRawScriptModule('inner_b', 'inner b')
				const innerNew = createRawScriptModule('inner_new', 'new at beginning of loop')

				// beforeFlow: loop with [inner_a, inner_b]
				// afterFlow: loop with [inner_new, inner_a, inner_b]
				const beforeLoop = createForloopModule('loop1', [deepClone(innerA), deepClone(innerB)])
				const afterLoop = createForloopModule('loop1', [
					deepClone(innerNew),
					deepClone(innerA),
					deepClone(innerB)
				])

				const beforeFlow = createExtendedOpenFlow({ modules: [beforeLoop] })
				const afterFlow: FlowValue = { modules: [afterLoop] }

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				expect(manager.moduleActions['inner_new']).toEqual({ action: 'added', pending: true })

				// Accept the added nested module
				manager.acceptModule('inner_new')
				flushSync()

				// Verify 'inner_new' is inserted at beginning inside the loop
				const beforeModules = manager.beforeFlow?.value.modules ?? []
				expect(beforeModules).toHaveLength(1)
				const loopModule = beforeModules[0]
				const loopModules = (loopModule.value as ForloopFlow).modules
				expectModuleOrder(loopModules, ['inner_new', 'inner_a', 'inner_b'])
			})
			cleanup()
		})

		it('reject removed module inside loop - restores at correct nested position', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const innerA = createRawScriptModule('inner_a', 'inner a')
				const innerB = createRawScriptModule('inner_b', 'inner b')
				const innerC = createRawScriptModule('inner_c', 'inner c')

				// beforeFlow: loop with [inner_a, inner_b, inner_c]
				// afterFlow: loop with [inner_a, inner_c] - inner_b removed
				const beforeLoop = createForloopModule('loop1', [
					deepClone(innerA),
					deepClone(innerB),
					deepClone(innerC)
				])
				const afterLoop = createForloopModule('loop1', [deepClone(innerA), deepClone(innerC)])

				const beforeFlow = createExtendedOpenFlow({ modules: [beforeLoop] })
				const currentFlowValue: FlowValue = { modules: [afterLoop] }
				const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(flowStore.val.value)
				flushSync()

				expect(manager.moduleActions['inner_b']).toEqual({ action: 'removed', pending: true })

				// Reject the removal - should restore 'inner_b' at original nested position
				manager.rejectModule('inner_b', flowStore)
				flushSync()

				// Verify 'inner_b' is restored at the middle position inside the loop
				const currentModules = flowStore.val.value.modules
				expect(currentModules).toHaveLength(1)
				const loopModule = currentModules[0]
				const loopModules = (loopModule.value as ForloopFlow).modules
				expectModuleOrder(loopModules, ['inner_a', 'inner_b', 'inner_c'])
			})
			cleanup()
		})

		it('accept multiple added modules at different positions', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const moduleNew1 = createRawScriptModule('new1', 'new at start')
				const moduleNew2 = createRawScriptModule('new2', 'new in middle')
				const moduleNew3 = createRawScriptModule('new3', 'new at end')

				// beforeFlow: [a, b]
				// afterFlow: [new1, a, new2, b, new3]
				const beforeFlow = createExtendedOpenFlow({
					modules: [deepClone(moduleA), deepClone(moduleB)]
				})
				const afterFlow: FlowValue = {
					modules: [
						deepClone(moduleNew1),
						deepClone(moduleA),
						deepClone(moduleNew2),
						deepClone(moduleB),
						deepClone(moduleNew3)
					]
				}

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				// Accept all added modules one by one
				manager.acceptModule('new1')
				flushSync()
				manager.acceptModule('new2')
				flushSync()
				manager.acceptModule('new3')
				flushSync()

				// Verify all modules are at correct positions
				const beforeModules = manager.beforeFlow?.value.modules ?? []
				expectModuleOrder(beforeModules, ['new1', 'a', 'new2', 'b', 'new3'])
			})
			cleanup()
		})

		it('reject multiple removed modules - restores all at correct positions', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'first')
				const moduleB = createRawScriptModule('b', 'second')
				const moduleC = createRawScriptModule('c', 'third')
				const moduleD = createRawScriptModule('d', 'fourth')
				const moduleE = createRawScriptModule('e', 'fifth')

				// beforeFlow: [a, b, c, d, e]
				// afterFlow: [b, d] - a, c, e removed
				const beforeFlow = createExtendedOpenFlow({
					modules: [
						deepClone(moduleA),
						deepClone(moduleB),
						deepClone(moduleC),
						deepClone(moduleD),
						deepClone(moduleE)
					]
				})

				const currentFlowValue: FlowValue = {
					modules: [deepClone(moduleB), deepClone(moduleD)]
				}
				const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(flowStore.val.value)
				flushSync()

				// Reject removals one by one
				manager.rejectModule('a', flowStore)
				flushSync()
				manager.rejectModule('c', flowStore)
				flushSync()
				manager.rejectModule('e', flowStore)
				flushSync()

				// Verify all modules are restored at correct positions
				const currentModules = flowStore.val.value.modules
				expectModuleOrder(currentModules, ['a', 'b', 'c', 'd', 'e'])
			})
			cleanup()
		})
	})

	describe('branch operations', () => {
		describe('branchone', () => {
			it('accept added module in default branch - inserts at correct position', () => {
				const cleanup = $effect.root(() => {
					const manager = createFlowDiffManager({ testMode: true })

					const defaultA = createRawScriptModule('default_a', 'default step a')
					const defaultB = createRawScriptModule('default_b', 'default step b')

					// beforeFlow: branch with [default_a] in default
					// afterFlow: branch with [default_a, default_b] in default
					const beforeBranch = createBranchOneModule('branch1', [deepClone(defaultA)], [])
					const afterBranch = createBranchOneModule(
						'branch1',
						[deepClone(defaultA), deepClone(defaultB)],
						[]
					)

					const beforeFlow = createExtendedOpenFlow({ modules: [beforeBranch] })
					const afterFlow: FlowValue = { modules: [afterBranch] }

					manager.setEditMode(true)
					manager.setBeforeFlow(beforeFlow)
					manager.setCurrentFlow(afterFlow)
					flushSync()

					expect(manager.moduleActions['default_b']).toEqual({ action: 'added', pending: true })

					// Accept the added module
					manager.acceptModule('default_b')
					flushSync()

					// Verify 'default_b' is inserted at correct position in default branch
					const beforeModules = manager.beforeFlow?.value.modules ?? []
					expect(beforeModules).toHaveLength(1)
					const branchModule = beforeModules[0]
					const branchDefault = (branchModule.value as BranchOne).default
					expectModuleOrder(branchDefault, ['default_a', 'default_b'])
				})
				cleanup()
			})

			it('reject removed module from default branch - restores at correct position', () => {
				const cleanup = $effect.root(() => {
					const manager = createFlowDiffManager({ testMode: true })

					const defaultA = createRawScriptModule('default_a', 'default step a')
					const defaultB = createRawScriptModule('default_b', 'default step b')
					const defaultC = createRawScriptModule('default_c', 'default step c')

					// beforeFlow: branch with [default_a, default_b, default_c] in default
					// afterFlow: branch with [default_a, default_c] in default - default_b removed
					const beforeBranch = createBranchOneModule(
						'branch1',
						[deepClone(defaultA), deepClone(defaultB), deepClone(defaultC)],
						[]
					)
					const afterBranch = createBranchOneModule(
						'branch1',
						[deepClone(defaultA), deepClone(defaultC)],
						[]
					)

					const beforeFlow = createExtendedOpenFlow({ modules: [beforeBranch] })
					const currentFlowValue: FlowValue = { modules: [afterBranch] }
					const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

					manager.setEditMode(true)
					manager.setBeforeFlow(beforeFlow)
					manager.setCurrentFlow(flowStore.val.value)
					flushSync()

					expect(manager.moduleActions['default_b']).toEqual({ action: 'removed', pending: true })

					// Reject the removal - should restore 'default_b' at original position
					manager.rejectModule('default_b', flowStore)
					flushSync()

					// Verify 'default_b' is restored at the middle position in default branch
					const currentModules = flowStore.val.value.modules
					expect(currentModules).toHaveLength(1)
					const branchModule = currentModules[0]
					const branchDefault = (branchModule.value as BranchOne).default
					expectModuleOrder(branchDefault, ['default_a', 'default_b', 'default_c'])
				})
				cleanup()
			})

			it('accept added module in conditional branch - inserts at correct position', () => {
				const cleanup = $effect.root(() => {
					const manager = createFlowDiffManager({ testMode: true })

					const branchModuleA = createRawScriptModule('branch_a', 'branch step a')
					const branchModuleB = createRawScriptModule('branch_b', 'branch step b')

					// beforeFlow: branch with [branch_a] in conditional branch
					// afterFlow: branch with [branch_a, branch_b] in conditional branch
					const beforeBranch = createBranchOneModule(
						'branch1',
						[],
						[{ expr: 'x > 0', modules: [deepClone(branchModuleA)] }]
					)
					const afterBranch = createBranchOneModule(
						'branch1',
						[],
						[{ expr: 'x > 0', modules: [deepClone(branchModuleA), deepClone(branchModuleB)] }]
					)

					const beforeFlow = createExtendedOpenFlow({ modules: [beforeBranch] })
					const afterFlow: FlowValue = { modules: [afterBranch] }

					manager.setEditMode(true)
					manager.setBeforeFlow(beforeFlow)
					manager.setCurrentFlow(afterFlow)
					flushSync()

					expect(manager.moduleActions['branch_b']).toEqual({ action: 'added', pending: true })

					// Accept the added module
					manager.acceptModule('branch_b')
					flushSync()

					// Verify 'branch_b' is inserted in the conditional branch
					const beforeModules = manager.beforeFlow?.value.modules ?? []
					expect(beforeModules).toHaveLength(1)
					const branchModule = beforeModules[0]
					const branches = (branchModule.value as BranchOne).branches
					expect(branches).toHaveLength(1)
					expectModuleOrder(branches[0].modules, ['branch_a', 'branch_b'])
				})
				cleanup()
			})

			it('reject removed module from conditional branch - restores at correct position', () => {
				const cleanup = $effect.root(() => {
					const manager = createFlowDiffManager({ testMode: true })

					const branchModuleA = createRawScriptModule('branch_a', 'branch step a')
					const branchModuleB = createRawScriptModule('branch_b', 'branch step b')

					// beforeFlow: branch with [branch_a, branch_b] in conditional branch
					// afterFlow: branch with [branch_a] in conditional branch - branch_b removed
					const beforeBranch = createBranchOneModule(
						'branch1',
						[],
						[{ expr: 'x > 0', modules: [deepClone(branchModuleA), deepClone(branchModuleB)] }]
					)
					const afterBranch = createBranchOneModule(
						'branch1',
						[],
						[{ expr: 'x > 0', modules: [deepClone(branchModuleA)] }]
					)

					const beforeFlow = createExtendedOpenFlow({ modules: [beforeBranch] })
					const currentFlowValue: FlowValue = { modules: [afterBranch] }
					const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

					manager.setEditMode(true)
					manager.setBeforeFlow(beforeFlow)
					manager.setCurrentFlow(flowStore.val.value)
					flushSync()

					expect(manager.moduleActions['branch_b']).toEqual({ action: 'removed', pending: true })

					// Reject the removal
					manager.rejectModule('branch_b', flowStore)
					flushSync()

					// Verify 'branch_b' is restored in the conditional branch
					const currentModules = flowStore.val.value.modules
					expect(currentModules).toHaveLength(1)
					const branchModule = currentModules[0]
					const branches = (branchModule.value as BranchOne).branches
					expect(branches).toHaveLength(1)
					expectModuleOrder(branches[0].modules, ['branch_a', 'branch_b'])
				})
				cleanup()
			})

			it('accept modified module in branch - updates content correctly', () => {
				const cleanup = $effect.root(() => {
					const manager = createFlowDiffManager({ testMode: true })

					const branchModuleBefore = createRawScriptModule('branch_a', 'original content')
					const branchModuleAfter = createRawScriptModule('branch_a', 'modified content')

					// beforeFlow: branch with original module
					// afterFlow: branch with modified module
					const beforeBranch = createBranchOneModule('branch1', [deepClone(branchModuleBefore)], [])
					const afterBranch = createBranchOneModule('branch1', [deepClone(branchModuleAfter)], [])

					const beforeFlow = createExtendedOpenFlow({ modules: [beforeBranch] })
					const afterFlow: FlowValue = { modules: [afterBranch] }

					manager.setEditMode(true)
					manager.setBeforeFlow(beforeFlow)
					manager.setCurrentFlow(afterFlow)
					flushSync()

					expect(manager.moduleActions['branch_a']).toEqual({ action: 'modified', pending: true })

					// Accept the modification
					manager.acceptModule('branch_a')
					flushSync()

					// Verify content is updated
					const beforeModules = manager.beforeFlow?.value.modules ?? []
					const branchModule = beforeModules[0]
					const branchDefault = (branchModule.value as BranchOne).default
					const moduleA = branchDefault.find((m) => m.id === 'branch_a')
					expect((moduleA?.value as RawScript).content).toBe('modified content')
				})
				cleanup()
			})

			it('reject entire conditional branch removal - restores the branch with its modules', () => {
				const cleanup = $effect.root(() => {
					const manager = createFlowDiffManager({ testMode: true })

					const defaultModule = createRawScriptModule('default_mod', 'default')
					const branch1ModuleA = createRawScriptModule('b1_a', 'branch 1 a')
					const branch2ModuleA = createRawScriptModule('b2_a', 'branch 2 a')
					const branch2ModuleB = createRawScriptModule('b2_b', 'branch 2 b')

					// beforeFlow: branch with 2 conditional branches
					// afterFlow: branch with only 1 conditional branch - second branch removed
					const beforeBranch = createBranchOneModule(
						'branch1',
						[deepClone(defaultModule)],
						[
							{ expr: 'x > 0', modules: [deepClone(branch1ModuleA)] },
							{ expr: 'x < 0', modules: [deepClone(branch2ModuleA), deepClone(branch2ModuleB)] }
						]
					)
					const afterBranch = createBranchOneModule(
						'branch1',
						[deepClone(defaultModule)],
						[{ expr: 'x > 0', modules: [deepClone(branch1ModuleA)] }]
					)

					const beforeFlow = createExtendedOpenFlow({ modules: [beforeBranch] })
					const currentFlowValue: FlowValue = { modules: [afterBranch] }
					const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

					manager.setEditMode(true)
					manager.setBeforeFlow(beforeFlow)
					manager.setCurrentFlow(flowStore.val.value)
					flushSync()

					// Both modules from the removed branch should be marked as removed
					expect(manager.moduleActions['b2_a']).toEqual({ action: 'removed', pending: true })
					expect(manager.moduleActions['b2_b']).toEqual({ action: 'removed', pending: true })

					// Reject both removals to restore the entire branch
					manager.rejectModule('b2_a', flowStore)
					flushSync()
					manager.rejectModule('b2_b', flowStore)
					flushSync()

					// Verify the second branch is restored with both modules
					const currentModules = flowStore.val.value.modules
					expect(currentModules).toHaveLength(1)
					const branchModule = currentModules[0]
					const branches = (branchModule.value as BranchOne).branches

					// Should have 2 branches again
					expect(branches).toHaveLength(2)
					expectModuleOrder(branches[0].modules, ['b1_a'])
					expectModuleOrder(branches[1].modules, ['b2_a', 'b2_b'])
				})
				cleanup()
			})
		})

		describe('branchall', () => {
			it('accept added module in parallel branch - inserts at correct position', () => {
				const cleanup = $effect.root(() => {
					const manager = createFlowDiffManager({ testMode: true })

					const parallel1A = createRawScriptModule('p1_a', 'parallel 1 a')
					const parallel2A = createRawScriptModule('p2_a', 'parallel 2 a')
					const parallel2B = createRawScriptModule('p2_b', 'parallel 2 b - new')

					// beforeFlow: branchall with [p1_a] and [p2_a]
					// afterFlow: branchall with [p1_a] and [p2_a, p2_b]
					const beforeBranchAll = createBranchAllModule('branchall1', [
						{ modules: [deepClone(parallel1A)] },
						{ modules: [deepClone(parallel2A)] }
					])
					const afterBranchAll = createBranchAllModule('branchall1', [
						{ modules: [deepClone(parallel1A)] },
						{ modules: [deepClone(parallel2A), deepClone(parallel2B)] }
					])

					const beforeFlow = createExtendedOpenFlow({ modules: [beforeBranchAll] })
					const afterFlow: FlowValue = { modules: [afterBranchAll] }

					manager.setEditMode(true)
					manager.setBeforeFlow(beforeFlow)
					manager.setCurrentFlow(afterFlow)
					flushSync()

					expect(manager.moduleActions['p2_b']).toEqual({ action: 'added', pending: true })

					// Accept the added module
					manager.acceptModule('p2_b')
					flushSync()

					// Verify 'p2_b' is inserted in the second parallel branch
					const beforeModules = manager.beforeFlow?.value.modules ?? []
					expect(beforeModules).toHaveLength(1)
					const branchAllModule = beforeModules[0]
					const branches = (branchAllModule.value as BranchAll).branches
					expect(branches).toHaveLength(2)
					expectModuleOrder(branches[0].modules, ['p1_a'])
					expectModuleOrder(branches[1].modules, ['p2_a', 'p2_b'])
				})
				cleanup()
			})

			it('reject removed module from parallel branch - restores at correct position', () => {
				const cleanup = $effect.root(() => {
					const manager = createFlowDiffManager({ testMode: true })

					const parallel1A = createRawScriptModule('p1_a', 'parallel 1 a')
					const parallel1B = createRawScriptModule('p1_b', 'parallel 1 b')
					const parallel2A = createRawScriptModule('p2_a', 'parallel 2 a')

					// beforeFlow: branchall with [p1_a, p1_b] and [p2_a]
					// afterFlow: branchall with [p1_a] and [p2_a] - p1_b removed
					const beforeBranchAll = createBranchAllModule('branchall1', [
						{ modules: [deepClone(parallel1A), deepClone(parallel1B)] },
						{ modules: [deepClone(parallel2A)] }
					])
					const afterBranchAll = createBranchAllModule('branchall1', [
						{ modules: [deepClone(parallel1A)] },
						{ modules: [deepClone(parallel2A)] }
					])

					const beforeFlow = createExtendedOpenFlow({ modules: [beforeBranchAll] })
					const currentFlowValue: FlowValue = { modules: [afterBranchAll] }
					const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

					manager.setEditMode(true)
					manager.setBeforeFlow(beforeFlow)
					manager.setCurrentFlow(flowStore.val.value)
					flushSync()

					expect(manager.moduleActions['p1_b']).toEqual({ action: 'removed', pending: true })

					// Reject the removal
					manager.rejectModule('p1_b', flowStore)
					flushSync()

					// Verify 'p1_b' is restored in the first parallel branch
					const currentModules = flowStore.val.value.modules
					expect(currentModules).toHaveLength(1)
					const branchAllModule = currentModules[0]
					const branches = (branchAllModule.value as BranchAll).branches
					expect(branches).toHaveLength(2)
					expectModuleOrder(branches[0].modules, ['p1_a', 'p1_b'])
					expectModuleOrder(branches[1].modules, ['p2_a'])
				})
				cleanup()
			})

			it('reject entire parallel branch removal - restores the branch with its modules', () => {
				const cleanup = $effect.root(() => {
					const manager = createFlowDiffManager({ testMode: true })

					const parallel1A = createRawScriptModule('p1_a', 'parallel 1 a')
					const parallel1B = createRawScriptModule('p1_b', 'parallel 1 b')
					const parallel2A = createRawScriptModule('p2_a', 'parallel 2 a')

					// beforeFlow: branchall with 2 parallel branches
					// afterFlow: branchall with only 1 parallel branch - second branch removed
					const beforeBranchAll = createBranchAllModule('branchall1', [
						{ modules: [deepClone(parallel1A), deepClone(parallel1B)] },
						{ modules: [deepClone(parallel2A)] }
					])
					const afterBranchAll = createBranchAllModule('branchall1', [
						{ modules: [deepClone(parallel1A), deepClone(parallel1B)] }
					])

					const beforeFlow = createExtendedOpenFlow({ modules: [beforeBranchAll] })
					const currentFlowValue: FlowValue = { modules: [afterBranchAll] }
					const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

					manager.setEditMode(true)
					manager.setBeforeFlow(beforeFlow)
					manager.setCurrentFlow(flowStore.val.value)
					flushSync()

					// Module from the removed branch should be marked as removed
					expect(manager.moduleActions['p2_a']).toEqual({ action: 'removed', pending: true })

					// Reject the removal to restore the entire branch
					manager.rejectModule('p2_a', flowStore)
					flushSync()

					// Verify the second parallel branch is restored
					const currentModules = flowStore.val.value.modules
					expect(currentModules).toHaveLength(1)
					const branchAllModule = currentModules[0]
					const branches = (branchAllModule.value as BranchAll).branches

					// Should have 2 branches again
					expect(branches).toHaveLength(2)
					expectModuleOrder(branches[0].modules, ['p1_a', 'p1_b'])
					expectModuleOrder(branches[1].modules, ['p2_a'])
				})
				cleanup()
			})
		})
	})

	describe('module movement', () => {
		it('accept module moved from root to loop - accepts the addition in loop', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const emptyLoop = createForloopModule('loop1', [])
				const loopWithB = createForloopModule('loop1', [deepClone(moduleB)])

				// beforeFlow: [a, b, loop(empty)]
				// afterFlow: [a, loop(b)]
				const beforeFlow = createExtendedOpenFlow({
					modules: [deepClone(moduleA), deepClone(moduleB), deepClone(emptyLoop)]
				})
				const afterFlow: FlowValue = {
					modules: [deepClone(moduleA), deepClone(loopWithB)]
				}

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				// 'b' should be detected as moved (removed from root, added in loop)
				expect(manager.moduleActions['b']).toEqual({ action: 'added', pending: true })
				expect(manager.moduleActions['old__b']).toEqual({ action: 'removed', pending: true })

				// Accept the added 'b' in loop
				manager.acceptModule('b')
				flushSync()

				// After accepting, beforeFlow should have 'b' inside the loop
				const beforeModules = manager.beforeFlow?.value.modules ?? []
				const loopModule = beforeModules.find((m) => m.id === 'loop1')
				const loopModules = (loopModule?.value as ForloopFlow).modules
				expect(loopModules.some((m) => m.id === 'b')).toBe(true)
			})
			cleanup()
		})

		it('accept module moved from root to loop - accepts the removal at root', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const emptyLoop = createForloopModule('loop1', [])
				const loopWithB = createForloopModule('loop1', [deepClone(moduleB)])

				// beforeFlow: [a, b, loop(empty)]
				// afterFlow: [a, loop(b)]
				const beforeFlow = createExtendedOpenFlow({
					modules: [deepClone(moduleA), deepClone(moduleB), deepClone(emptyLoop)]
				})
				const afterFlow: FlowValue = {
					modules: [deepClone(moduleA), deepClone(loopWithB)]
				}

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				// Accept the removed 'old__b' at root
				manager.acceptModule('old__b')
				flushSync()

				// After accepting removal, 'b' should no longer be at root in beforeFlow
				const beforeModules = manager.beforeFlow?.value.modules ?? []
				const rootIds = beforeModules.map((m) => m.id)
				expect(rootIds).not.toContain('b')
			})
			cleanup()
		})

		it('reject module moved from root to loop - rejects the addition', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const emptyLoop = createForloopModule('loop1', [])
				const loopWithB = createForloopModule('loop1', [deepClone(moduleB)])

				// beforeFlow: [a, b, loop(empty)]
				const beforeFlow = createExtendedOpenFlow({
					modules: [deepClone(moduleA), deepClone(moduleB), deepClone(emptyLoop)]
				})

				// currentFlow (flowStore): [a, loop(b)]
				const currentFlowValue: FlowValue = {
					modules: [deepClone(moduleA), deepClone(loopWithB)]
				}
				const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(flowStore.val.value)
				flushSync()

				// Reject the added 'b' in loop - should remove it from flowStore
				manager.rejectModule('b', flowStore)
				flushSync()

				// 'b' should no longer be in the loop in flowStore
				const currentModules = flowStore.val.value.modules
				const loopModule = currentModules.find((m) => m.id === 'loop1')
				const loopModules = (loopModule?.value as ForloopFlow).modules
				expect(loopModules.some((m) => m.id === 'b')).toBe(false)
			})
			cleanup()
		})

		it('reject module moved from root to loop - rejects the removal (restores at root)', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const emptyLoop = createForloopModule('loop1', [])
				const loopWithB = createForloopModule('loop1', [deepClone(moduleB)])

				// beforeFlow: [a, b, loop(empty)]
				const beforeFlow = createExtendedOpenFlow({
					modules: [deepClone(moduleA), deepClone(moduleB), deepClone(emptyLoop)]
				})

				// currentFlow (flowStore): [a, loop(b)]
				const currentFlowValue: FlowValue = {
					modules: [deepClone(moduleA), deepClone(loopWithB)]
				}
				const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(flowStore.val.value)
				flushSync()

				// Reject the removed 'old__b' - restores original at root, renames new to 'new__b'
				manager.rejectModule('old__b', flowStore)
				flushSync()

				// 'b' should be restored at root level in flowStore
				const currentModules = flowStore.val.value.modules
				const rootIds = currentModules.map((m) => m.id)
				expect(rootIds).toContain('b')

				// The module in the loop should now be 'new__b'
				const loopModule = currentModules.find((m) => m.id === 'loop1')
				const loopModules = (loopModule?.value as ForloopFlow).modules
				expect(loopModules.some((m) => m.id === `${NEW_MODULE_PREFIX}b`)).toBe(true)
			})
			cleanup()
		})

		it('accept module moved between branches', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'step a')

				// beforeFlow: branch with 'a' in first conditional branch
				const beforeBranch = createBranchOneModule(
					'branch1',
					[],
					[
						{ expr: 'x > 0', modules: [deepClone(moduleA)] },
						{ expr: 'x < 0', modules: [] }
					]
				)
				// afterFlow: branch with 'a' in second conditional branch
				const afterBranch = createBranchOneModule(
					'branch1',
					[],
					[
						{ expr: 'x > 0', modules: [] },
						{ expr: 'x < 0', modules: [deepClone(moduleA)] }
					]
				)

				const beforeFlow = createExtendedOpenFlow({ modules: [beforeBranch] })
				const afterFlow: FlowValue = { modules: [afterBranch] }

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)
				flushSync()

				// 'a' is moved - should be removed from branch 0 and added to branch 1
				expect(manager.moduleActions['a']).toEqual({ action: 'added', pending: true })
				expect(manager.moduleActions['old__a']).toEqual({ action: 'removed', pending: true })

				// Accept both the addition and removal
				manager.acceptModule('a')
				flushSync()
				manager.acceptModule('old__a')
				flushSync()

				// beforeFlow should now have 'a' in branch 1, not in branch 0
				const beforeModules = manager.beforeFlow?.value.modules ?? []
				const branchModule = beforeModules.find((m) => m.id === 'branch1')
				const branches = (branchModule?.value as BranchOne).branches
				expect(branches[0].modules.some((m) => m.id === 'a')).toBe(false)
				expect(branches[1].modules.some((m) => m.id === 'a')).toBe(true)
			})
			cleanup()
		})

		it('reject module moved between branches - restores original position', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleA = createRawScriptModule('a', 'step a')

				// beforeFlow: branch with 'a' in first conditional branch
				const beforeBranch = createBranchOneModule(
					'branch1',
					[],
					[
						{ expr: 'x > 0', modules: [deepClone(moduleA)] },
						{ expr: 'x < 0', modules: [] }
					]
				)
				// afterFlow: branch with 'a' in second conditional branch
				const afterBranch = createBranchOneModule(
					'branch1',
					[],
					[
						{ expr: 'x > 0', modules: [] },
						{ expr: 'x < 0', modules: [deepClone(moduleA)] }
					]
				)

				const beforeFlow = createExtendedOpenFlow({ modules: [beforeBranch] })
				const currentFlowValue: FlowValue = { modules: [afterBranch] }
				const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(flowStore.val.value)
				flushSync()

				// Reject the removal from branch 0 - restores original 'a', renames new to 'new__a'
				manager.rejectModule('old__a', flowStore)
				flushSync()

				// Accept the addition in branch 1
				manager.acceptModule('a')
				flushSync()

				// flowStore should now have 'a' in branch 0 (original), 'new__a' in branch 1 (renamed new)
				const currentModules = flowStore.val.value.modules
				const branchModule = currentModules.find((m) => m.id === 'branch1')
				const branches = (branchModule?.value as BranchOne).branches
				expect(branches[0].modules.some((m) => m.id === 'a')).toBe(true)
				expect(branches[1].modules.some((m) => m.id === `${NEW_MODULE_PREFIX}a`)).toBe(true)
			})
			cleanup()
		})

		it('prevents bug when rejecting moved module removal (branchall scenario)', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager({ testMode: true })

				const moduleR = createRawScriptModule('r', 'module r content')

				// beforeFlow: 'r' in branch 2
				const beforeBranchAll = createBranchAllModule('branchall1', [
					{ modules: [] }, // branch 0
					{ modules: [] }, // branch 1
					{ modules: [deepClone(moduleR)] } // branch 2
				])

				// currentFlow: 'r' moved to branch 0
				const afterBranchAll = createBranchAllModule('branchall1', [
					{ modules: [deepClone(moduleR)] }, // branch 0
					{ modules: [] }, // branch 1
					{ modules: [] } // branch 2
				])

				const beforeFlow = createExtendedOpenFlow({ modules: [beforeBranchAll] })
				const currentFlowValue: FlowValue = { modules: [afterBranchAll] }
				const flowStore = createFlowStore(createExtendedOpenFlow(currentFlowValue))

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(flowStore.val.value)
				flushSync()

				// Verify initial state: 'r' is added at branch 0, 'old__r' is removed from branch 2
				expect(manager.moduleActions['r']).toEqual({ action: 'added', pending: true })
				expect(manager.moduleActions['old__r']).toEqual({ action: 'removed', pending: true })

				// User rejects the removal - wants to keep module at original location (branch 2)
				manager.rejectModule('old__r', flowStore)
				flushSync()

				// After rejection, verify the current behavior:
				// 1. Original module should be restored at branch 2 with id 'r'
				// 2. New module at branch 0 should be renamed to 'new__r'
				const currentModules = flowStore.val.value.modules
				const branchAllModule = currentModules.find((m) => m.id === 'branchall1')
				const branches = (branchAllModule?.value as BranchAll).branches

				// Branch 0 should have 'new__r' (renamed new module)
				expect(branches[0].modules.some((m) => m.id === `${NEW_MODULE_PREFIX}r`)).toBe(true)

				// Branch 2 should have 'r' (restored original)
				expect(branches[2].modules.some((m) => m.id === 'r')).toBe(true)
			})
			cleanup()
		})
	})
})
