import type { FlowModule } from '$lib/gen'
import { type Node, type Edge } from '@xyflow/svelte'

export default function graphBuilder(modules: FlowModule[] | undefined): {
	nodes: Node[]
	edges: Edge[]
} {
	const nodes: Node[] = []
	const edges: Edge[] = []

	if (!modules) {
		return { nodes, edges }
	}

	function addNode(module: FlowModule, offset: number, type: string) {
		nodes.push({
			id: module.id,
			data: {
				value: module.value,
				offset: offset,
				module: module,
				modules: modules,
				parentIds: []
			},
			position: { x: -1, y: -1 },
			type: type
		})

		return module.id
	}

	const parents: { [key: string]: string[] } = {}

	function addEdge(sourceId: string, targetId: string, customId?: string, type?: string) {
		parents[targetId] = [...(parents[targetId] ?? []), sourceId]

		edges.push({
			id: customId || `edge:${sourceId}->${targetId}`,
			source: sourceId,
			target: targetId,
			type: type ?? 'edge'
		})
	}

	const inputNode: Node = {
		id: 'input',
		position: { x: -1, y: -1 },
		type: 'input2',
		data: {
			parentIds: []
		}
	}

	const resultNode: Node = {
		id: 'result',
		data: {
			parentIds: []
		},
		position: { x: -1, y: -1 },
		type: 'result'
	}

	nodes.push(inputNode)
	nodes.push(resultNode)

	function processModules(
		modules: FlowModule[],
		beforeNode: Node,
		nextNode: Node,
		currentOffset = 0
	) {
		let previousId: string | undefined = undefined

		modules.forEach((module, index) => {
			// Add the edge between the previous node and the current one
			if (index > 0 && previousId) {
				addEdge(previousId, module.id)
			}

			if (module.value.type === 'rawscript') {
				addNode(module, currentOffset, 'module')

				previousId = module.id
			} else if (module.value.type === 'branchall') {
				// Start
				addNode(module, currentOffset, 'module')

				// "Collect result of each branch" node
				const endNode = {
					id: `${module.id}-end`,
					data: { offset: currentOffset, id: module.id },
					position: { x: -1, y: -1 },
					type: 'branchAllEnd'
				}
				nodes.push(endNode)

				module.value.branches.forEach((branch, branchIndex) => {
					// Start node by branch

					const startNode = {
						id: `${module.id}-branch-${branchIndex}`,
						data: { offset: currentOffset, label: `Branch ${branchIndex + 1}`, id: module.id },
						position: { x: -1, y: -1 },
						type: 'branchAllStart'
					}

					nodes.push(startNode)

					addEdge(module.id, startNode.id, undefined, 'empty')

					processModules(branch.modules, startNode, endNode)
				})

				previousId = endNode.id
			} else if (module.value.type === 'forloopflow') {
				addNode(module, currentOffset, 'module')

				const startNode = {
					id: `${module.id}-start`,
					data: { offset: currentOffset + 50, id: module.id },
					position: { x: -1, y: -1 },
					type: 'forLoopStart'
				}

				addEdge(module.id, startNode.id, undefined, 'empty')

				const endNode = {
					id: `${module.id}-end`,
					data: { offset: currentOffset, id: module.id },
					position: { x: -1, y: -1 },
					type: 'forLoopEnd'
				}

				nodes.push(startNode)
				nodes.push(endNode)

				processModules(module.value.modules, startNode, endNode, currentOffset + 50)

				previousId = endNode.id
			} else if (module.value.type === 'whileloopflow') {
				addNode(module, currentOffset, 'module')

				const startNode = {
					id: `${module.id}-start`,
					data: { offset: currentOffset + 50 },
					position: { x: -1, y: -1 },
					type: 'whileLoopStart'
				}
				addEdge(module.id, startNode.id, undefined, 'empty')

				const endNode = {
					id: `${module.id}-end`,
					data: { offset: currentOffset },
					position: { x: -1, y: -1 },
					type: 'whileLoopEnd'
				}

				nodes.push(startNode)
				nodes.push(endNode)

				processModules(module.value.modules, startNode, endNode, currentOffset + 50)
				previousId = endNode.id
			} else if (module.value.type === 'branchone') {
				// Start
				addNode(module, currentOffset, 'module')

				// "Collect result of each branch" node
				const endNode = {
					id: `${module.id}-end`,
					data: { offset: currentOffset },
					position: { x: -1, y: -1 },
					type: 'branchOneEnd'
				}
				nodes.push(endNode)

				module.value.branches.forEach((branch, branchIndex) => {
					// Start node by branch

					const startNode = {
						id: `${module.id}-branch-${branchIndex}`,
						data: { offset: currentOffset },
						position: { x: -1, y: -1 },
						type: 'branchOneStart'
					}

					nodes.push(startNode)

					addEdge(module.id, startNode.id)

					processModules(branch.modules, startNode, endNode)
				})
			}

			if (index === 0) {
				addEdge(beforeNode.id, module.id)
			}

			if (index === modules.length - 1 && previousId) {
				addEdge(previousId, nextNode.id)
			}
		})
	}

	processModules(modules, inputNode, resultNode)

	Object.keys(parents).forEach((key) => {
		const node = nodes.find((n) => n.id === key)

		if (node) {
			node.data.parentIds = parents[key]
		}
	})

	return { nodes, edges }
}
