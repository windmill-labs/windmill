import { describe, it, expect } from 'vitest'
import { buildDiffTree, type AppRootMeta, type DiffTreeItem, type NavEntry } from './diffTree'

// A diff item identified by its storage `key` but placed in the tree by its
// friendly `structurePath` — mirroring how the drawer feeds raw-app files.
function item(key: string, structurePath: string): DiffTreeItem<string> {
	return { key, structurePath, data: key }
}

const noApps = new Map<string, AppRootMeta>()
const allOpen = () => true

function orderKeys(model: ReturnType<typeof buildDiffTree<string>>, isOpen = allOpen) {
	return model.order(isOpen).map((e) => e.key)
}

describe('buildDiffTree — structure', () => {
	it('groups items by their 2-segment scope', () => {
		const t = buildDiffTree([item('a', 'u/admin/foo'), item('b', 'u/admin/bar')], noApps)
		expect(t.root.children).toHaveLength(1)
		const scope = t.root.children[0]
		expect(scope.type).toBe('folder')
		if (scope.type !== 'folder') return
		expect(scope.fullPath).toBe('u/admin')
		expect(scope.isScope).toBe(true)
		expect(scope.children.map((c) => c.type)).toEqual(['file', 'file'])
	})

	it('nests deeper segments as folders and sorts folders-first then alphabetically', () => {
		const t = buildDiffTree(
			[item('z', 'u/admin/zeta'), item('a', 'u/admin/alpha'), item('s', 'u/admin/sub/leaf')],
			noApps
		)
		const scope = t.root.children[0]
		if (scope.type !== 'folder') throw new Error('expected scope folder')
		// folder ('sub') before files, files alphabetical
		expect(scope.children.map((c) => `${c.type}:${c.name}`)).toEqual([
			'folder:sub',
			'file:alpha',
			'file:zeta'
		])
	})

	it('keeps a single-segment item at the root with no scope', () => {
		const t = buildDiffTree([item('orphan', 'orphan')], noApps)
		expect(t.root.children).toHaveLength(1)
		expect(t.root.children[0]).toMatchObject({ type: 'file', name: 'orphan', key: 'orphan' })
	})

	it('app roots sort as items among leaves — same position before and after unwrap', () => {
		const apps = new Map<string, AppRootMeta>([['u/admin/beta_app', { summaryKey: 'raw_app/b' }]])
		// Unloaded: the app is a plain leaf between alpha and zeta.
		const before = buildDiffTree(
			[
				item('a', 'u/admin/alpha'),
				item('raw_app/b', 'u/admin/beta_app'),
				item('z', 'u/admin/zeta')
			],
			apps
		)
		const scopeBefore = before.root.children[0]
		if (scopeBefore.type !== 'folder') throw new Error('expected scope')
		expect(scopeBefore.children.map((c) => `${c.type}:${c.name}`)).toEqual([
			'file:alpha',
			'file:beta_app',
			'file:zeta'
		])
		// Loaded: the leaf is replaced by the app-root folder — same position,
		// not hoisted to the folder group.
		const after = buildDiffTree(
			[
				item('a', 'u/admin/alpha'),
				item('f', 'u/admin/beta_app/App.tsx'),
				item('z', 'u/admin/zeta')
			],
			apps
		)
		const scopeAfter = after.root.children[0]
		if (scopeAfter.type !== 'folder') throw new Error('expected scope')
		expect(scopeAfter.children.map((c) => `${c.type}:${c.name}`)).toEqual([
			'file:alpha',
			'folder:beta_app',
			'file:zeta'
		])
	})

	it('tags the folder matching an app root and leaves others untagged', () => {
		const apps = new Map<string, AppRootMeta>([
			['u/admin/my_app', { summaryKey: 'raw_app/x', summary: 'My app' }]
		])
		const t = buildDiffTree(
			[item('f1', 'u/admin/my_app/App.tsx'), item('f2', 'u/admin/other/x.ts')],
			apps
		)
		const scope = t.root.children[0]
		if (scope.type !== 'folder') throw new Error('expected scope')
		const myApp = scope.children.find((c) => c.type === 'folder' && c.fullPath === 'u/admin/my_app')
		const other = scope.children.find((c) => c.type === 'folder' && c.fullPath === 'u/admin/other')
		expect(myApp && myApp.type === 'folder' && myApp.app?.summary).toBe('My app')
		expect(other && other.type === 'folder' && other.app).toBeUndefined()
	})
})

describe('buildDiffTree — order(isOpen)', () => {
	const t = buildDiffTree(
		[item('a', 'u/admin/app/App.tsx'), item('b', 'u/admin/app/index.css')],
		new Map([['u/admin/app', { summaryKey: 'raw_app/app' }]])
	)

	it('lists scope → app folder → files when everything is open', () => {
		expect(orderKeys(t)).toEqual(['folder:u/admin', 'folder:u/admin/app', 'a', 'b'])
	})

	it('hides a folder’s children when it is collapsed', () => {
		const closedApp = (key: string) => key !== 'folder:u/admin/app'
		expect(orderKeys(t, closedApp)).toEqual(['folder:u/admin', 'folder:u/admin/app'])
	})
})

describe('buildDiffTree — parentKeyOf (ArrowLeft)', () => {
	// The regression that shipped: a raw-app file keyed on its STORAGE path
	// (`…/draft_<uuid>/…`) but placed by its FRIENDLY path. The parent must be the
	// friendly folder it actually nests under — not anything derived from the key.
	it('returns the friendly parent folder for a storage-keyed file', () => {
		const t = buildDiffTree(
			[item('raw_app_file/u/admin/draft_x/App.tsx', 'u/admin/my_app/App.tsx')],
			noApps
		)
		expect(t.parentKeyOf('raw_app_file/u/admin/draft_x/App.tsx')).toBe('folder:u/admin/my_app')
	})

	it('returns the immediate folder for a deeper file', () => {
		const t = buildDiffTree([item('k', 'u/admin/app/runnables/a')], noApps)
		expect(t.parentKeyOf('k')).toBe('folder:u/admin/app/runnables')
		expect(t.parentKeyOf('folder:u/admin/app/runnables')).toBe('folder:u/admin/app')
	})

	it('returns the scope folder for a file directly under it', () => {
		const t = buildDiffTree([item('k', 'u/alice/script')], noApps)
		expect(t.parentKeyOf('k')).toBe('folder:u/alice')
	})

	it('returns undefined for a scope folder and a root-level leaf', () => {
		const t = buildDiffTree([item('k', 'u/admin/app/App.tsx'), item('o', 'orphan')], noApps)
		expect(t.parentKeyOf('folder:u/admin')).toBeUndefined()
		expect(t.parentKeyOf('o')).toBeUndefined()
	})
})

describe('buildDiffTree — firstChildKeyOf (ArrowRight)', () => {
	it('returns the first child key (folders sort first)', () => {
		const t = buildDiffTree(
			[item('file', 'u/admin/app/App.tsx'), item('run', 'u/admin/app/runnables/a')],
			noApps
		)
		// 'runnables' folder sorts before the 'App.tsx' file
		expect(t.firstChildKeyOf('folder:u/admin/app')).toBe('folder:u/admin/app/runnables')
		expect(t.firstChildKeyOf('folder:u/admin/app/runnables')).toBe('run')
	})

	it('returns undefined for an unknown or empty folder', () => {
		const t = buildDiffTree([item('k', 'u/admin/app/App.tsx')], noApps)
		expect(t.firstChildKeyOf('folder:does/not/exist')).toBeUndefined()
	})
})
