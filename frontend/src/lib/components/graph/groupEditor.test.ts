import { describe, it, expect, vi } from 'vitest'

// Mock modules that transitively import CSS/Monaco
vi.mock('monaco-editor', () => ({}))
vi.mock('@xyflow/svelte', () => ({}))
vi.mock('./renderers/nodes/AssetNode.svelte', () => ({
	assetDisplaysAsOutputInFlowGraph: () => false
}))
vi.mock('../modulesTest.svelte', () => ({}))

import { buildGroupedModules, type GraphGroup } from './groupEditor.svelte'
import type { FlowModule } from '$lib/gen'

function makeModule(id: string): FlowModule {
	return {
		id,
		value: { type: 'rawscript', content: '', language: 'python3' } as any
	} as FlowModule
}

function makeGroup(
	id: string,
	start_id: string,
	end_id: string,
	moduleIds: string[] = []
): GraphGroup {
	return { id, start_id, end_id, moduleIds }
}

describe('buildGroupedModules', () => {
	const modules = [makeModule('a'), makeModule('b'), makeModule('c')]

	it('builds grouped modules for a valid group', () => {
		const groups = [makeGroup('g1', 'a', 'b', ['a', 'b'])]
		const result = buildGroupedModules(modules, groups)
		// Should have a group item + the remaining module 'c'
		expect(result).toHaveLength(2)
	})

	it('throws on inverted range (start_id after end_id)', () => {
		const groups = [makeGroup('g1', 'c', 'a', ['a', 'b', 'c'])]
		expect(() => buildGroupedModules(modules, groups)).toThrow(/inverted range/i)
	})

	it('throws on partially overlapping groups', () => {
		const groups = [makeGroup('g1', 'a', 'b', ['a', 'b']), makeGroup('g2', 'b', 'c', ['b', 'c'])]
		expect(() => buildGroupedModules(modules, groups)).toThrow(/overlap without nesting/i)
	})

	it('throws when group start_id is a virtual node (Input)', () => {
		const groups = [makeGroup('g1', 'Input', 'b', ['a', 'b'])]
		expect(() => buildGroupedModules(modules, groups)).toThrow(/virtual node/i)
	})

	it('throws when group end_id is a virtual node (Result)', () => {
		const groups = [makeGroup('g1', 'a', 'Result', ['a', 'b', 'c'])]
		expect(() => buildGroupedModules(modules, groups)).toThrow(/virtual node/i)
	})

	it('throws when group references Trigger', () => {
		const groups = [makeGroup('g1', 'Trigger', 'c', ['a', 'b', 'c'])]
		expect(() => buildGroupedModules(modules, groups)).toThrow(/virtual node/i)
	})

	it('allows fully nested groups', () => {
		const mods = [makeModule('a'), makeModule('b'), makeModule('c'), makeModule('d')]
		const groups = [
			makeGroup('outer', 'a', 'd', ['a', 'b', 'c', 'd']),
			makeGroup('inner', 'b', 'c', ['b', 'c'])
		]
		const result = buildGroupedModules(mods, groups)
		expect(result).toHaveLength(1) // outer group contains everything
	})
})
