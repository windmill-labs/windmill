import { describe, it, expect } from 'vitest'
import { writable, get } from 'svelte/store'
import type { FlowModule } from '$lib/gen'
import type { StateStore } from '$lib/utils'
import type { ExtendedOpenFlow } from './types'
import { reorderBranches } from './branchOps'

function branch(expr: string) {
	return { summary: expr, expr, modules: [] as FlowModule[] }
}

function ctx(branches: ReturnType<typeof branch>[]) {
	const flow = {
		summary: '',
		value: {
			modules: [{ id: 'a', value: { type: 'branchone', branches, default: [] } } as FlowModule]
		}
	} as ExtendedOpenFlow
	const flowStore: StateStore<ExtendedOpenFlow> = { val: flow }
	const history = writable({ history: [] as ExtendedOpenFlow[], index: -1 })
	const read = () =>
		(flowStore.val.value.modules[0].value as { branches: ReturnType<typeof branch>[] }).branches
	return { flowStore, history, read }
}

describe('reorderBranches', () => {
	it('commits the new order', () => {
		const [x, y, z] = [branch('x'), branch('y'), branch('z')]
		const { flowStore, history, read } = ctx([x, y, z])

		reorderBranches('a', [z, x, y], { flowStore, history })

		expect(read().map((b) => b.expr)).toEqual(['z', 'x', 'y'])
	})

	it('records an undo entry, so a drag can be undone like an add or a delete', () => {
		const [x, y] = [branch('x'), branch('y')]
		const { flowStore, history } = ctx([x, y])

		reorderBranches('a', [y, x], { flowStore, history })

		expect(get(history).history).toHaveLength(1)
	})

	it('spends nothing when the drag lands where it started', () => {
		// `finalize` fires on every drop, so a no-op drag must not consume an undo step.
		const [x, y] = [branch('x'), branch('y')]
		const { flowStore, history, read } = ctx([x, y])

		reorderBranches('a', [x, y], { flowStore, history })

		expect(get(history).history).toHaveLength(0)
		expect(read().map((b) => b.expr)).toEqual(['x', 'y'])
	})
})
