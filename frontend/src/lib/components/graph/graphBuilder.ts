import type { FlowModule } from '$lib/gen'
import { type Node, type Edge } from '@xyflow/svelte'
import { getDependeeAndDependentComponents } from '../flows/flowExplorer'
import { dfsByModule } from '../flows/previousResults'

export type GraphEventHandlers = {
	insert: (detail) => void
	deleteBranch: (detail, label: string) => void
	select: (modId: string) => void
	delete: (detail, label: string) => void
	newBranch: (module: FlowModule) => void
	move: (module: FlowModule, modules: FlowModule[]) => void
	selectIteration: (detail, moduleId: string) => void
}

export default function graphBuilder(
	modules: FlowModule[] | undefined,
	extra: Record<string, any>,
	failureModule: FlowModule | undefined,
	eventHandlers: GraphEventHandlers,
	success: boolean | undefined,
	useDataflow: boolean | undefined,
	selectedId: string | undefined
): {
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
				parentIds: [],
				eventHandlers: eventHandlers,
				...extra
			},
			position: { x: -1, y: -1 },
			type: type
		})

		return module.id
	}

	const parents: { [key: string]: string[] } = {}

	function addEdge(
		sourceId: string,
		targetId: string,
		customId?: string,
		type?: string,
		offset?: number
	) {
		parents[targetId] = [...(parents[targetId] ?? []), sourceId]

		edges.push({
			id: customId || `edge:${sourceId}->${targetId}`,
			source: sourceId,
			target: targetId,
			type: type ?? 'edge',
			data: {
				insertable: extra.insertable,
				modules,
				sourceId,
				targetId,
				offset
			}
		})
	}

	const inputNode: Node = {
		id: 'Input',
		position: { x: -1, y: -1 },
		type: 'input2',
		data: {
			eventHandlers: eventHandlers,
			modules: modules,
			...extra
		}
	}

	const resultNode: Node = {
		id: 'result',
		data: {
			eventHandlers: eventHandlers,
			modules: modules,
			success: success,
			...extra
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
					data: {
						offset: currentOffset,
						id: module.id,
						module: module,
						modules: modules,
						...extra
					},
					position: { x: -1, y: -1 },
					type: 'branchAllEnd'
				}
				nodes.push(endNode)

				module.value.branches.forEach((branch, branchIndex) => {
					// Start node by branch

					const startNode = {
						id: `${module.id}-branch-${branchIndex}`,
						data: {
							offset: currentOffset,
							label: `Branch ${branchIndex + 1}`,
							id: module.id,
							branchIndex: branchIndex,
							modules: modules,
							eventHandlers: eventHandlers,
							...extra
						},
						position: { x: -1, y: -1 },
						type: 'branchAllStart'
					}

					nodes.push(startNode)

					addEdge(module.id, startNode.id, undefined, 'empty')

					if (branch.modules.length === 0) {
						addEdge(startNode.id, endNode.id, undefined, 'empty')
					} else {
						processModules(branch.modules, startNode, endNode)
					}
				})

				previousId = endNode.id
			} else if (module.value.type === 'forloopflow') {
				addNode(module, currentOffset, 'module')

				const startNode = {
					id: `${module.id}-start`,
					data: {
						offset: currentOffset + 50,
						id: module.id,
						module: module,
						modules: modules,
						eventHandlers: eventHandlers,
						...extra
					},
					position: { x: -1, y: -1 },
					type: 'forLoopStart'
				}

				addEdge(module.id, startNode.id, undefined, 'empty')

				const endNode = {
					id: `${module.id}-end`,
					data: {
						offset: currentOffset,
						id: module.id,
						module: module,
						modules: modules,
						eventHandlers: eventHandlers,
						...extra
					},
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
					data: {
						offset: currentOffset + 50,
						module: module,
						modules: modules,
						eventHandlers: eventHandlers,
						...extra
					},
					position: { x: -1, y: -1 },
					type: 'whileLoopStart'
				}
				addEdge(module.id, startNode.id, undefined, 'empty')

				const endNode = {
					id: `${module.id}-end`,
					data: { offset: currentOffset, module: module, modules: modules, ...extra },
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

	if (failureModule) {
		addNode(failureModule, 0, 'module')
	}

	Object.keys(parents).forEach((key) => {
		const node = nodes.find((n) => n.id === key)

		if (node) {
			node.data.parentIds = parents[key]
		}
	})

	// DATAFLOW
	if (useDataflow && selectedId) {
		let deps = getDependeeAndDependentComponents(selectedId, modules ?? [], failureModule)

		if (deps) {
			Object.entries(deps.dependees).forEach((x, i) => {
				const inputs = x[1]

				inputs?.forEach((input, index) => {
					let pid = x[0]

					if (input?.startsWith('flow_input.iter')) {
						const parent = dfsByModule(selectedId!, modules ?? [])?.pop()

						if (parent?.id) {
							pid = parent.id
						}
					}

					addEdge(
						pid,
						selectedId!,
						`dep-${pid}-${selectedId}-${input}-${index}`,
						'dataflowedge',
						i * 20
					)
				})
			})

			Object.entries(deps.dependents).forEach((x, i) => {
				let pid = x[0]

				console.log('offset', i * 10)

				addEdge(selectedId!, pid, `dep-${selectedId}-${pid}-${i}`, 'dataflowedge', i * 10)
			})
		}
	}

	return { nodes, edges }
}
