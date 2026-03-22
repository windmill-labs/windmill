import { untrack } from 'svelte'
import type { FlowModule } from '$lib/gen'
import {
	buildGroupedModules,
	type FlowGroup,
	type GraphGroup,
	type GroupedModule
} from './groupEditor.svelte'
import type { StateStore } from '$lib/utils'
import { getAllModules } from '../flows/flowExplorer'
import { computeGroupModuleIds } from './groupDetectionUtils'
import { stateSnapshot } from '$lib/svelte5Utils.svelte'

export type ExtendedOpenFlow = {
	value: {
		modules: FlowModule[]
		groups?: FlowGroup[]
		[key: string]: any
	}
	[key: string]: any
}

/** Check if a GroupedModule item is a group (not a FlowModule) */
export function isGroupItem(item: GroupedModule): item is {
	type: 'group'
	group: FlowGroup
	modules: GroupedModule[]
	moduleIds: string[]
} {
	return 'type' in item && item.type === 'group'
}

/** Match a GroupedModule item against a graph node ID */
export function matchGroupedModule(item: GroupedModule, nodeId: string): boolean {
	if (isGroupItem(item)) {
		const gId = item.group.id
		return nodeId === `collapsed-group:${gId}` || nodeId === `group:${gId}`
	}
	return (item as FlowModule).id === nodeId
}

/** Find insert position in a GroupedModule[] array by target node ID.
 *  For group-end nodes, returns index after the group.
 *  For group-head / collapsed-group, returns the group's index.
 *  For regular modules, returns that module's index. */
export function findInsertIndex(items: GroupedModule[], targetId: string): number {
	// Handle group-end nodes: insert after the group
	if (targetId.startsWith('group:') && targetId.endsWith('-end')) {
		const groupId = targetId.slice('group:'.length, -'-end'.length)
		const idx = items.findIndex((m) => isGroupItem(m) && m.group.id === groupId)
		return idx >= 0 ? idx + 1 : items.length
	}
	// For everything else, find the matching item
	const idx = items.findIndex((m) => matchGroupedModule(m, targetId))
	return idx >= 0 ? idx : items.length
}

/** DFS traversal of GroupedModule[], handling groups by recursing into group.modules */
export function dfsGrouped(
	items: GroupedModule[],
	f: (item: GroupedModule, parentArray: GroupedModule[], branches: FlowModule[][]) => void
): void {
	for (const item of items) {
		if (isGroupItem(item)) {
			f(item, items, [item.modules as FlowModule[]])
			dfsGrouped(item.modules, f)
		} else {
			const mod = item as FlowModule
			if (mod.value.type === 'forloopflow' || mod.value.type === 'whileloopflow') {
				f(item, items, [mod.value.modules])
				dfsGrouped(mod.value.modules as GroupedModule[], f)
			} else if (mod.value.type === 'branchone') {
				const branches = [mod.value.default, ...mod.value.branches.map((b) => b.modules)]
				f(item, items, branches)
				for (const b of branches) dfsGrouped(b as GroupedModule[], f)
			} else if (mod.value.type === 'branchall') {
				const branches = mod.value.branches.map((b) => b.modules)
				f(item, items, branches)
				for (const b of branches) dfsGrouped(b as GroupedModule[], f)
			} else if (mod.value.type === 'aiagent') {
				f(item, items, [mod.value.tools as FlowModule[]])
				dfsGrouped(mod.value.tools as GroupedModule[], f)
			} else {
				f(item, items, [])
			}
		}
	}
}

/** Match a GroupedModule for DFS, handling group:X-end suffix */
export function matchGroupedModuleForDfs(item: GroupedModule, nodeId: string): boolean {
	if (nodeId.startsWith('group:') && nodeId.endsWith('-end')) {
		const groupId = nodeId.slice('group:'.length, -'-end'.length)
		return isGroupItem(item) && item.group.id === groupId
	}
	return matchGroupedModule(item, nodeId)
}

/** Flatten GroupedModule[] back to FlowModule[] */
export function flattenGroupedModules(items: GroupedModule[]): FlowModule[] {
	return items.flatMap((item) => {
		if (isGroupItem(item)) return flattenGroupedModules(item.modules)
		const mod = item as FlowModule
		if (mod.value.type === 'forloopflow' || mod.value.type === 'whileloopflow') {
			mod.value.modules = flattenGroupedModules(mod.value.modules as GroupedModule[])
		} else if (mod.value.type === 'branchone') {
			mod.value.default = flattenGroupedModules(mod.value.default as GroupedModule[])
			for (const b of mod.value.branches) {
				b.modules = flattenGroupedModules(b.modules as GroupedModule[])
			}
		} else if (mod.value.type === 'branchall') {
			for (const b of mod.value.branches) {
				b.modules = flattenGroupedModules(b.modules as GroupedModule[])
			}
		}
		return [mod]
	})
}

/** Derive FlowGroup[] from the grouped structure */
export function deriveGroups(items: GroupedModule[]): FlowGroup[] {
	const groups: FlowGroup[] = []
	for (const item of items) {
		if (isGroupItem(item)) {
			const flat = flattenGroupedModules(item.modules)
			if (flat.length === 0) {
				throw new Error(
					`deriveGroups: group "${item.group.id}" has no modules — use getGroupsEmptiedBy() to detect and confirm before removing`
				)
			}
			groups.push({
				id: item.group.id,
				summary: item.group.summary,
				note: item.group.note,
				color: item.group.color,
				collapsed_by_default: item.group.collapsed_by_default,
				start_id: flat[0].id,
				end_id: flat[flat.length - 1].id
			})
			// Recurse for nested groups
			groups.push(...deriveGroups(item.modules))
		} else {
			const mod = item as FlowModule
			if (mod.value.type === 'forloopflow' || mod.value.type === 'whileloopflow') {
				groups.push(...deriveGroups(mod.value.modules as GroupedModule[]))
			} else if (mod.value.type === 'branchone') {
				groups.push(...deriveGroups(mod.value.default as GroupedModule[]))
				for (const b of mod.value.branches) {
					groups.push(...deriveGroups(b.modules as GroupedModule[]))
				}
			} else if (mod.value.type === 'branchall') {
				for (const b of mod.value.branches) {
					groups.push(...deriveGroups(b.modules as GroupedModule[]))
				}
			}
		}
	}
	return groups
}

