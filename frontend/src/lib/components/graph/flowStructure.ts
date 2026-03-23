import type { FlowModule } from '$lib/gen'

import type { FlowGroup, GraphGroup } from './groupEditor.svelte'
import { getContainerInnerArrays } from './groupEditor.svelte'
import { VIRTUAL_NODE_IDS } from './groupDetectionUtils'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ContainerKind = 'forloopflow' | 'whileloopflow' | 'branchone' | 'branchall'

export type StructureBranch = {
	label?: string
	children: FlowStructureNode[]
}

export type FlowStructureNode = {
	/** FlowModule.id for modules, FlowGroup.id for groups */
	id: string
	kind: 'leaf' | 'group' | ContainerKind
	/** Only present when kind === 'group' */
	group?: FlowGroup
	/** Only present when kind === 'group' — flat module IDs for step count */
	moduleIds?: string[]
	/** Child branches. leaf=[], group=[{children}], container=[{children}, ...] */
	branches: StructureBranch[]
}

// ---------------------------------------------------------------------------
// Type guards
// ---------------------------------------------------------------------------
// Building the structure tree
// ---------------------------------------------------------------------------

export function buildStructureTree(
	modules: FlowModule[],
	groups: GraphGroup[]
): FlowStructureNode[] {
	const { items, consumed } = buildStructureTreeRecurse(modules, groups)
	const unconsumed = groups.filter((g) => !consumed.has(g.id))
	if (unconsumed.length > 0) {
		console.warn(
			'Groups with no matching modules (ill-formed):',
			unconsumed.map((g) => g.id)
		)
	}
	return items
}

export function moduleToStructureNode(mod: FlowModule): FlowStructureNode {
	const innerArrays = getContainerInnerArrays(mod)
	if (innerArrays.length === 0) {
		return { id: mod.id, kind: 'leaf', branches: [] }
	}

	const kind = (mod.value as any).type as ContainerKind
	const branches: StructureBranch[] = innerArrays.map(({ get, label }) => ({
		label,
		children: [] // filled later by recursion
	}))

	return { id: mod.id, kind, branches }
}

function buildStructureTreeRecurse(
	modules: FlowModule[],
	groups: GraphGroup[]
): { items: FlowStructureNode[]; consumed: Set<string> } {
	if (modules.length === 0) {
		return { items: [], consumed: new Set() }
	}

	const indexMap = new Map<string, number>()
	for (let i = 0; i < modules.length; i++) {
		indexMap.set(modules[i].id, i)
	}

	// Reject duplicate group IDs
	const seenGroupIds = new Set<string>()
	for (const g of groups) {
		if (seenGroupIds.has(g.id)) {
			throw new Error(`Duplicate group id: '${g.id}'`)
		}
		seenGroupIds.add(g.id)
	}

	// Reject groups referencing virtual nodes
	for (const g of groups) {
		if (VIRTUAL_NODE_IDS.has(g.start_id) || VIRTUAL_NODE_IDS.has(g.end_id)) {
			throw new Error(
				`Group '${g.id}' references virtual node: groups cannot include Input, Result, or Trigger`
			)
		}
	}

	// Partition: groups for this level vs rest
	const levelGroups: GraphGroup[] = []
	const otherGroups: GraphGroup[] = []
	for (const g of groups) {
		if (indexMap.has(g.start_id) && indexMap.has(g.end_id)) {
			const s = indexMap.get(g.start_id)!
			const e = indexMap.get(g.end_id)!
			if (s > e) {
				throw new Error(
					`Group '${g.id}' has inverted range: start_id='${g.start_id}' (index ${s}) > end_id='${g.end_id}' (index ${e})`
				)
			}
			levelGroups.push(g)
		} else {
			otherGroups.push(g)
		}
	}

	// Validate no partial overlaps
	for (let i = 0; i < levelGroups.length; i++) {
		for (let j = i + 1; j < levelGroups.length; j++) {
			const a = levelGroups[i]
			const b = levelGroups[j]
			const aStart = indexMap.get(a.start_id)!
			const aEnd = indexMap.get(a.end_id)!
			const bStart = indexMap.get(b.start_id)!
			const bEnd = indexMap.get(b.end_id)!

			if (aEnd < bStart || bEnd < aStart) continue
			if (aStart <= bStart && bEnd <= aEnd) continue
			if (bStart <= aStart && aEnd <= bEnd) continue

			throw new Error(`Groups '${a.id}' and '${b.id}' overlap without nesting`)
		}
	}

	// Build grouped structure for this level
	function build(
		startIdx: number,
		endIdx: number,
		availableGroups: GraphGroup[]
	): FlowStructureNode[] {
		const result: FlowStructureNode[] = []
		let i = startIdx
		while (i <= endIdx) {
			const candidates = availableGroups.filter((g) => {
				const gStart = indexMap.get(g.start_id)!
				const gEnd = indexMap.get(g.end_id)!
				return gStart === i && gEnd <= endIdx
			})
			candidates.sort((a, b) => {
				const spanA = indexMap.get(a.end_id)! - indexMap.get(a.start_id)!
				const spanB = indexMap.get(b.end_id)! - indexMap.get(b.start_id)!
				return spanB - spanA
			})

			const group = candidates[0]
			if (group) {
				const gEnd = indexMap.get(group.end_id)!
				const remaining = availableGroups.filter((g) => g.id !== group.id)
				const innerNodes = build(i, gEnd, remaining)

				const moduleIds: string[] = []
				for (let k = i; k <= gEnd; k++) {
					moduleIds.push(modules[k].id)
				}

				result.push({
					id: group.id,
					kind: 'group',
					group: {
						id: group.id,
						summary: group.summary,
						note: group.note,
						color: group.color,
						collapsed_by_default: group.collapsed_by_default,
						start_id: group.start_id,
						end_id: group.end_id
					},
					moduleIds,
					branches: [{ children: innerNodes }]
				})
				i = gEnd + 1
			} else {
				result.push(moduleToStructureNode(modules[i]))
				i++
			}
		}
		return result
	}

	const result = build(0, modules.length - 1, levelGroups)

	// Recurse into containers with remaining unconsumed groups
	const consumed = new Set(levelGroups.map((g) => g.id))
	let remaining = otherGroups

	function recurseIntoContainers(items: FlowStructureNode[]): void {
		for (const item of items) {
			if (item.kind === 'group') {
				recurseIntoContainers(item.branches[0].children)
				continue
			}
			if (item.branches.length === 0) continue

			// This is a container module — get inner FlowModule arrays and recurse
			const modIdx = indexMap.get(item.id)
			if (modIdx === undefined) continue
			const mod = modules[modIdx]

			const innerArrays = getContainerInnerArrays(mod)
			for (let bi = 0; bi < innerArrays.length; bi++) {
				const inner = buildStructureTreeRecurse(innerArrays[bi].get(), remaining)
				item.branches[bi] = {
					label: item.branches[bi]?.label,
					children: inner.items
				}
				for (const id of inner.consumed) consumed.add(id)
				remaining = remaining.filter((g) => !inner.consumed.has(g.id))
			}
		}
	}
	recurseIntoContainers(result)

	return { items: result, consumed }
}

