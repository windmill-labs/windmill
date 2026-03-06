import type { FlowModule } from '$lib/gen'
import { dfs } from './dfs'

/** Virtual/non-module node IDs that should be filtered out of multi-select operations */
const VIRTUAL_ID_PREFIXES = ['Input', 'Result', 'Trigger', 'preprocessor', 'failure']

function isVirtualId(id: string): boolean {
	if (VIRTUAL_ID_PREFIXES.includes(id)) return true
	if (
		id.endsWith('-start') ||
		id.endsWith('-end') ||
		id.includes('-branch-') ||
		id.startsWith('subflow:')
	) {
		return true
	}
	return false
}

/**
 * Single DFS pass to build a map of moduleId → set of ancestor module IDs.
 * Used to deduplicate nested selections (keep container, drop children).
 */
function buildAncestorMap(modules: FlowModule[]): Map<string, Set<string>> {
	const ancestors = new Map<string, Set<string>>()

	function walk(mods: FlowModule[], parentAncestors: Set<string>) {
		for (const mod of mods) {
			ancestors.set(mod.id, parentAncestors)
			const childAncestors = new Set([...parentAncestors, mod.id])

			const val = mod.value
			if (val.type === 'forloopflow' || val.type === 'whileloopflow') {
				walk(val.modules, childAncestors)
			} else if (val.type === 'branchall') {
				for (const branch of val.branches) walk(branch.modules, childAncestors)
			} else if (val.type === 'branchone') {
				for (const branch of val.branches) walk(branch.modules, childAncestors)
				walk(val.default, childAncestors)
			}
		}
	}

	walk(modules, new Set())
	return ancestors
}

/**
 * Filter raw selected node IDs down to the minimal set of top-level module IDs:
 * 1. Filter out virtual graph nodes (Input, Result, Trigger, -start, -end, -branch-*, subflow:*, preprocessor, failure)
 * 2. Verify each ID exists as a real module in the flow module tree
 * 3. Deduplicate nested: if a container (loop/branch) AND its children are both selected, keep only the container
 */
export function resolveSelectedModuleIds(rawIds: string[], modules: FlowModule[]): string[] {
	// Step 1: Filter out virtual IDs
	const candidateIds = rawIds.filter((id) => !isVirtualId(id))

	// Step 2+3: Single DFS to build ancestor map (also verifies existence)
	const ancestorMap = buildAncestorMap(modules)
	const verifiedIds = candidateIds.filter((id) => ancestorMap.has(id))

	// If any ancestor of this module is also selected, it's a nested child — drop it
	const selectedSet = new Set(verifiedIds)
	return verifiedIds.filter((id) => {
		const ancestors = ancestorMap.get(id)!
		for (const ancestor of ancestors) {
			if (selectedSet.has(ancestor)) return false
		}
		return true
	})
}

export type ModuleLocation = {
	id: string
	parentArray: FlowModule[]
	index: number
}

/**
 * For each ID, find its parent array (reference) and index using DFS.
 */
export function locateModules(ids: string[], modules: FlowModule[]): ModuleLocation[] {
	const idSet = new Set(ids)
	const locations: ModuleLocation[] = []

	dfs(modules, (mod, parentModules) => {
		if (idSet.has(mod.id)) {
			const index = parentModules.findIndex((m) => m.id === mod.id)
			if (index !== -1) {
				locations.push({ id: mod.id, parentArray: parentModules, index })
			}
		}
	})

	return locations
}

/**
 * Group locations that share the same parent array, sorted by index within each group.
 */
export function groupByParent(locations: ModuleLocation[]): ModuleLocation[][] {
	const groups = new Map<FlowModule[], ModuleLocation[]>()
	for (const loc of locations) {
		const existing = groups.get(loc.parentArray)
		if (existing) {
			existing.push(loc)
		} else {
			groups.set(loc.parentArray, [loc])
		}
	}
	// Sort each group by index
	for (const group of groups.values()) {
		group.sort((a, b) => a.index - b.index)
	}
	return Array.from(groups.values())
}

/**
 * True if all locations share the same parent array and have consecutive indices.
 * Required for move to be valid.
 */
export function areContiguousSiblings(locations: ModuleLocation[]): boolean {
	if (locations.length === 0) return false
	if (locations.length === 1) return true

	// All must share the same parent
	const parent = locations[0].parentArray
	if (!locations.every((loc) => loc.parentArray === parent)) return false

	// Sort by index and check contiguity
	const sorted = [...locations].sort((a, b) => a.index - b.index)
	for (let i = 1; i < sorted.length; i++) {
		if (sorted[i].index !== sorted[i - 1].index + 1) return false
	}
	return true
}
