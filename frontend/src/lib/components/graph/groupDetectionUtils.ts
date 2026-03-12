import { topologicalSort } from './graphBuilder.svelte'
import { expandWithContainerDescendants } from './util'

type FlowNode = { id: string; parentIds?: string[] }

export type GroupMembership = {
	memberIds: string[]
	valid: boolean
	error?: string
}

/**
 * Compute the set of nodes that belong to a group defined by start_id and end_id.
 *
 * Algorithm:
 * 1. Build a childrenMap from parentIds
 * 2. Walk forward from start_id collecting all reachable nodes (stop at end_id, don't go past)
 * 3. Expand with containerDescendants
 */
export function computeGroupMembers(
	startId: string,
	endId: string,
	flowNodes: FlowNode[],
	containerDescendants: Map<string, string[]>
): GroupMembership {
	const nodeMap = new Map<string, FlowNode>()
	for (const n of flowNodes) nodeMap.set(n.id, n)

	if (!nodeMap.has(startId) || !nodeMap.has(endId)) {
		return { memberIds: [], valid: false, error: 'Start or end node not found' }
	}

	// Single node group
	if (startId === endId) {
		const members = expandWithContainerDescendants([startId], containerDescendants)
		return { memberIds: members, valid: true }
	}

	// Build children map (node → children that list it as parent)
	const childrenMap = new Map<string, string[]>()
	for (const node of flowNodes) {
		for (const pid of node.parentIds ?? []) {
			const children = childrenMap.get(pid)
			if (children) {
				children.push(node.id)
			} else {
				childrenMap.set(pid, [node.id])
			}
		}
	}

	// Forward walk from start_id: collect all nodes reachable without going past end_id
	const reachable = new Set<string>()
	const queue = [startId]
	reachable.add(startId)
	let qi = 0
	while (qi < queue.length) {
		const current = queue[qi++]
		if (current === endId) continue // don't go past end
		for (const child of childrenMap.get(current) ?? []) {
			if (!reachable.has(child)) {
				reachable.add(child)
				queue.push(child)
			}
		}
	}

	if (!reachable.has(endId)) {
		return { memberIds: [], valid: false, error: 'End node is not reachable from start node' }
	}

	// Expand with container descendants
	const members = expandWithContainerDescendants(Array.from(reachable), containerDescendants)
	return { memberIds: members, valid: true }
}

/**
 * Check whether a set of selected node IDs can form a valid group.
 * Uses topologicalSort to find start (first) and end (last) of selection,
 * then validates with computeGroupMembers.
 */
export function canFormValidGroup(
	selectedIds: string[],
	flowNodes: FlowNode[],
	containerDescendants: Map<string, string[]>
): { valid: true; startId: string; endId: string } | { valid: false } {
	if (selectedIds.length === 0) return { valid: false }

	const selectedSet = new Set(selectedIds)

	// Topological sort of the full graph, then filter to selected nodes
	const sorted = topologicalSort(flowNodes)
	const selectedSorted = sorted.filter((n) => selectedSet.has(n.id))

	if (selectedSorted.length === 0) return { valid: false }

	const startId = selectedSorted[0].id
	const endId = selectedSorted[selectedSorted.length - 1].id

	// Validate that the computed members match the selection
	const membership = computeGroupMembers(startId, endId, flowNodes, containerDescendants)
	if (!membership.valid) return { valid: false }

	// Check that all selected ids are in the computed members
	const memberSet = new Set(membership.memberIds)
	for (const id of selectedIds) {
		if (!memberSet.has(id)) return { valid: false }
	}

	return { valid: true, startId, endId }
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