// ---------------------------------------------------------------------------
// Traversal utilities
// ---------------------------------------------------------------------------

/** Generic DFS over the structure tree */
export function dfsStructure(
	nodes: FlowStructureNode[],
	fn: (node: FlowStructureNode, parentArray: FlowStructureNode[]) => void
): void {
	for (const node of nodes) {
		fn(node, nodes)
		for (const branch of node.branches) {
			dfsStructure(branch.children, fn)
		}
	}
}

/** Flatten to ordered module IDs (groups are transparent) */
export function flattenStructureIds(nodes: FlowStructureNode[]): string[] {
	const ids: string[] = []
	for (const node of nodes) {
		if (node.kind === 'group') {
			ids.push(...flattenStructureIds(node.branches[0].children))
		} else {
			ids.push(node.id)
		}
	}
	return ids
}

/** Collect leaf module IDs recursively (including inside containers) */
export function collectLeafIds(nodes: FlowStructureNode[]): string[] {
	const ids: string[] = []
	for (const node of nodes) {
		if (node.kind === 'group') {
			ids.push(...collectLeafIds(node.branches[0].children))
		} else {
			ids.push(node.id)
			for (const branch of node.branches) {
				ids.push(...collectLeafIds(branch.children))
			}
		}
	}
	return ids
}

// ---------------------------------------------------------------------------
// Finding nodes in the tree
// ---------------------------------------------------------------------------

export type FindResult = { parentChildren: FlowStructureNode[]; index: number }

export function findInStructure(nodes: FlowStructureNode[], id: string): FindResult | undefined {
	for (let i = 0; i < nodes.length; i++) {
		const node = nodes[i]
		if (node.id === id) return { parentChildren: nodes, index: i }
		for (const branch of node.branches) {
			const found = findInStructure(branch.children, id)
			if (found) return found
		}
	}
	return undefined
}

/**
 * Match a structure node against a graph node ID.
 * Handles group head/end IDs (group:X, group:X-end) and collapsed-group:X.
 */
