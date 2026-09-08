import { Folder, Layers, User } from 'lucide-svelte'
import {
	dirKey,
	KIND_LABEL,
	kindKey,
	leafKeyFor,
	workspaceItemDisplayPath,
	type WorkspaceItem,
	type WorkspaceItemKind
} from './workspacePicker'
import type { DrillBranch, DrillLeaf, DrillNode } from './drillPicker'

/** Intermediate path-hierarchy node — same shape as the previous
 * `buildTreeFromItems` output, kept internal because the DrillPicker
 * consumes `DrillNode`s instead. */
type DirNode = {
	fullPath: string
	name: string
	/** True for the top-level `f/<folder>` or `u/<user>` directories. */
	isScope: boolean
	children: DirNode[]
	leaves: WorkspaceItem[]
}

/** Build the path-hierarchy from a flat list of workspace items. Items are
 * placed by their display path, so a draft-only item shows up under its
 * friendly folder rather than the `u/<user>/draft_<uuid>` storage location. */
function buildDirForest(items: WorkspaceItem[]): DirNode[] {
	const scopeRoots = new Map<string, DirNode>()
	for (const it of items) {
		const parts = workspaceItemDisplayPath(it).split('/')
		if (parts.length < 3) continue
		const scopeFp = parts.slice(0, 2).join('/')
		let node = scopeRoots.get(scopeFp)
		if (!node) {
			node = { fullPath: scopeFp, name: scopeFp, isScope: true, children: [], leaves: [] }
			scopeRoots.set(scopeFp, node)
		}
		const slug = parts.slice(2)
		let cur = node
		for (let i = 0; i < slug.length - 1; i++) {
			const seg = slug[i]
			const fullPath = cur.fullPath + '/' + seg
			let next = cur.children.find((c) => c.name === seg)
			if (!next) {
				next = { fullPath, name: seg, isScope: false, children: [], leaves: [] }
				cur.children.push(next)
			}
			cur = next
		}
		cur.leaves.push(it)
	}
	const scopes = Array.from(scopeRoots.values()).sort((a, b) => {
		// `u/` (user) scopes before `f/` (folder) scopes; alphabetical within.
		const au = a.fullPath.startsWith('u/') ? 0 : 1
		const bu = b.fullPath.startsWith('u/') ? 0 : 1
		if (au !== bu) return au - bu
		return a.fullPath.localeCompare(b.fullPath)
	})
	const sortNode = (n: DirNode) => {
		n.children.sort((a, b) => a.name.localeCompare(b.name))
		n.leaves.sort((a, b) => workspaceItemDisplayPath(a).localeCompare(workspaceItemDisplayPath(b)))
		n.children.forEach(sortNode)
	}
	scopes.forEach(sortNode)
	return scopes
}

/** True when `p` names this item — its storage path or its friendly draft
 * path. A draft-only editor's live/saved paths are the friendly path while
 * the loaded row sits at the storage path, so matching on `path` alone would
 * treat them as two different items. */
function itemMatchesPath(it: WorkspaceItem, p: string | undefined): boolean {
	return p !== undefined && (it.path === p || it.draftPath === p)
}

/** Inject the currently-edited item at its live path, dropping the saved
 * entry when a draft rename is mid-flight. Only applies to items of the
 * same kind. */
function withCurrent(
	items: WorkspaceItem[],
	k: WorkspaceItemKind,
	currentItem: (WorkspaceItem & { savedPath?: string }) | undefined
): WorkspaceItem[] {
	if (!currentItem || currentItem.kind !== k) return items
	const drafted =
		currentItem.savedPath && currentItem.savedPath !== currentItem.path
			? items.filter((it) => !itemMatchesPath(it, currentItem.savedPath))
			: items
	if (drafted.some((it) => itemMatchesPath(it, currentItem.path))) return drafted
	return [
		...drafted,
		{
			path: currentItem.path,
			summary: currentItem.summary,
			kind: k,
			raw_app: currentItem.raw_app
		}
	]
}