type FindResult = { array: GroupedModule[]; index: number }

/** Recursively find a module's position in the grouped tree */
function findInTreeImpl(items: GroupedModule[], id: string): FindResult | undefined {
	for (let i = 0; i < items.length; i++) {
		const item = items[i]
		if (matchGroupedModule(item, id)) return { array: items, index: i }
		if (isGroupItem(item)) {
			const inner = findInTreeImpl(item.modules, id)
			if (inner) return inner
		}
	}
	return undefined
}

/**
 * Proxy store that wraps GroupedModule[] and syncs mutations
 * back to flowStore.val.value.modules and flowStore.val.value.groups.
 */
export class GroupedModulesProxy {
	#items = $state<GroupedModule[]>([])
	#error = $state<unknown>(undefined)
	#flowStore: StateStore<ExtendedOpenFlow>
	#syncing = false

	constructor(flowStore: StateStore<ExtendedOpenFlow>) {
		this.#flowStore = flowStore
		this.rebuild()

		// Watch for external changes (undo/load)
		$effect(() => {
			// Track only the identity of modules/groups arrays
			void flowStore.val.value.modules
			void flowStore.val.value.groups
			if (!this.#syncing) {
				// untrack rebuild to avoid tracking deep reads (getAllModules etc.)
				// that would re-trigger when buildGroupedModules mutates container inner arrays
				untrack(() => this.rebuild())
			}
		})
	}

	/** Reactive access to the grouped structure */
	get items(): GroupedModule[] {
		return this.#items
	}

	/** Reactive access to group build errors */
	get error(): unknown {
		return this.#error
	}

	/** Find a position in the tree by ID (recurses into groups) */
	findInTree(id: string): FindResult | undefined {
		return findInTreeImpl(this.#items, id)
	}

	/** Remove a module by ID. Walks the tree to find it. Syncs to flowStore. */
	removeById(id: string): FlowModule | undefined {
		const found = this.findInTree(id)
		if (!found) return undefined

		const [removed] = found.array.splice(found.index, 1)
		const removedModule = isGroupItem(removed)
			? undefined // Groups themselves shouldn't be removed this way
			: (removed as FlowModule)

		this.syncToStore()
		return removedModule
	}

	/** Check which groups would become empty if these module IDs were removed */
	getGroupsEmptiedBy(ids: string[]): FlowGroup[] {
		const nodeSet = new Set(ids)
		return this.#collectEmptiedGroups(this.#items, nodeSet)
	}

	/** Rebuild from flowStore */
	rebuild(): void {
		const modules = stateSnapshot(this.#flowStore.val.value.modules) as FlowModule[]
		const allGroups = this.#flowStore.val.value.groups ?? []
		const allModules = getAllModules(modules)
		const graphGroups: GraphGroup[] = allGroups.map((g) => ({
			...g,
			moduleIds: computeGroupModuleIds(g.start_id, g.end_id, allModules)
		}))
		const excludeIds = new Set<string>()
		const pp = this.#flowStore.val.value.preprocessor_module?.id
		if (pp) excludeIds.add(pp)
		const fm = this.#flowStore.val.value.failure_module?.id
		if (fm) excludeIds.add(fm)
		try {
			this.#items = buildGroupedModules(modules, graphGroups, excludeIds)
			this.#error = undefined
		} catch (e) {
			this.#error = e
		}
	}

	/** Sync grouped structure back to flowStore */
	syncToStore(): void {
		this.#syncing = true
		// IMPORTANT: derive groups BEFORE flattening — flattenGroupedModules mutates
		// container inner arrays in-place, which would destroy group items before
		// deriveGroups can find them.
		const derivedGroups = deriveGroups(this.#items)
		this.#flowStore.val.value.modules = flattenGroupedModules(this.#items)
		this.#flowStore.val.value.groups = derivedGroups
		this.#syncing = false
	}

	#collectEmptiedGroups(items: GroupedModule[], nodeSet: Set<string>): FlowGroup[] {
		const result: FlowGroup[] = []
		for (const item of items) {
			if (isGroupItem(item)) {
				const flat = flattenGroupedModules(item.modules)
				const remaining = flat.filter((m) => !nodeSet.has(m.id))
				if (remaining.length === 0 && flat.length > 0) {
					result.push(item.group)
				}
				// Recurse into nested groups
				result.push(...this.#collectEmptiedGroups(item.modules, nodeSet))
			} else {
				const mod = item as FlowModule
				if (mod.value.type === 'forloopflow' || mod.value.type === 'whileloopflow') {
					result.push(...this.#collectEmptiedGroups(mod.value.modules as GroupedModule[], nodeSet))
				} else if (mod.value.type === 'branchone') {
					result.push(...this.#collectEmptiedGroups(mod.value.default as GroupedModule[], nodeSet))
					for (const b of mod.value.branches) {
						result.push(...this.#collectEmptiedGroups(b.modules as GroupedModule[], nodeSet))
					}
				} else if (mod.value.type === 'branchall') {
					for (const b of mod.value.branches) {
						result.push(...this.#collectEmptiedGroups(b.modules as GroupedModule[], nodeSet))
					}
				}
			}
		}
		return result
	}
}
