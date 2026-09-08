import type { Component, ComponentType } from 'svelte'

/** Icon constructor accepted by the picker — covers Svelte-5 `Component` and
 * legacy `ComponentType` (lucide icons resolve to the former, but other
 * callers in the repo still hand in the latter, see `TriggersBadge.svelte`). */
export type DrillIcon = ComponentType | Component<any, {}, ''>

/** Leaf node — terminal entry the user picks. The picker emits the leaf
 * back via `onPick` so callers can react with the original `data` payload. */
export type DrillLeaf<L> = {
	type: 'leaf'
	key: string
	/** Primary line. */
	label: string
	/** Optional secondary line (e.g. full path). */
	secondary?: string
	/** Lucide-style component rendered with `size={12}`. The picker also
	 * accepts a `leafIcon` snippet override that gets the whole leaf. */
	icon?: DrillIcon
	data: L
	/** Optional override for the fuzzy-search haystack. Defaults to
	 * `label` (or `secondary` when label is empty). */
	searchableText?: string
	/** Category header this leaf renders under, in both the browse list and
	 * search results (a `searchGroup` branch ancestor wins in search).
	 * Consecutive leaves sharing a section share one header. */
	section?: string
	/** Marks this leaf as the user's current location — gets `aria-current`
	 * and a styled, no-op click. */
	current?: boolean
	/** When true, leaf is rendered but disabled (greyed + no-op click). */
	disabled?: boolean
}

/** Branch node — interior entry the user drills into. */
export type DrillBranch<L> = {
	type: 'branch'
	key: string
	label: string
	icon?: DrillIcon
	children: DrillNode<L>[]
	/** Show a spinner alongside the branch (async loading in progress). */
	loading?: boolean
	/** Hide from search index traversal. Used by the workspace adapter to
	 * keep the cross-kind 'all' branch out of search (its leaves are
	 * duplicates of the per-kind branches' leaves). */
	omitFromSearch?: boolean
	/** When true, leaves under this branch are grouped under its label in
	 * the search-results display. The DEEPEST such ancestor wins. Used to
	 * collapse folder hierarchies into kind/section headers — e.g. a leaf
	 * at `Workspace > Flows > f/demo > foo` groups under "Flows" (not
	 * "f/demo"). */
	searchGroup?: boolean
}

export type DrillNode<L> = DrillBranch<L> | DrillLeaf<L>

/** Walk the tree to the branch at the given scope path. Returns null at
 * root (empty scope) or when any segment doesn't resolve to a branch. */
export function resolveScope<L>(tree: DrillNode<L>[], scope: string[]): DrillBranch<L> | null {
	if (scope.length === 0) return null
	let level: DrillNode<L>[] = tree
	let current: DrillBranch<L> | null = null
	for (const key of scope) {
		const node = level.find((n) => n.key === key)
		if (!node || node.type !== 'branch') return null
		current = node
		level = node.children
	}
	return current
}

/** Walk the tree to the branch at scope, returning ALL branches along the
 * path (for breadcrumb rendering). The root is implicit and not returned. */
export function scopeChain<L>(tree: DrillNode<L>[], scope: string[]): DrillBranch<L>[] {
	const chain: DrillBranch<L>[] = []
	let level: DrillNode<L>[] = tree
	for (const key of scope) {
		const node = level.find((n) => n.key === key)
		if (!node || node.type !== 'branch') break
		chain.push(node)
		level = node.children
	}
	return chain
}

/** Flatten the tree into a leaf list with each leaf's deepest
 * `searchGroup`-anchor ancestor (or null if none). Skips branches marked
 * `omitFromSearch`. Deduplicates leaves by `key` (first occurrence wins). */
export function collectLeavesGrouped<L>(
	tree: DrillNode<L>[]
): { leaf: DrillLeaf<L>; group: DrillBranch<L> | null }[] {
	const out: { leaf: DrillLeaf<L>; group: DrillBranch<L> | null }[] = []
	const seen = new Set<string>()

	function walk(nodes: DrillNode<L>[], group: DrillBranch<L> | null) {
		for (const n of nodes) {
			if (n.type === 'leaf') {
				if (!seen.has(n.key)) {
					seen.add(n.key)
					out.push({ leaf: n, group })
				}
			} else {
				if (n.omitFromSearch) continue
				// Deeper `searchGroup` anchors override shallower ones.
				const nextGroup = n.searchGroup ? n : group
				walk(n.children, nextGroup)
			}
		}
	}
	walk(tree, null)
	return out
}

/** Fuzzy-search haystack string for a leaf. */
export function leafHaystack<L>(leaf: DrillLeaf<L>): string {
	if (leaf.searchableText) return leaf.searchableText
	if (leaf.label && leaf.secondary) return `${leaf.label} (${leaf.secondary})`
	return leaf.label || leaf.secondary || ''
}