function itemToLeaf(
	it: WorkspaceItem,
	currentItem: (WorkspaceItem & { savedPath?: string }) | undefined
): DrillLeaf<WorkspaceItem> {
	const isCurrent =
		!!currentItem && currentItem.kind === it.kind && itemMatchesPath(it, currentItem.path)
	const display = workspaceItemDisplayPath(it)
	return {
		type: 'leaf',
		key: leafKeyFor(it.kind, it.path),
		label: it.summary || display,
		secondary: it.summary ? display : undefined,
		data: it,
		current: isCurrent
	}
}

function dirToBranch(
	d: DirNode,
	scopeKind: WorkspaceItemKind | 'all',
	currentItem: (WorkspaceItem & { savedPath?: string }) | undefined
): DrillBranch<WorkspaceItem> {
	// Top-level user scope (`u/<user>`) gets a person icon. Everything
	// else (top-level `f/<folder>` or any deeper folder) is a folder.
	const isUserScope = d.isScope && d.fullPath.startsWith('u/')
	return {
		type: 'branch',
		key: dirKey(scopeKind, d.fullPath),
		label: d.name,
		icon: isUserScope ? User : Folder,
		children: [
			...d.children.map((c) => dirToBranch(c, scopeKind, currentItem)),
			...d.leaves.map((l) => itemToLeaf(l, currentItem))
		]
	}
}

/** Merge AI-created in-memory drafts (or any caller-provided extras) into a
 * kind's loaded list. The chat tools / session previews scaffold items via
 * `UserDraft` before the user deploys; those should be navigable from the
 * picker. An extra matching a loaded item (by storage or friendly path — else
 * one draft renders as two leaves) is folded into it: the loaded row wins on
 * backend metadata (summary etc.), but the extra's `draftPath` is overlaid
 * when set — a live editor cell knows a rename before the backend list does. */
function withExtras(
	items: WorkspaceItem[],
	k: WorkspaceItemKind,
	extraItemsByKind: Partial<Record<WorkspaceItemKind, WorkspaceItem[]>> | undefined
): WorkspaceItem[] {
	const extras = extraItemsByKind?.[k]
	if (!extras || extras.length === 0) return items
	const leftover = new Set(extras)
	const merged = items.map((it) => {
		const ex = extras.find((d) => itemMatchesPath(it, d.path) || itemMatchesPath(it, d.draftPath))
		if (!ex) return it
		leftover.delete(ex)
		return ex.draftPath !== undefined && ex.draftPath !== it.draftPath
			? { ...it, draftPath: ex.draftPath }
			: it
	})
	return leftover.size > 0 ? merged.concat([...leftover]) : merged
}

/** Build the workspace drill tree.
 *
 *  Default `by-kind` layout:
 *  - One branch per kind in `kinds` (`Flows` / `Scripts` / `Apps`),
 *    each containing the kind's path hierarchy.
 *  - When `kinds.length > 1`, prepend an `All` branch that merges items
 *    across kinds. The `All` branch is flagged `omitFromSearch` so its
 *    leaves don't appear twice in global-search results.
 *  - When `kinds.length === 1`, return the single kind branch's children
 *    directly so the user lands on folders without a redundant level.
 *
 *  `flat` layout:
 *  - No kind grouping — the root IS the workspace home's first level: the
 *    `f/<folder>` / `u/<user>` scope dirs of the cross-kind merge, mixing
 *    every kind's items inside. Callers must eager-load all kinds (there is
 *    no per-kind drill step left to lazy-load from).
 */
