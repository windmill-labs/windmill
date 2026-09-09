import { describe, it, expect, vi } from 'vitest'

// Mock modules that transitively import CSS/Monaco
vi.mock('monaco-editor', () => ({}))
vi.mock('@xyflow/svelte', () => ({}))

import type { FlowModule, OpenFlow } from '$lib/gen'
import type { StateStore } from '$lib/utils'
import type { ExtendedOpenFlow } from '../flows/types'
import { NoteEditor } from './noteEditor.svelte'

function makeFlowStore(
	moduleIds: string[],
	containedNodeIds: string[]
): StateStore<ExtendedOpenFlow> {
	const modules: FlowModule[] = moduleIds.map((id) => ({
		id,
		value: { type: 'rawscript', content: '', language: 'bun' } as any
	}))
	const flow: OpenFlow = {
		summary: '',
		value: {
			modules,
			notes: [
				{
					id: 'note',
					text: 'note',
					color: 'yellow',
					type: 'group',
					contained_node_ids: containedNodeIds
				}
			]
		},
		schema: {}
	}
	return { val: flow as ExtendedOpenFlow } as StateStore<ExtendedOpenFlow>
}

function containedIds(flowStore: StateStore<ExtendedOpenFlow>): string[] | undefined {
	return flowStore.val.value.notes?.[0]?.contained_node_ids
}

describe('cleanupGroupNotes', () => {
	it('keeps a module that the graph has not rendered yet', () => {
		// A module id change rebuilds the graph in several passes, one of which has the
		// module under neither its old nor its new id.
		const flowStore = makeFlowStore(['renamed', 'b'], ['renamed', 'b'])
		new NoteEditor(flowStore).cleanupGroupNotes([{ id: 'b' }])

		expect(containedIds(flowStore)).toEqual(['renamed', 'b'])
	})

	it('drops a module that no longer exists in the flow', () => {
		const flowStore = makeFlowStore(['b'], ['deleted', 'b'])
		new NoteEditor(flowStore).cleanupGroupNotes([{ id: 'b' }])

		expect(containedIds(flowStore)).toEqual(['b'])
	})
})
