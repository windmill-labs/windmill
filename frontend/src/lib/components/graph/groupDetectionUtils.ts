import { topologicalSort } from './graphBuilder.svelte'

/** Node IDs synthesized by graphBuilder that are not real FlowModules */
export const VIRTUAL_NODE_IDS = new Set(['Input', 'Result', 'Trigger'])

type FlowNode = { id: string; parentIds?: string[] }

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
 * Normalizes marker IDs (branch/forloop) to parent module IDs,
 * then uses topologicalSort to derive start and end boundaries.
 */
export function canFormValidGroup(
	selectedIds: string[],
	flowNodes: FlowNode[],
	excludeIds?: Set<string>
): { valid: true; startId: string; endId: string } | { valid: false } {
	if (selectedIds.length === 0) return { valid: false }

	// Normalize marker IDs to parent module IDs.
	// -start (forloop head) → parent ID. -end/-branch-* → skip if parent covered, else reject.
	const rawSet = new Set(selectedIds)
	const normalizedIds: string[] = []

	for (const id of selectedIds) {
		const parentId = id.replace(/-(end|start|branch-.*)$/, '')
		if (parentId === id) {
			normalizedIds.push(id)
			continue
		}
		if (id.endsWith('-start')) {
			normalizedIds.push(parentId)
			continue
		}
		// -end or -branch-*: parent must be covered (directly or via -start)
		if (!rawSet.has(parentId) && !rawSet.has(`${parentId}-start`)) {
			return { valid: false }
		}
	}

	if (normalizedIds.length === 0) return { valid: false }
	const normalizedSet = new Set(normalizedIds)

	// Topo sort full graph, filter to normalized selection
	const sorted = topologicalSort(flowNodes)
	const selectedSorted = sorted.filter((n) => normalizedSet.has(n.id))

	if (selectedSorted.length === 0) return { valid: false }

	// Reject virtual or excluded nodes
	if (selectedSorted.some((n) => VIRTUAL_NODE_IDS.has(n.id) || excludeIds?.has(n.id))) {
		return { valid: false }
	}

	// Topo order is bottom-first: first = bottom (end), last = top (start)
	const startId = selectedSorted[selectedSorted.length - 1].id
	const endId = selectedSorted[0].id

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
