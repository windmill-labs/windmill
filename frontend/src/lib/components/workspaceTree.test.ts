import { describe, it, expect } from 'vitest'
import { buildWorkspaceTree, legacyScopeToPath, relativizeWorkspacePath } from './workspaceTree'
import {
	dirKey,
	kindKey,
	leafKeyFor,
	type WorkspaceItem,
	type WorkspaceItemKind
} from './workspacePicker'
import type { DrillBranch, DrillLeaf, DrillNode } from './drillPicker'

const item = (
	kind: WorkspaceItemKind,
	path: string,
	summary?: string,
	raw_app?: boolean
): WorkspaceItem => ({ kind, path, summary: summary ?? '', raw_app })

const isBranch = <L>(n: DrillNode<L> | undefined): n is DrillBranch<L> => !!n && n.type === 'branch'
const isLeaf = <L>(n: DrillNode<L> | undefined): n is DrillLeaf<L> => !!n && n.type === 'leaf'

const childKeys = <L>(b: DrillBranch<L>) => b.children.map((c) => c.key)
const findBranch = <L>(nodes: DrillNode<L>[], key: string): DrillBranch<L> => {
	const n = nodes.find((x) => x.key === key)
	if (!isBranch(n)) throw new Error(`expected branch ${key} in [${nodes.map((x) => x.key)}]`)
	return n
}

