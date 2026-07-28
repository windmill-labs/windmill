import { describe, it, expect } from 'vitest'
import type { FlowModule } from '$lib/gen'
import { moduleSlot } from './moduleSlot'

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
