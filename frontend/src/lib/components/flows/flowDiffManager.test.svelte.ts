import { describe, it, expect } from 'vitest'
import { flushSync } from 'svelte'
import { createFlowDiffManager } from './flowDiffManager.svelte'
import { DUPLICATE_MODULE_PREFIX } from './flowDiff'
import type { FlowModule, FlowValue, RawScript, ForloopFlow } from '$lib/gen'
import type { ExtendedOpenFlow } from './types'
import type { StateStore } from '$lib/utils'
import { SPECIAL_MODULE_IDS } from '../copilot/chat/shared'

// Helper: create a simple rawscript module
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

// Helper: create a forloop module with nested modules
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

// Helper: wrap FlowValue in ExtendedOpenFlow
function createExtendedOpenFlow(
	flowValue: FlowValue,
	schema?: Record<string, any>
): ExtendedOpenFlow {
	return { value: flowValue, summary: '', schema }
}

// Helper: create a minimal StateStore
function createFlowStore(flow: ExtendedOpenFlow): StateStore<ExtendedOpenFlow> {
	return { val: flow }
}

// Helper: deep clone an object
function deepClone<T>(obj: T): T {
	return JSON.parse(JSON.stringify(obj))
}

describe('FlowDiffManager', () => {
	describe('effect auto-computation', () => {
		it('auto-computes moduleActions when flows differ', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager()

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
				const manager = createFlowDiffManager()

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
				const manager = createFlowDiffManager()

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
				const manager = createFlowDiffManager()

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
				const manager = createFlowDiffManager()

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
				const manager = createFlowDiffManager()

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
				const manager = createFlowDiffManager()

				const moduleA = createRawScriptModule('a', 'content')
				const beforeFlow = createExtendedOpenFlow({ modules: [moduleA] })
				const afterFlow: FlowValue = { modules: [moduleA] }

				manager.setEditMode(true)
				manager.setBeforeFlow(beforeFlow)
				manager.setCurrentFlow(afterFlow)

				// Manually set an action with the duplicate prefix (simulating type change)
				manager.setModuleActions({
					[`${DUPLICATE_MODULE_PREFIX}a`]: { action: 'removed', pending: true }
				})
				flushSync()

				// Accept should strip the prefix and process 'a'
				manager.acceptModule(`${DUPLICATE_MODULE_PREFIX}a`)
				flushSync()

				// The module 'a' should be removed from beforeFlow
				const beforeModules = manager.beforeFlow?.value.modules ?? []
				expect(beforeModules.map((m) => m.id)).not.toContain('a')
			})
			cleanup()
		})
	})

	describe('rejectModule', () => {
		it('rejects added module - removes from flowStore', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager()

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const beforeFlow = createExtendedOpenFlow({ modules: [deepClone(moduleA)] })

				const currentFlowValue: FlowValue = {
					modules: [deepClone(moduleA), deepClone(moduleB)]
				}
				const flowStore = createFlowStore(
					createExtendedOpenFlow(currentFlowValue)
				)

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
				const manager = createFlowDiffManager()

				const moduleA = createRawScriptModule('a', 'content-a')
				const moduleB = createRawScriptModule('b', 'content-b')
				const beforeFlow = createExtendedOpenFlow({
					modules: [deepClone(moduleA), deepClone(moduleB)]
				})

				const currentFlowValue: FlowValue = { modules: [deepClone(moduleA)] }
				const flowStore = createFlowStore(
					createExtendedOpenFlow(currentFlowValue)
				)

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
				const manager = createFlowDiffManager()

				const moduleBeforeA = createRawScriptModule('a', 'original-content')
				const moduleAfterA = createRawScriptModule('a', 'modified-content')
				const beforeFlow = createExtendedOpenFlow({ modules: [deepClone(moduleBeforeA)] })

				const currentFlowValue: FlowValue = { modules: [deepClone(moduleAfterA)] }
				const flowStore = createFlowStore(
					createExtendedOpenFlow(currentFlowValue)
				)

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
				const manager = createFlowDiffManager()

				const beforeSchema = { properties: { x: { type: 'string' } } }
				const afterSchema = { properties: { x: { type: 'string' }, y: { type: 'number' } } }
				const beforeFlow = createExtendedOpenFlow({ modules: [] }, beforeSchema)

				const flowStore = createFlowStore(
					createExtendedOpenFlow({ modules: [] }, afterSchema)
				)

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
				const manager = createFlowDiffManager()

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
				const manager = createFlowDiffManager()

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
	})

	describe('batch operations', () => {
		it('acceptAll accepts all pending modules', () => {
			const cleanup = $effect.root(() => {
				const manager = createFlowDiffManager()

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
				const manager = createFlowDiffManager()

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
				const manager = createFlowDiffManager()

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
				const manager = createFlowDiffManager()

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
				const manager = createFlowDiffManager()

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
})