describe('buildWorkspaceTree', () => {
	describe('shape', () => {
		it('returns an empty tree when kinds is empty', () => {
			expect(buildWorkspaceTree({ loaded: {}, kinds: [], loadingKind: {} })).toEqual([])
		})

		it('multi-kind: prepends an All branch then per-kind branches', () => {
			const tree = buildWorkspaceTree({
				loaded: {
					flow: [item('flow', 'f/demo/a')],
					script: [item('script', 'f/demo/b')]
				},
				kinds: ['flow', 'script'],
				loadingKind: {}
			})
			expect(tree.map((n) => n.key)).toEqual([kindKey('all'), kindKey('flow'), kindKey('script')])
		})

		it('All branch is omitFromSearch and labeled "All"', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [item('flow', 'f/demo/a')], script: [] },
				kinds: ['flow', 'script'],
				loadingKind: {}
			})
			const all = findBranch(tree, kindKey('all'))
			expect(all.omitFromSearch).toBe(true)
			expect(all.label).toBe('All')
		})

		it('per-kind branches are searchGroup anchors', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [item('flow', 'f/demo/a')], script: [] },
				kinds: ['flow', 'script'],
				loadingKind: {}
			})
			const flow = findBranch(tree, kindKey('flow'))
			expect(flow.searchGroup).toBe(true)
		})

		it("single-kind: returns that kind branch's children directly (no kind-level)", () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [item('flow', 'f/demo/a'), item('flow', 'u/alice/b')] },
				kinds: ['flow'],
				loadingKind: {}
			})
			// At the top we should see the scope dirs (u/alice, f/demo) directly,
			// not a single 'kind:flow' branch wrapping them.
			expect(tree.every((n) => isBranch(n) && n.key.startsWith('dir:flow:'))).toBe(true)
			// u-scopes come before f-scopes
			expect(tree.map((n) => n.key)).toEqual([dirKey('flow', 'u/alice'), dirKey('flow', 'f/demo')])
		})
	})

	describe('loading state', () => {
		it('per-kind branch is loading=true when loaded[k] is undefined and loadingKind[k] is true', () => {
			const tree = buildWorkspaceTree({
				loaded: {},
				kinds: ['flow', 'script'],
				loadingKind: { flow: true }
			})
			const flow = findBranch(tree, kindKey('flow'))
			expect(flow.loading).toBe(true)
		})

		it('per-kind branch is not loading once loaded[k] is set, even mid-refetch', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [] },
				kinds: ['flow', 'script'],
				loadingKind: { flow: true }
			})
			const flow = findBranch(tree, kindKey('flow'))
			expect(flow.loading).toBeFalsy()
		})

		it('All branch is loading when any kind is loading', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [] },
				kinds: ['flow', 'script'],
				loadingKind: { script: true }
			})
			const all = findBranch(tree, kindKey('all'))
			expect(all.loading).toBe(true)
		})
	})

	describe('dir forest', () => {
		it('groups leaves under their scope, then nested folders', () => {
			const tree = buildWorkspaceTree({
				loaded: {
					flow: [
						item('flow', 'f/demo/a'),
						item('flow', 'f/demo/sub/b'),
						item('flow', 'f/demo/sub/c'),
						item('flow', 'u/alice/d')
					]
				},
				kinds: ['flow'],
				loadingKind: {}
			})
			// Top-level: u/alice (user scope), f/demo (folder scope)
			expect(tree.map((n) => n.key)).toEqual([dirKey('flow', 'u/alice'), dirKey('flow', 'f/demo')])
			const demo = findBranch(tree, dirKey('flow', 'f/demo'))
			// Children: nested folder `sub` first, then leaf `a`
			expect(childKeys(demo)).toEqual([
				dirKey('flow', 'f/demo/sub'),
				leafKeyFor('flow', 'f/demo/a')
			])
			const sub = findBranch(demo.children, dirKey('flow', 'f/demo/sub'))
			expect(childKeys(sub)).toEqual([
				leafKeyFor('flow', 'f/demo/sub/b'),
				leafKeyFor('flow', 'f/demo/sub/c')
			])
		})

		it('skips items with paths shorter than 3 segments', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [item('flow', 'f/demo'), item('flow', 'f/demo/a')] },
				kinds: ['flow'],
				loadingKind: {}
			})
			const demo = findBranch(tree, dirKey('flow', 'f/demo'))
			expect(childKeys(demo)).toEqual([leafKeyFor('flow', 'f/demo/a')])
		})
	})

	describe('leaf shape', () => {
		it('uses summary as label and path as secondary when summary is present', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [item('flow', 'f/demo/a', 'Hello')] },
				kinds: ['flow'],
				loadingKind: {}
			})
			const demo = findBranch(tree, dirKey('flow', 'f/demo'))
			const leaf = demo.children[0]
			if (!isLeaf(leaf)) throw new Error('expected leaf')
			expect(leaf.label).toBe('Hello')
			expect(leaf.secondary).toBe('f/demo/a')
		})

		it('falls back to path as label when summary is empty', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [item('flow', 'f/demo/a')] },
				kinds: ['flow'],
				loadingKind: {}
			})
			const demo = findBranch(tree, dirKey('flow', 'f/demo'))
			const leaf = demo.children[0]
			if (!isLeaf(leaf)) throw new Error('expected leaf')
			expect(leaf.label).toBe('f/demo/a')
			expect(leaf.secondary).toBeUndefined()
		})

		it('marks the currentItem leaf with current=true', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [item('flow', 'f/demo/a'), item('flow', 'f/demo/b')] },
				kinds: ['flow'],
				loadingKind: {},
				currentItem: item('flow', 'f/demo/a')
			})
			const demo = findBranch(tree, dirKey('flow', 'f/demo'))
			const [a, b] = demo.children
			if (!isLeaf(a) || !isLeaf(b)) throw new Error('expected leaves')
			expect(a.current).toBe(true)
			expect(b.current).toBeFalsy()
		})
	})

	describe('withCurrent: rename suppression', () => {
		it('injects currentItem at its live path when not already in the list', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [] },
				kinds: ['flow'],
				loadingKind: {},
				currentItem: { ...item('flow', 'f/demo/new'), summary: 'My Flow' }
			})
			const demo = findBranch(tree, dirKey('flow', 'f/demo'))
			expect(demo.children.map((c) => c.key)).toEqual([leafKeyFor('flow', 'f/demo/new')])
		})

		it('drops the savedPath entry during a mid-rename so only the live one shows', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [item('flow', 'f/demo/old', 'My Flow')] },
				kinds: ['flow'],
				loadingKind: {},
				currentItem: { ...item('flow', 'f/demo/new', 'My Flow'), savedPath: 'f/demo/old' }
			})
			const demo = findBranch(tree, dirKey('flow', 'f/demo'))
			const paths = demo.children.map((c) => c.key)
			expect(paths).toContain(leafKeyFor('flow', 'f/demo/new'))
			expect(paths).not.toContain(leafKeyFor('flow', 'f/demo/old'))
		})

		it('does not re-inject when the live entry already exists in loaded', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [item('flow', 'f/demo/a', 'Original')] },
				kinds: ['flow'],
				loadingKind: {},
				currentItem: item('flow', 'f/demo/a', 'Original')
			})
			const demo = findBranch(tree, dirKey('flow', 'f/demo'))
			expect(demo.children.length).toBe(1)
		})

		it('passes other-kind items through untouched', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [item('flow', 'f/demo/a')], script: [item('script', 'f/demo/b')] },
				kinds: ['flow', 'script'],
				loadingKind: {},
				currentItem: { ...item('flow', 'f/demo/new'), savedPath: 'f/demo/old' }
			})
			const script = findBranch(tree, kindKey('script'))
			const demo = findBranch(script.children, dirKey('script', 'f/demo'))
			expect(demo.children.map((c) => c.key)).toEqual([leafKeyFor('script', 'f/demo/b')])
		})
	})

	describe('extraItemsByKind (drafts)', () => {
		it('merges extras alongside loaded items', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [item('flow', 'f/demo/a')] },
				kinds: ['flow'],
				loadingKind: {},
				extraItemsByKind: { flow: [item('flow', 'f/demo/draft')] }
			})
			const demo = findBranch(tree, dirKey('flow', 'f/demo'))
			expect(demo.children.map((c) => c.key).sort()).toEqual(
				[leafKeyFor('flow', 'f/demo/a'), leafKeyFor('flow', 'f/demo/draft')].sort()
			)
		})

		it('drops extras whose path collides with a loaded item (loaded wins)', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [item('flow', 'f/demo/a', 'Backend summary')] },
				kinds: ['flow'],
				loadingKind: {},
				extraItemsByKind: { flow: [item('flow', 'f/demo/a', 'Draft summary')] }
			})
			const demo = findBranch(tree, dirKey('flow', 'f/demo'))
			expect(demo.children.length).toBe(1)
			const leaf = demo.children[0]
			if (!isLeaf(leaf)) throw new Error('expected leaf')
			expect(leaf.label).toBe('Backend summary')
		})

		it('extras flow into the cross-kind All branch too', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [], script: [item('script', 'f/demo/b')] },
				kinds: ['flow', 'script'],
				loadingKind: {},
				extraItemsByKind: { flow: [item('flow', 'f/demo/draft')] }
			})
			const all = findBranch(tree, kindKey('all'))
			const demo = findBranch(all.children, dirKey('all', 'f/demo'))
			const keys = demo.children.map((c) => c.key)
			expect(keys).toContain(leafKeyFor('flow', 'f/demo/draft'))
			expect(keys).toContain(leafKeyFor('script', 'f/demo/b'))
		})

		it('is a no-op when extras are absent or empty', () => {
			const noOpts = buildWorkspaceTree({
				loaded: { flow: [item('flow', 'f/demo/a')] },
				kinds: ['flow'],
				loadingKind: {}
			})
			const emptyExtras = buildWorkspaceTree({
				loaded: { flow: [item('flow', 'f/demo/a')] },
				kinds: ['flow'],
				loadingKind: {},
				extraItemsByKind: { flow: [] }
			})
			expect(JSON.stringify(noOpts)).toEqual(JSON.stringify(emptyExtras))
		})
	})

	describe('flat layout', () => {
		it('roots the cross-kind scope dirs directly (no All / kind branches)', () => {
			const tree = buildWorkspaceTree({
				loaded: {
					flow: [item('flow', 'f/demo/a'), item('flow', 'u/alice/b')],
					script: [item('script', 'f/demo/c')]
				},
				kinds: ['flow', 'script'],
				loadingKind: {},
				layout: 'flat'
			})
			// u-scopes before f-scopes, keyed under the 'all' namespace.
			expect(tree.map((n) => n.key)).toEqual([dirKey('all', 'u/alice'), dirKey('all', 'f/demo')])
		})

		it('mixes every kind inside the same scope dir', () => {
			const tree = buildWorkspaceTree({
				loaded: {
					flow: [item('flow', 'f/demo/a')],
					script: [item('script', 'f/demo/b')]
				},
				kinds: ['flow', 'script'],
				loadingKind: {},
				layout: 'flat'
			})
			const demo = findBranch(tree, dirKey('all', 'f/demo'))
			expect(childKeys(demo)).toEqual([
				leafKeyFor('flow', 'f/demo/a'),
				leafKeyFor('script', 'f/demo/b')
			])
		})

		it('applies extras and currentItem like the by-kind layout', () => {
			const tree = buildWorkspaceTree({
				loaded: { flow: [item('flow', 'f/demo/a')], script: [] },
				kinds: ['flow', 'script'],
				loadingKind: {},
				extraItemsByKind: { script: [item('script', 'f/demo/draft')] },
				currentItem: item('flow', 'f/demo/a'),
				layout: 'flat'
			})
			const demo = findBranch(tree, dirKey('all', 'f/demo'))
			const leaves = demo.children.filter(isLeaf)
			expect(leaves.map((l) => l.key)).toEqual([
				leafKeyFor('flow', 'f/demo/a'),
				leafKeyFor('script', 'f/demo/draft')
			])
			expect(leaves[0].current).toBe(true)
		})
	})
})

