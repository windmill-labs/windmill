import { topologicalSort } from './graphBuilder.svelte'

/** Node IDs synthesized by graphBuilder that are not real FlowModules */
export const VIRTUAL_NODE_IDS = new Set(['Input', 'Result', 'Trigger'])

type FlowNode = { id: string; parentIds?: string[] }

export type GroupMembership = {
	memberIds: string[]
	valid: boolean
	error?: string
	/** Topo sort slice range [firstIdx, lastIdx] — used for nesting depth */
	topoRange?: [number, number]
	/** Nesting depth: number of other groups whose topoRange strictly contains this one */
	depth?: number
}

/**
 * Compute the set of graph nodes that belong to a group defined by start_id and end_id.
 * Uses topological sort and slices between start and end — valid because windmill flows
 * are series-parallel (branches always converge at explicit end nodes).
 */
export function computeGroupNodeIds(
	startId: string,
	endId: string,
	flowNodes: FlowNode[]
): GroupMembership {
	if (startId === endId) {
		const sorted = topologicalSort(flowNodes)
		const idx = sorted.findIndex((n) => n.id === startId)
		return idx !== -1
			? { memberIds: [startId], valid: true, topoRange: [idx, idx] as [number, number] }
			: { memberIds: [], valid: false, error: 'Start node not found' }
	}

	const sorted = topologicalSort(flowNodes)
	// Topo sort puts bottom-of-graph nodes first, top-of-graph nodes last.
	// start_id = top of group (visually) = topo-last, end_id = bottom (visually) = topo-first.
	const endIdx = sorted.findIndex((n) => n.id === endId)

	// If end_id is a container (branchall, forloop, etc.), its last graph node
	// is `${endId}-end`. Use that as the actual start of the slice.
	const endNodeId = sorted.some((n) => n.id === `${endId}-end`) ? `${endId}-end` : endId
	let firstIdx = endNodeId === endId ? endIdx : sorted.findIndex((n) => n.id === endNodeId)

	let lastIdx = sorted.findIndex((n) => n.id === startId)

	if (firstIdx === -1 || lastIdx === -1) {
		return { memberIds: [], valid: false, error: 'Start or end node not found' }
	}
	if (firstIdx > lastIdx) {
		return { memberIds: [], valid: false, error: 'end_id must be topologically before start_id' }
	}

	// Extend range to fully include any groups partially within the range
	const groupHeadIndices = new Map<string, number>()
	const groupEndIndices = new Map<string, number>()
	for (let i = 0; i < sorted.length; i++) {
		const id = sorted[i].id
		if (id.startsWith('group:') && id.endsWith('-end')) {
			groupEndIndices.set(id.slice('group:'.length, -'-end'.length), i)
		} else if (id.startsWith('group:')) {
			groupHeadIndices.set(id.slice('group:'.length), i)
		}
	}

	let changed = true
	while (changed) {
		changed = false
		for (const [groupId, headIdx] of groupHeadIndices) {
			const endIdx = groupEndIndices.get(groupId)
			if (endIdx === undefined) continue
			const headIn = headIdx >= firstIdx && headIdx <= lastIdx
			const endIn = endIdx >= firstIdx && endIdx <= lastIdx
			if (headIn !== endIn) {
				firstIdx = Math.min(firstIdx, endIdx, headIdx)
				lastIdx = Math.max(lastIdx, endIdx, headIdx)
				changed = true
			}
		}
	}

	return {
		memberIds: sorted.slice(firstIdx, lastIdx + 1).map((n) => n.id),
		valid: true,
		topoRange: [firstIdx, lastIdx] as [number, number]
	}
}

/**
 * Compute the set of module IDs that belong to a group defined by start_id and end_id.
 * Uses the flattened module list (from getAllModules) and slices between start and end.
 * Used for collapsed group icons, step count, and moduleToCollapsedGroup mapping.
 */
export function computeGroupModuleIds(
	startId: string,
	endId: string,
	allModules: { id: string }[]
): string[] {
	if (startId === endId) {
		return allModules.some((m) => m.id === startId) ? [startId] : []
	}

	const startIdx = allModules.findIndex((m) => m.id === startId)
	const endIdx = allModules.findIndex((m) => m.id === endId)

	if (startIdx === -1 || endIdx === -1 || startIdx > endIdx) {
		return []
	}

	return allModules.slice(startIdx, endIdx + 1).map((m) => m.id)
}

/**
 * Check whether a set of selected node IDs can form a valid group.
 * Uses topologicalSort to find start (first) and end (last) of selection,
 * then validates with computeGroupNodeIds.
 */
