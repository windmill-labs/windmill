// Pure file-tree model for the diff drawer (WorkspaceDiffDrawer).
//
// Turns a flat list of diff items into a navigable folder tree and answers the
// keyboard-navigation queries the drawer needs (visible order, parent, first
// child). It is generic over the leaf payload `T` on purpose: the drawer's
// `DiffRow` type lives in WorkspaceDiffDrawer, which its two adapters import —
// importing it here would make a cycle.
//
// Items are grouped by "scope" (the first two path segments, e.g. `f/foo` or
// `u/alice`); deeper segments become nested folders. Folder keys are
// `folder:<fullPath>`. The crucial invariant: every structural answer
// (parent/first-child/order) comes from the tree as it was *built*, never from
// re-splitting a path at the call site — so a node's tree position and its
// navigation parent can never drift apart.
//
// Two path spaces meet here. `structurePath` decides tree placement (it's the
// friendly path, so a never-deployed draft nests under `u/me/my_app` rather
// than `u/me/draft_<uuid>`); `key` is the caller's stable identity (the storage
// key — used for the load cache, the row id and `data-nav-key`). Structure code
// only ever reads `structurePath`; it carries `key` through opaquely.

export type DiffTreeItem<T> = {
	/** Stable identity (storage key). Becomes the file node's key + nav key. */
	key: string
	/** Friendly path that decides where the item sits in the tree. */
	structurePath: string
	data: T
}

/** Marks a folder as a raw app's root so the drawer renders its header as the
 * raw-app row instead of a plain folder. Keyed by the app's friendly path. */
export type AppRootMeta = {
	summaryKey: string
	summary?: string
	hasDraft?: boolean
	draftOnly?: boolean
	draftUsers?: { username?: string | null }[]
	draftItemKind?: import('$lib/gen').UserDraftItemKind
}

export type FolderNode<T> = {
	type: 'folder'
	name: string
	fullPath: string
	isScope: boolean
	key: string
	children: TreeNode<T>[]
	app?: AppRootMeta
}
export type FileNode<T> = { type: 'file'; name: string; key: string; data: T }
export type TreeNode<T> = FolderNode<T> | FileNode<T>

export type NavEntry<T> =
	| { type: 'folder'; key: string; node: FolderNode<T> }
	| { type: 'file'; key: string; data: T }

export type DiffTreeModel<T> = {
	root: FolderNode<T>
	/** Visible entries top-to-bottom, honoring the caller's fold state. */
	order(isOpen: (key: string) => boolean): NavEntry<T>[]
	/** Folder key containing `key`, or undefined at the top (no parent folder). */
	parentKeyOf(key: string): string | undefined
	/** Key of a folder's first child (ArrowRight target), or undefined if empty. */
	firstChildKeyOf(folderKey: string): string | undefined
}

export function folderKeyFor(fullPath: string): string {
	return `folder:${fullPath}`
}

export function buildDiffTree<T>(
	items: DiffTreeItem<T>[],
	appMeta: Map<string, AppRootMeta>
): DiffTreeModel<T> {
	const root: FolderNode<T> = {
		type: 'folder',
		name: '',
		fullPath: '',
		isScope: false,
		key: folderKeyFor(''),
		children: []
	}
	const folderCache = new Map<string, FolderNode<T>>()
	const nodeByKey = new Map<string, FolderNode<T>>()
	// child key → parent folder key. Direct children of `root` are intentionally
	// absent (they have no navigable parent), so parentKeyOf returns undefined
	// for scope folders and root-level leaves — matching ArrowLeft's old behavior.
	const parentByKey = new Map<string, string>()

	function getFolder(parent: FolderNode<T>, fullPath: string, name: string, isScope: boolean) {
		let f = folderCache.get(fullPath)
		if (!f) {
			f = { type: 'folder', name, fullPath, isScope, key: folderKeyFor(fullPath), children: [] }
			folderCache.set(fullPath, f)
			nodeByKey.set(f.key, f)
			parent.children.push(f)
			if (parent !== root) parentByKey.set(f.key, parent.key)
		}
		return f
	}

	function addFile(parent: FolderNode<T>, name: string, it: DiffTreeItem<T>) {
		parent.children.push({ type: 'file', name, key: it.key, data: it.data })
		if (parent !== root) parentByKey.set(it.key, parent.key)
	}

	for (const it of items) {
		const parts = it.structurePath.split('/')
		if (parts.length < 2) {
			addFile(root, it.structurePath, it)
			continue
		}
		const scopeKey = parts.slice(0, 2).join('/')
		const scope = getFolder(root, scopeKey, scopeKey, true)
		if (parts.length === 2) {
			addFile(scope, parts[1], it)
			continue
		}
		const rest = parts.slice(2)
		let parent = scope
		let fkey = scopeKey
		for (let i = 0; i < rest.length - 1; i++) {
			fkey = `${fkey}/${rest[i]}`
			parent = getFolder(parent, fkey, rest[i], false)
		}
		addFile(parent, rest[rest.length - 1], it)
	}

	// Tag the folder matching each raw app's friendly path as its app root.
	// Must happen before sorting: app roots sort as items, not as folders.
	for (const [fp, meta] of appMeta) {
		const f = folderCache.get(fp)
		if (f) f.app = meta
	}

	// Path folders group first; everything item-like — leaves AND app roots —
	// sorts together by name, so a raw app keeps its position when expanding
	// turns its leaf row into an app-root folder.
	const isGrouping = (n: TreeNode<T>) => n.type === 'folder' && !n.app
	const sortRec = (n: FolderNode<T>) => {
		n.children.sort((a, b) => {
			const ga = isGrouping(a) ? 0 : 1
			const gb = isGrouping(b) ? 0 : 1
			if (ga !== gb) return ga - gb
			return a.name.localeCompare(b.name)
		})
		for (const c of n.children) if (c.type === 'folder') sortRec(c)
	}
	sortRec(root)

	return {
		root,
		order(isOpen) {
			const out: NavEntry<T>[] = []
			const walk = (n: TreeNode<T>) => {
				if (n.type === 'file') {
					out.push({ type: 'file', key: n.key, data: n.data })
					return
				}
				out.push({ type: 'folder', key: n.key, node: n })
				if (isOpen(n.key)) for (const c of n.children) walk(c)
			}
			for (const c of root.children) walk(c)
			return out
		},
		parentKeyOf(key) {
			return parentByKey.get(key)
		},
		firstChildKeyOf(folderKey) {
			return nodeByKey.get(folderKey)?.children[0]?.key
		}
	}
}