describe('legacyScopeToPath', () => {
	it('returns [] for undefined scope', () => {
		expect(legacyScopeToPath(undefined, ['flow', 'script'])).toEqual([])
	})

	it('multi-kind: returns [kindKey] for a kind-only scope', () => {
		expect(legacyScopeToPath({ kind: 'flow' }, ['flow', 'script'])).toEqual([kindKey('flow')])
	})

	it('multi-kind: returns [kindKey, dirKey] for a kind+dir scope', () => {
		expect(legacyScopeToPath({ kind: 'flow', dir: 'f/demo' }, ['flow', 'script'])).toEqual([
			kindKey('flow'),
			dirKey('flow', 'f/demo')
		])
	})

	it('multi-kind: handles `all` as a kind', () => {
		expect(legacyScopeToPath({ kind: 'all', dir: 'f/demo' }, ['flow', 'script'])).toEqual([
			kindKey('all'),
			dirKey('all', 'f/demo')
		])
	})

	it('single-kind: returns [] for a kind-only scope (no kind level in tree)', () => {
		expect(legacyScopeToPath({ kind: 'flow' }, ['flow'])).toEqual([])
	})

	it('single-kind: returns [dirKey] for a kind+dir scope', () => {
		expect(legacyScopeToPath({ kind: 'flow', dir: 'f/demo' }, ['flow'])).toEqual([
			dirKey('flow', 'f/demo')
		])
	})

	it('flat: returns [dirKey under all] for a dir scope, ignoring the kind', () => {
		expect(legacyScopeToPath({ kind: 'flow', dir: 'f/demo' }, ['flow', 'script'], 'flat')).toEqual([
			dirKey('all', 'f/demo')
		])
	})

	it('flat: returns [] without a dir', () => {
		expect(legacyScopeToPath({ kind: 'all' }, ['flow', 'script'], 'flat')).toEqual([])
	})
})

describe('relativizeWorkspacePath', () => {
	it('returns the absolute path when scope has no dir segment', () => {
		expect(relativizeWorkspacePath('f/demo/a', [])).toBe('f/demo/a')
		expect(relativizeWorkspacePath('f/demo/a', [kindKey('flow')])).toBe('f/demo/a')
	})

	it('shortens to the path relative to the deepest dir scope', () => {
		const scope = [kindKey('flow'), dirKey('flow', 'f/demo')]
		expect(relativizeWorkspacePath('f/demo/a', scope)).toBe('a')
	})

	it('uses the DEEPEST dir scope when there are nested ones', () => {
		const scope = [kindKey('flow'), dirKey('flow', 'f/demo'), dirKey('flow', 'f/demo/sub')]
		expect(relativizeWorkspacePath('f/demo/sub/b', scope)).toBe('b')
	})

	it('falls back to absolute path when the leaf is not under the dir scope', () => {
		const scope = [kindKey('flow'), dirKey('flow', 'f/demo')]
		expect(relativizeWorkspacePath('f/other/a', scope)).toBe('f/other/a')
	})
})
