import { describe, it, expect } from 'vitest'
import {
	collectLeavesGrouped,
	leafHaystack,
	resolveScope,
	scopeChain,
	type DrillBranch,
	type DrillLeaf,
	type DrillNode
} from './drillPicker'

const leaf = (key: string, label = key, secondary?: string): DrillLeaf<string> => ({
	type: 'leaf',
	key,
	label,
	secondary,
	data: key
})

const branch = (
	key: string,
	children: DrillNode<string>[],
	opts: { label?: string; omitFromSearch?: boolean; searchGroup?: boolean } = {}
): DrillBranch<string> => ({
	type: 'branch',
	key,
	label: opts.label ?? key,
	children,
	omitFromSearch: opts.omitFromSearch,
	searchGroup: opts.searchGroup
})

describe('resolveScope', () => {
	const tree: DrillNode<string>[] = [
		branch('a', [branch('a.x', [leaf('a.x.1')]), leaf('a.2')]),
		branch('b', [leaf('b.1')]),
		leaf('top')
	]

	it('returns null at the root (empty scope)', () => {
		expect(resolveScope(tree, [])).toBeNull()
	})

	it('returns the branch at a one-level scope', () => {
		expect(resolveScope(tree, ['a'])?.key).toBe('a')
	})

	it('returns the branch at a nested scope', () => {
		expect(resolveScope(tree, ['a', 'a.x'])?.key).toBe('a.x')
	})

	it('returns null when any segment is missing', () => {
		expect(resolveScope(tree, ['a', 'missing'])).toBeNull()
		expect(resolveScope(tree, ['nope'])).toBeNull()
	})

	it('returns null when a segment resolves to a leaf (not a branch)', () => {
		expect(resolveScope(tree, ['top'])).toBeNull()
		expect(resolveScope(tree, ['a', 'a.2'])).toBeNull()
	})
})

describe('scopeChain', () => {
	const tree: DrillNode<string>[] = [
		branch('a', [branch('a.x', [leaf('a.x.1')]), leaf('a.2')]),
		branch('b', [leaf('b.1')])
	]

	it('returns [] at the root', () => {
		expect(scopeChain(tree, [])).toEqual([])
	})

	it('returns one branch for a one-level scope', () => {
		const chain = scopeChain(tree, ['a'])
		expect(chain.map((b) => b.key)).toEqual(['a'])
	})

	it('returns each branch along the path for a nested scope', () => {
		const chain = scopeChain(tree, ['a', 'a.x'])
		expect(chain.map((b) => b.key)).toEqual(['a', 'a.x'])
	})

	it('stops at the first missing/non-branch segment', () => {
		const chain = scopeChain(tree, ['a', 'a.2', 'never-reached'])
		expect(chain.map((b) => b.key)).toEqual(['a'])
	})
})

describe('collectLeavesGrouped', () => {
	it('flattens all leaves with null group when no branch has searchGroup', () => {
		const tree: DrillNode<string>[] = [branch('a', [leaf('a.1')]), leaf('top')]
		const result = collectLeavesGrouped(tree)
		expect(result.map((r) => [r.leaf.key, r.group?.key])).toEqual([
			['a.1', undefined],
			['top', undefined]
		])
	})

	it('groups leaves under their nearest searchGroup ancestor', () => {
		const tree: DrillNode<string>[] = [
			branch('flows', [branch('flows-folder', [leaf('flows-folder.1')]), leaf('flows.root')], {
				searchGroup: true
			})
		]
		const result = collectLeavesGrouped(tree)
		expect(result.map((r) => [r.leaf.key, r.group?.key])).toEqual([
			['flows-folder.1', 'flows'],
			['flows.root', 'flows']
		])
	})

	it('the DEEPEST searchGroup wins when nested', () => {
		const tree: DrillNode<string>[] = [
			branch('outer', [branch('inner', [leaf('deep')], { searchGroup: true })], {
				searchGroup: true
			})
		]
		const result = collectLeavesGrouped(tree)
		expect(result[0].group?.key).toBe('inner')
	})

	it('skips branches marked omitFromSearch entirely', () => {
		const tree: DrillNode<string>[] = [
			branch('all', [leaf('shared')], { omitFromSearch: true }),
			branch('flows', [leaf('shared'), leaf('uniq')], { searchGroup: true })
		]
		const result = collectLeavesGrouped(tree)
		// `all` branch is skipped, so `shared` is only seen once and grouped under `flows`.
		expect(result.map((r) => [r.leaf.key, r.group?.key])).toEqual([
			['shared', 'flows'],
			['uniq', 'flows']
		])
	})

	it('deduplicates leaves by key (first occurrence wins)', () => {
		// Simulate the workspace 'All' branch (omitFromSearch=true) plus per-kind
		// branches having the same leaf — even without omitFromSearch the dedup
		// would still guarantee no double-counting if the search tree changes.
		const tree: DrillNode<string>[] = [
			branch('flows', [leaf('a')], { searchGroup: true }),
			branch('scripts', [leaf('a')], { searchGroup: true })
		]
		const result = collectLeavesGrouped(tree)
		expect(result.length).toBe(1)
		expect(result[0].group?.key).toBe('flows')
	})

	it('handles a mix of top-level leaves and branches', () => {
		const tree: DrillNode<string>[] = [
			leaf('root-leaf'),
			branch('b', [leaf('b.1')], { searchGroup: true })
		]
		const result = collectLeavesGrouped(tree)
		expect(result.map((r) => [r.leaf.key, r.group?.key])).toEqual([
			['root-leaf', undefined],
			['b.1', 'b']
		])
	})
})

describe('leafHaystack', () => {
	it('uses searchableText when present (overrides label/secondary)', () => {
		expect(leafHaystack({ ...leaf('k', 'Label'), searchableText: 'custom' })).toBe('custom')
	})

	it('joins label and secondary with parens when both are present', () => {
		expect(leafHaystack(leaf('k', 'My Flow', 'f/demo/my_flow'))).toBe('My Flow (f/demo/my_flow)')
	})

	it('uses just label when secondary is absent', () => {
		expect(leafHaystack(leaf('k', 'just label'))).toBe('just label')
	})

	it('falls back to secondary when label is empty', () => {
		expect(leafHaystack({ type: 'leaf', key: 'k', label: '', secondary: 'sec', data: 'd' })).toBe(
			'sec'
		)
	})

	it('returns the empty string when nothing is set', () => {
		expect(leafHaystack({ type: 'leaf', key: 'k', label: '', data: 'd' })).toBe('')
	})
})
