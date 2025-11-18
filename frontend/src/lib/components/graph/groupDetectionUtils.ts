type FlowNode = { id: string; parentIds?: string[] }

/**
 * Use a simple algorithm to complete a group and split it into connected components
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