export function matchStructureNode(node: FlowStructureNode, nodeId: string): boolean {
	if (node.id === nodeId) return true
	if (node.kind === 'group') {
		return (
			nodeId === `group:${node.id}` ||
			nodeId === `group:${node.id}-end` ||
			nodeId === `collapsed-group:${node.id}`
		)
	}
	return false
}

/**
 * Find insert index using graph node IDs (handles group:X-end etc.).
 * Returns the index OF the matched item (insert before it).
 * For group-end nodes, returns index AFTER the group (insert after it).
 */
export function findInsertIndexByNodeId(items: FlowStructureNode[], targetNodeId: string): number {
	// group-end: insert after the group
	if (targetNodeId.startsWith('group:') && targetNodeId.endsWith('-end')) {
		const groupId = targetNodeId.slice('group:'.length, -'-end'.length)
		const idx = items.findIndex((n) => n.kind === 'group' && n.id === groupId)
		return idx >= 0 ? idx + 1 : items.length
	}
	// Everything else: insert at the matched item's position
	for (let i = 0; i < items.length; i++) {
		if (matchStructureNode(items[i], targetNodeId)) return i
	}
	return items.length
}

// ---------------------------------------------------------------------------
// Deriving groups from the structure tree
// ---------------------------------------------------------------------------

export function deriveGroupsFromStructure(nodes: FlowStructureNode[]): FlowGroup[] {
	const groups: FlowGroup[] = []
	for (const node of nodes) {
		if (node.kind === 'group' && node.group) {
			const flatIds = flattenStructureIds(node.branches[0].children)
			if (flatIds.length === 0) {
				console.warn(`deriveGroupsFromStructure: skipping empty group "${node.id}"`)
				continue
			}
			groups.push({
				...node.group,
				start_id: flatIds[0],
				end_id: flatIds[flatIds.length - 1]
			})
			// Recurse for nested groups
			groups.push(...deriveGroupsFromStructure(node.branches[0].children))
		} else {
			for (const branch of node.branches) {
				groups.push(...deriveGroupsFromStructure(branch.children))
			}
		}
	}
	return groups
}

// ---------------------------------------------------------------------------
// Syncing structure back to FlowModule[]
// ---------------------------------------------------------------------------

/**
 * Reconstruct a FlowModule[] from the structure tree, looking up originals
 * from moduleMap and patching container inner arrays to match the tree ordering.
 */
export function applyStructureToModules(
	nodes: FlowStructureNode[],
	moduleMap: Map<string, FlowModule>
): FlowModule[] {
	const result: FlowModule[] = []
	for (const node of nodes) {
		if (node.kind === 'group') {
			// Groups are transparent — splice their children into this level
			result.push(...applyStructureToModules(node.branches[0].children, moduleMap))
		} else {
			const mod = moduleMap.get(node.id)
			if (!mod) continue

			// Patch container inner arrays
			if (node.branches.length > 0) {
				const innerArrays = getContainerInnerArrays(mod)
				for (let bi = 0; bi < innerArrays.length && bi < node.branches.length; bi++) {
					innerArrays[bi].set(applyStructureToModules(node.branches[bi].children, moduleMap))
				}
			}

			result.push(mod)
		}
	}
	return result
}

// ---------------------------------------------------------------------------
// Empty groups cleanup
// ---------------------------------------------------------------------------

/**
 * Walk the tree, remove group nodes that have no leaf modules, and return
 * the removed groups. Mutates the input array in-place.
 * Recurses depth-first so inner groups are cleaned before checking outer ones.
 */
export function removeEmptyGroups(nodes: FlowStructureNode[]): FlowGroup[] {
	const removed: FlowGroup[] = []
	for (let i = nodes.length - 1; i >= 0; i--) {
		const node = nodes[i]
		if (node.kind === 'group' && node.group) {
			// Recurse first — inner groups may become empty too
			removed.push(...removeEmptyGroups(node.branches[0].children))
			if (flattenStructureIds(node.branches[0].children).length === 0) {
				removed.push(node.group)
				nodes.splice(i, 1)
			}
		} else {
			for (const branch of node.branches) {
				removed.push(...removeEmptyGroups(branch.children))
			}
		}
	}
	return removed
}

/** Walk the structure tree to compute nesting depth for each group (O(n)). */
export function computeGroupDepths(tree: FlowStructureNode[]): Record<string, number> {
	const depths: Record<string, number> = {}
	function walk(nodes: FlowStructureNode[], groupDepth: number): void {
		for (const node of nodes) {
			if (node.kind === 'group') {
				depths[node.id] = groupDepth
				for (const branch of node.branches) {
					walk(branch.children, groupDepth + 1)
				}
			} else {
				for (const branch of node.branches) {
					walk(branch.children, groupDepth)
				}
			}
		}
	}
	walk(tree, 0)
	return depths
}