export function canFormValidGroup(
	selectedIds: string[],
	flowNodes: FlowNode[]
): { valid: true; startId: string; endId: string } | { valid: false } {
	if (selectedIds.length === 0) return { valid: false }

	const selectedSet = new Set(selectedIds)

	// Topological sort of the full graph, then filter to selected nodes
	const sorted = topologicalSort(flowNodes)
	const selectedSorted = sorted.filter((n) => selectedSet.has(n.id))

	if (selectedSorted.length === 0) return { valid: false }

	// Filter out virtual nodes — they cannot be part of a group
	const nonVirtualSorted = selectedSorted.filter((n) => !VIRTUAL_NODE_IDS.has(n.id))
	if (nonVirtualSorted.length === 0) return { valid: false }

	// Topo sort: index 0 = bottom of flow, last index = top of flow
	// start_id = top (visually), end_id = bottom (visually)
	const startId = nonVirtualSorted[nonVirtualSorted.length - 1].id
	const endId = nonVirtualSorted[0].id

	// Validate that the computed members match the selection
	const membership = computeGroupNodeIds(startId, endId, flowNodes)
	if (!membership.valid) return { valid: false }

	// Check that all selected ids are in the computed members
	const memberSet = new Set(membership.memberIds)
	for (const id of selectedIds) {
		if (!memberSet.has(id)) return { valid: false }
	}

	// Derive boundaries from expanded membership (which includes full groups)
	// Filter out graph-internal marker nodes — start_id/end_id must be real module IDs
	const realModules = membership.memberIds.filter(
		(id) => !id.startsWith('group:') && !id.endsWith('-end')
	)
	if (realModules.length === 0) return { valid: false }

	// memberIds are in topo order (bottom-first), so:
	const expandedEndId = realModules[0]
	const expandedStartId = realModules[realModules.length - 1]

	return { valid: true, startId: expandedStartId, endId: expandedEndId }
}

/**
 * Legacy utility: complete a group and split it into connected components.
 * Still used by NoteEditor for FlowNote group notes (contained_node_ids).
 */
export function completeAndSplitGroup(groupNodes: string[], flowNodes: FlowNode[]): string[][] {
	if (groupNodes.length <= 1) {
		return groupNodes.length === 1 ? [groupNodes] : []
	}

	// Build parent map for upward traversal only
	const parents = new Map<string, string[]>()
	for (const node of flowNodes) {
		parents.set(node.id, node.parentIds || [])
	}

	const groupSet = new Set(groupNodes)
	const assignedComponent = new Map<string, number>()
	const components: Array<Set<string>> = []

	const mergeComponents = (fromIdx: number, toIdx: number): void => {
		if (fromIdx === toIdx) return
		const target = components[toIdx]
		const source = components[fromIdx]

		source.forEach((node) => target.add(node))
		source.clear()

		for (const [nodeId, idx] of assignedComponent.entries()) {
			if (idx === fromIdx) {
				assignedComponent.set(nodeId, toIdx)
			}
		}
	}

	for (const startNode of groupNodes) {
		if (assignedComponent.has(startNode)) continue

		const componentIdx = components.length
		components.push(new Set([startNode]))
		assignedComponent.set(startNode, componentIdx)

		const stack: { nodeId: string; path: string[]; seen: Set<string> }[] = [
			{ nodeId: startNode, path: [startNode], seen: new Set([startNode]) }
		]

		while (stack.length > 0) {
			const { nodeId, path, seen } = stack.pop()!
			const parentIds = parents.get(nodeId) || []

			for (const parentId of parentIds) {
				if (seen.has(parentId)) continue

				const newPath = [...path, parentId]
				const newSeen = new Set(seen)
				newSeen.add(parentId)

				if (groupSet.has(parentId)) {
					const existingIdx = assignedComponent.get(parentId)
					if (existingIdx === undefined) {
						assignedComponent.set(parentId, componentIdx)
						components[componentIdx].add(parentId)
						stack.push({ nodeId: parentId, path: [parentId], seen: new Set([parentId]) })
					} else if (existingIdx !== componentIdx) {
						mergeComponents(existingIdx, componentIdx)
					}

					for (const node of newPath) {
						components[componentIdx].add(node)
					}
				} else {
					stack.push({ nodeId: parentId, path: newPath, seen: newSeen })
				}
			}
		}
	}

	return components
		.filter((component) => component.size > 0)
		.map((component) =>
			Array.from(component)
				.filter((nodeId) => !nodeId.startsWith('subflow:'))
				.sort()
		)
		.filter((component) => component.length > 0)
}
