import { describe, it, expect } from 'vitest'
import type { FlowModule } from '$lib/gen'
import { moduleSlot, savedModuleById } from './moduleSlot'

function mod(id: string, content = ''): FlowModule {
	return {
		id,
		value: { type: 'rawscript', language: 'bun', content, input_transforms: {} }
	} as FlowModule
}

describe('moduleSlot', () => {
	it('follows its step when a preceding sibling is deleted', () => {
		const modules = [mod('a'), mod('b'), mod('c')]
		const slot = moduleSlot(() => modules, 'b', modules[1])
		modules.splice(0, 1)
		expect(slot.get().id).toBe('b')
	})

	it('drops a write once its own step is gone, instead of hitting the step that took its place', () => {
		// This is the whole point: an editor flushes its trailing keystrokes on unmount, and
		// the delete has already spliced the array by then.
		const b = mod('b')
		const modules = [mod('a'), b, mod('c')]
		const slot = moduleSlot(() => modules, 'b', b)
		modules.splice(1, 1)

		const target = slot.get()
		;(target.value as { content: string }).content = 'typed into b'

		expect(target).toBe(b)
		expect(modules.map((m) => m.id)).toEqual(['a', 'c'])
		expect((modules[1].value as { content: string }).content).toBe('')
	})

	it('ignores a whole-module replacement aimed at a deleted step', () => {
		const b = mod('b')
		const modules = [mod('a'), b]
		const slot = moduleSlot(() => modules, 'b', b)
		modules.splice(1, 1)
		slot.set(mod('replacement'))
		expect(modules.map((m) => m.id)).toEqual(['a'])
	})
})

describe('savedModuleById', () => {
	function branchOne(id: string, branches: FlowModule[][]): FlowModule {
		return {
			id,
			value: {
				type: 'branchone',
				default: [],
				branches: branches.map((modules) => ({ expr: 'true', modules }))
			}
		} as unknown as FlowModule
	}

	it('finds a step whose branch has moved, where its old position now holds another step', () => {
		// The deployed flow keeps the order it was deployed in. Matching by position would
		// pair the step with the other branch's step and diff against the wrong code.
		const deployed = branchOne('a', [[mod('x', 'code-x')], [mod('y', 'code-y')]])

		const found = savedModuleById(deployed, 'y')

		expect((found?.value as { content: string }).content).toBe('code-y')
	})

	it('reaches into a loop nested in a branch', () => {
		const loop = {
			id: 'l',
			value: { type: 'forloopflow', modules: [mod('deep', 'code-deep')] }
		} as unknown as FlowModule
		const deployed = branchOne('a', [[loop]])

		expect((savedModuleById(deployed, 'deep')?.value as { content: string }).content).toBe(
			'code-deep'
		)
	})

	it('returns undefined for a step that has no deployed counterpart yet', () => {
		expect(savedModuleById(branchOne('a', [[mod('x')]]), 'brand-new')).toBeUndefined()
	})

	it('searches the deployed top-level module list when given one', () => {
		// Inserting a step at the top shifts every later one, so position and id disagree.
		const deployed = [mod('a', 'code-a'), mod('b', 'code-b')]

		expect((savedModuleById(deployed, 'b')?.value as { content: string }).content).toBe('code-b')
	})
})
