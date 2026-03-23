import { describe, it, expect, vi } from 'vitest'

// Mock modules that transitively import CSS/Monaco
vi.mock('monaco-editor', () => ({}))
vi.mock('@xyflow/svelte', () => ({}))
vi.mock('./renderers/nodes/AssetNode.svelte', () => ({
	assetDisplaysAsOutputInFlowGraph: () => false
}))
vi.mock('../modulesTest.svelte', () => ({}))

import type { GraphGroup } from './groupEditor.svelte'
import type { FlowModule } from '$lib/gen'
import {
	buildStructureTree,
	flattenStructureIds,
	deriveGroupsFromStructure,
	collectLeafIds,
	findInStructure
} from './flowStructure'

function makeModule(id: string): FlowModule {
	return {
		id,
		value: { type: 'rawscript', content: '', language: 'python3' } as any
	} as FlowModule
}

function makeForloop(id: string, innerIds: string[]): FlowModule {
	return {
		id,
		value: {
			type: 'forloopflow',
			modules: innerIds.map((iid) => makeModule(iid)),
			iterator: { type: 'javascript', expr: '' }
		} as any
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

describe('buildStructureTree', () => {
	const modules = [makeModule('a'), makeModule('b'), makeModule('c')]

	it('builds structure tree for a valid group', () => {
		const groups = [makeGroup('g1', 'a', 'b', ['a', 'b'])]
		const result = buildStructureTree(modules, groups)
		// Should have a group node + the remaining leaf 'c'
		expect(result).toHaveLength(2)
		expect(result[0].kind).toBe('group')
		expect(result[0].id).toBe('g1')
		expect(result[0].branches[0].children).toHaveLength(2)
		expect(result[1].kind).toBe('leaf')
		expect(result[1].id).toBe('c')
	})

	it('throws on inverted range (start_id after end_id)', () => {
		const groups = [makeGroup('g1', 'c', 'a', ['a', 'b', 'c'])]
		expect(() => buildStructureTree(modules, groups)).toThrow(/inverted range/i)
	})

	it('throws on partially overlapping groups', () => {
		const groups = [makeGroup('g1', 'a', 'b', ['a', 'b']), makeGroup('g2', 'b', 'c', ['b', 'c'])]
		expect(() => buildStructureTree(modules, groups)).toThrow(/overlap without nesting/i)
	})

	it('throws when group start_id is a virtual node (Input)', () => {
		const groups = [makeGroup('g1', 'Input', 'b', ['a', 'b'])]
		expect(() => buildStructureTree(modules, groups)).toThrow(/virtual node/i)
	})

	it('throws when group end_id is a virtual node (Result)', () => {
		const groups = [makeGroup('g1', 'a', 'Result', ['a', 'b', 'c'])]
		expect(() => buildStructureTree(modules, groups)).toThrow(/virtual node/i)
	})

	it('throws when group references Trigger', () => {
		const groups = [makeGroup('g1', 'Trigger', 'c', ['a', 'b', 'c'])]
		expect(() => buildStructureTree(modules, groups)).toThrow(/virtual node/i)
	})

	it('allows fully nested groups', () => {
		const mods = [makeModule('a'), makeModule('b'), makeModule('c'), makeModule('d')]
		const groups = [
			makeGroup('outer', 'a', 'd', ['a', 'b', 'c', 'd']),
			makeGroup('inner', 'b', 'c', ['b', 'c'])
		]
		const result = buildStructureTree(mods, groups)
		expect(result).toHaveLength(1) // outer group contains everything
		expect(result[0].kind).toBe('group')
		// Inner group should be nested
		const outerChildren = result[0].branches[0].children
		expect(outerChildren).toHaveLength(3) // a, inner-group, d
		expect(outerChildren[1].kind).toBe('group')
		expect(outerChildren[1].id).toBe('inner')
	})

	it('handles empty modules', () => {
		const result = buildStructureTree([], [])
		expect(result).toHaveLength(0)
	})

	it('handles container modules (forloop)', () => {
		const mods = [makeForloop('loop', ['x', 'y']), makeModule('c')]
		const result = buildStructureTree(mods, [])
		expect(result).toHaveLength(2)
		expect(result[0].kind).toBe('forloopflow')
		expect(result[0].branches).toHaveLength(1)
		expect(result[0].branches[0].children).toHaveLength(2)
		expect(result[0].branches[0].children[0].id).toBe('x')
	})

	it('handles groups inside containers', () => {
		const mods = [makeForloop('loop', ['x', 'y', 'z'])]
		const groups = [makeGroup('g1', 'x', 'y', ['x', 'y'])]
		const result = buildStructureTree(mods, groups)
		expect(result).toHaveLength(1)
		expect(result[0].kind).toBe('forloopflow')
		const innerChildren = result[0].branches[0].children
		expect(innerChildren).toHaveLength(2) // group + z
		expect(innerChildren[0].kind).toBe('group')
		expect(innerChildren[0].id).toBe('g1')
	})
})

describe('flattenStructureIds', () => {
	it('flattens a simple tree', () => {
		const modules = [makeModule('a'), makeModule('b'), makeModule('c')]
		const groups = [makeGroup('g1', 'a', 'b', ['a', 'b'])]
		const tree = buildStructureTree(modules, groups)
		const ids = flattenStructureIds(tree)
		expect(ids).toEqual(['a', 'b', 'c'])
	})

	it('flattens nested groups', () => {
		const mods = [makeModule('a'), makeModule('b'), makeModule('c'), makeModule('d')]
		const groups = [
			makeGroup('outer', 'a', 'd', ['a', 'b', 'c', 'd']),
			makeGroup('inner', 'b', 'c', ['b', 'c'])
		]
		const tree = buildStructureTree(mods, groups)
		const ids = flattenStructureIds(tree)
		expect(ids).toEqual(['a', 'b', 'c', 'd'])
	})
})

describe('deriveGroupsFromStructure', () => {
	it('derives group definitions with correct start/end', () => {
		const modules = [makeModule('a'), makeModule('b'), makeModule('c')]
		const groups = [makeGroup('g1', 'a', 'b', ['a', 'b'])]
		const tree = buildStructureTree(modules, groups)
		const derived = deriveGroupsFromStructure(tree)
		expect(derived).toHaveLength(1)
		expect(derived[0].id).toBe('g1')
		expect(derived[0].start_id).toBe('a')
		expect(derived[0].end_id).toBe('b')
	})

	it('derives nested groups', () => {
		const mods = [makeModule('a'), makeModule('b'), makeModule('c'), makeModule('d')]
		const groups = [
			makeGroup('outer', 'a', 'd', ['a', 'b', 'c', 'd']),
			makeGroup('inner', 'b', 'c', ['b', 'c'])
		]
		const tree = buildStructureTree(mods, groups)
		const derived = deriveGroupsFromStructure(tree)
		expect(derived).toHaveLength(2)
		expect(derived[0].id).toBe('outer')
		expect(derived[0].start_id).toBe('a')
		expect(derived[0].end_id).toBe('d')
		expect(derived[1].id).toBe('inner')
		expect(derived[1].start_id).toBe('b')
		expect(derived[1].end_id).toBe('c')
	})
})

describe('findInStructure', () => {
	it('finds a leaf node', () => {
		const modules = [makeModule('a'), makeModule('b')]
		const tree = buildStructureTree(modules, [])
		const found = findInStructure(tree, 'b')
		expect(found).toBeDefined()
		expect(found!.index).toBe(1)
	})

	it('finds a node inside a group', () => {
		const modules = [makeModule('a'), makeModule('b'), makeModule('c')]
		const groups = [makeGroup('g1', 'a', 'b', ['a', 'b'])]
		const tree = buildStructureTree(modules, groups)
		const found = findInStructure(tree, 'b')
		expect(found).toBeDefined()
		expect(found!.index).toBe(1)
		// parentChildren should be the group's branch children
		expect(found!.parentChildren).toHaveLength(2)
	})

	it('finds a group node by group id', () => {
		const modules = [makeModule('a'), makeModule('b')]
		const groups = [makeGroup('g1', 'a', 'b', ['a', 'b'])]
		const tree = buildStructureTree(modules, groups)
		const found = findInStructure(tree, 'g1')
		expect(found).toBeDefined()
		expect(found!.index).toBe(0)
	})
})

describe('collectLeafIds', () => {
	it('collects all leaf module IDs including inside containers', () => {
		const mods = [makeForloop('loop', ['x', 'y']), makeModule('c')]
		const tree = buildStructureTree(mods, [])
		const ids = collectLeafIds(tree)
		expect(ids).toEqual(['loop', 'x', 'y', 'c'])
	})
})
