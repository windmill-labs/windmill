import { describe, it, expect } from 'vitest'
import { buildFlowTimeline } from './flowDiff'
import type { FlowValue, FlowModule, RawScript, Identity } from '$lib/gen'

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

// Helper to create a FlowValue with modules
function createFlow(modules: FlowModule[]): FlowValue {
	return { modules }
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

			expect(result.mergedFlow.modules).toHaveLength(2)
			expect(result.mergedFlow.modules?.map((m) => m.id)).toContain('a')
			expect(result.mergedFlow.modules?.map((m) => m.id)).toContain('b')
		})

		it('mergedFlow includes removed modules from beforeFlow', () => {
			const moduleA = createRawScriptModule('a', 'console.log("a")')
			const moduleB = createRawScriptModule('b', 'console.log("b")')
			const beforeFlow = createFlow([moduleA, moduleB])
			const afterFlow = createFlow([moduleA])

			const result = buildFlowTimeline(beforeFlow, afterFlow)

			expect(result.mergedFlow.modules).toHaveLength(2)
			expect(result.mergedFlow.modules?.map((m) => m.id)).toContain('a')
			expect(result.mergedFlow.modules?.map((m) => m.id)).toContain('b')
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
})