export function buildWorkspaceTree(opts: {
	loaded: Partial<Record<WorkspaceItemKind, WorkspaceItem[]>>
	kinds: WorkspaceItemKind[]
	currentItem?: WorkspaceItem & { savedPath?: string }
	/** Per-kind spinner flag. Defaults to `{}` — callers that don't track
	 * loading state (e.g. chat picker, which preloads eagerly) can omit it. */
	loadingKind?: Partial<Record<WorkspaceItemKind, boolean>>
	/** Per-kind extras to merge into the loaded list before tree-building
	 * (e.g. AI-created localStorage drafts surfaced by the workspace adapter).
	 * Extras whose path matches an already-loaded item are dropped. */
	extraItemsByKind?: Partial<Record<WorkspaceItemKind, WorkspaceItem[]>>
	layout?: 'by-kind' | 'flat'
}): DrillNode<WorkspaceItem>[] {
	const { loaded, kinds, currentItem, extraItemsByKind } = opts
	const loadingKind = opts.loadingKind ?? {}
	const layout = opts.layout ?? 'by-kind'

	function kindBranch(k: WorkspaceItemKind): DrillBranch<WorkspaceItem> {
		const raw = withExtras(loaded[k] ?? [], k, extraItemsByKind)
		const items = withCurrent(raw, k, currentItem)
		const dirs = items.length > 0 ? buildDirForest(items) : []
		return {
			type: 'branch',
			key: kindKey(k),
			label: KIND_LABEL[k],
			children: dirs.map((d) => dirToBranch(d, k, currentItem)),
			loading: !loaded[k] && !!loadingKind[k],
			// Search results from this kind group under its label (collapses
			// the folder hierarchy in the search view).
			searchGroup: true
		}
	}

	if (kinds.length === 0) return []

	const mergedItems = () =>
		kinds.flatMap((k) =>
			withCurrent(withExtras(loaded[k] ?? [], k, extraItemsByKind), k, currentItem)
		)

	if (layout === 'flat') {
		const items = mergedItems()
		const dirs = items.length > 0 ? buildDirForest(items) : []
		return dirs.map((d) => dirToBranch(d, 'all', currentItem))
	}

	if (kinds.length === 1) {
		return kindBranch(kinds[0]).children
	}

	// Cross-kind 'all' branch — flagged so search doesn't double-count leaves.
	const allItems = mergedItems()
	const allDirs = allItems.length > 0 ? buildDirForest(allItems) : []
	const allBranch: DrillBranch<WorkspaceItem> = {
		type: 'branch',
		key: kindKey('all'),
		label: 'All',
		icon: Layers,
		children: allDirs.map((d) => dirToBranch(d, 'all', currentItem)),
		omitFromSearch: true,
		loading: kinds.some((k) => !loaded[k] && !!loadingKind[k])
	}

	return [allBranch, ...kinds.map((k) => kindBranch(k))]
}

/** Map the legacy `{ kind, dir? }` initial-scope shape used by callers
 * (BreadcrumbSegment / EditorHeader) onto the new generic `string[]` path. */
export function legacyScopeToPath(
	scope: { kind: WorkspaceItemKind | 'all'; dir?: string } | undefined,
	kinds: WorkspaceItemKind[],
	layout: 'by-kind' | 'flat' = 'by-kind'
): string[] {
	if (!scope) return []
	// Flat layout: no kind branches at all — scope dirs live at root and are
	// always keyed under the cross-kind 'all' namespace.
	if (layout === 'flat') {
		return scope.dir ? [dirKey('all', scope.dir)] : []
	}
	// Single-kind mode: there's no kind branch at root; scope's `kind` is
	// implicit. Only the dir (if any) makes it to the path.
	if (kinds.length === 1) {
		return scope.dir ? [dirKey(scope.kind, scope.dir)] : []
	}
	const path: string[] = [kindKey(scope.kind)]
	if (scope.dir) path.push(dirKey(scope.kind, scope.dir))
	return path
}

/** Return `absolutePath` shortened to its segment relative to the deepest
 * `dir:<kind>:<path>` segment in `scope`. Used to render leaf rows like
 * `parquet_etl` instead of `f/examples/parquet_etl` once the user has
 * drilled into `f/examples`. Falls back to the absolute path when no dir
 * scope matches (e.g. at the kind level, or when the leaf isn't actually
 * under the scoped dir). */
export function relativizeWorkspacePath(absolutePath: string, scope: string[]): string {
	for (let i = scope.length - 1; i >= 0; i--) {
		const k = scope[i]
		if (!k.startsWith('dir:')) continue
		const rest = k.slice(4) // '<kind>:<path>'
		const colon = rest.indexOf(':')
		if (colon < 0) continue
		const dirPath = rest.slice(colon + 1)
		if (absolutePath.startsWith(dirPath + '/')) {
			return absolutePath.slice(dirPath.length + 1)
		}
		return absolutePath
	}
	return absolutePath
}
