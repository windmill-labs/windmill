import { describe, it, expect, vi } from 'vitest'

// Mock modules that transitively import CSS/Monaco
vi.mock('monaco-editor', () => ({}))
vi.mock('@xyflow/svelte', () => ({}))
vi.mock('./renderers/nodes/AssetNode.svelte', () => ({
	assetDisplaysAsOutputInFlowGraph: () => false
}))
vi.mock('../modulesTest.svelte', () => ({}))

import { topologicalSort } from './graphBuilder.svelte'

describe('topologicalSort', () => {
	it('skips parent ids with no node instead of throwing', () => {
		const sorted = topologicalSort([
			{ id: 'a' },
			{ id: 'b', parentIds: ['a'] },
			{ id: 'orphan', parentIds: ['deleted'] }
		])

		expect(sorted.map((n) => n.id)).toEqual(['orphan', 'b', 'a'])
	})
})
