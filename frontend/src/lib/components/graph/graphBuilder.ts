import type { FlowModule } from '$lib/gen'
import { type Node, type Edge } from '@xyflow/svelte'
import { getDependeeAndDependentComponents } from '../flows/flowExplorer'
import { dfsByModule } from '../flows/previousResults'
import { defaultIfEmptyString } from '$lib/utils'

export type GraphEventHandlers = {
	insert: (detail) => void
	deleteBranch: (detail, label: string) => void
	select: (modId: string) => void
	delete: (detail, label: string) => void
	newBranch: (module: FlowModule) => void
	move: (module: FlowModule, modules: FlowModule[]) => void
	selectedIteration: (detail, moduleId: string) => void
	changeId: (newId: string) => void
}

export default function graphBuilder(
	modules: FlowModule[] | undefined,
	extra: Record<string, any>,
	failureModule: FlowModule | undefined,
	eventHandlers: GraphEventHandlers,
	success: boolean | undefined,
	useDataflow: boolean | undefined,
	selectedId: string | undefined,
	moving: string | undefined
): {
	nodes: Node[]
	edges: Edge[]
} {
	const nodes: Node[] = []
	const edges: Edge[] = []

	if (!modules) {
		return { nodes, edges }
	}

	function addNode(module: FlowModule, offset: number, type: string, subModules?: FlowModule[]) {
		nodes.push({
			id: module.id,
			data: {
				value: module.value,
				offset: offset,
				module: module,
				modules: subModules ?? modules,
				parentIds: [],
				eventHandlers: eventHandlers,
				moving: moving,
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
		options?: {
			customId?: string
			type?: string
			subModules?: FlowModule[]
			disableMoveIds?: string[]
		}
	) {
		parents[targetId] = [...(parents[targetId] ?? []), sourceId]

		// Find the index of the target module in the modules array
		const mods = options?.subModules ?? modules

		// Index of the target module in the modules array
		let index = mods?.findIndex((m) => m.id === targetId) ?? -1

		edges.push({
			id: options?.customId || `edge:${sourceId}->${targetId}`,
			source: sourceId,
			target: targetId,
			type: options?.type ?? 'edge',
			data: {
				insertable: extra.insertable,
				modules: options?.subModules ?? modules,
				sourceId,
				targetId,
				moving,
				eventHandlers,
				disableMoveIds: options?.disableMoveIds,
				enableTrigger: sourceId === 'Input',
				// If the index is -1, it means that the target module is not in the modules array, so we set it to the length of the array
				index: index >= 0 ? index : mods?.length ?? 0,
				...extra
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
		currentOffset = 0,
		disableMoveIds: string[] = []
	) {
		let previousId: string | undefined = undefined

		if (modules.length === 0) {
			addEdge(beforeNode.id, nextNode.id, {
				subModules: modules,
				disableMoveIds
			})
		} else {
			modules.forEach((module, index) => {
				const localDisableMoveIds = [...disableMoveIds, module.id]

				// Add the edge between the previous node and the current one
				if (index > 0 && previousId) {
					addEdge(previousId, module.id, {
						subModules: modules,
						disableMoveIds
					})
				}

				if (module.value.type === 'branchall') {
					// Start
					addNode(module, currentOffset, 'module', modules)

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

					if (module.value.branches.length === 0) {
						// Add a "No branches" node
						const startNode = {
							id: `${module.id}-branch-0`,
							data: {
								offset: currentOffset,
								id: module.id,
								branchIndex: -1,
								modules: modules,
								eventHandlers: eventHandlers,
								...extra
							},
							position: { x: -1, y: -1 },
							type: 'noBranch'
						}

						nodes.push(startNode)

						addEdge(module.id, startNode.id, {
							type: 'empty'
						})
						addEdge(startNode.id, endNode.id, {
							type: 'empty'
						})
					} else {
						module.value.branches.forEach((branch, branchIndex) => {
							// Start node by branch

							const startNode = {
								id: `${module.id}-branch-${branchIndex}`,
								data: {
									offset: currentOffset,
									label: defaultIfEmptyString(branch.summary, `Branch ${branchIndex + 1}`),
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

							addEdge(module.id, startNode.id, {
								type: 'empty'
							})

							processModules(branch.modules, startNode, endNode, currentOffset, localDisableMoveIds)
						})
					}

					previousId = endNode.id
				} else if (module.value.type === 'forloopflow') {
					addNode(module, currentOffset, 'module', modules)

					const startNode = {
						id: `${module.id}-start`,
						data: {
							offset: currentOffset + 25,
							id: module.id,
							module: module,
							modules: modules,
							eventHandlers: eventHandlers,
							...extra
						},
						position: { x: -1, y: -1 },
						type: 'forLoopStart'
					}

					addEdge(module.id, startNode.id, {
						type: 'empty'
					})

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

					processModules(
						module.value.modules,
						startNode,
						endNode,
						currentOffset + 25,
						localDisableMoveIds
					)

					previousId = endNode.id
				} else if (module.value.type === 'whileloopflow') {
					addNode(module, currentOffset, 'module', modules)

					const startNode = {
						id: `${module.id}-start`,
						data: {
							offset: currentOffset + 25,
							module: module,
							modules: modules,
							eventHandlers: eventHandlers,
							...extra
						},
						position: { x: -1, y: -1 },
						type: 'whileLoopStart'
					}
					addEdge(module.id, startNode.id, {
						type: 'empty'
					})

					const endNode = {
						id: `${module.id}-end`,
						data: { offset: currentOffset, module: module, modules: modules, ...extra },
						position: { x: -1, y: -1 },
						type: 'whileLoopEnd'
					}

					nodes.push(startNode)
					nodes.push(endNode)

					processModules(
						module.value.modules,
						startNode,
						endNode,
						currentOffset + 25,
						localDisableMoveIds
					)

					previousId = endNode.id
				} else if (module.value.type === 'branchone') {
					addNode(module, currentOffset, 'module', modules)

					const endNode = {
						id: `${module.id}-end`,
						data: { offset: currentOffset, eventHandlers: eventHandlers },
						position: { x: -1, y: -1 },
						type: 'branchOneEnd'
					}
					nodes.push(endNode)

					// Add default branch
					const defaultBranch = {
						id: `${module.id}-default`,
						data: {
							offset: currentOffset,
							label: 'Default',
							id: module.id,
							branchIndex: -1,
							modules: module.value.default,
							eventHandlers: eventHandlers,
							branchOne: true,
							...extra
						},
						position: { x: -1, y: -1 },
						type: 'noBranch'
					}

					nodes.push(defaultBranch)

					addEdge(module.id, defaultBranch.id, { type: 'empty' })

					processModules(
						module.value.default,
						defaultBranch,
						endNode,
						currentOffset,
						localDisableMoveIds
					)

					module.value.branches.forEach((branch, branchIndex) => {
						// Start node by branch

						const startNode = {
							id: `${module.id}-branch-${branchIndex}`,
							data: {
								offset: currentOffset,
								label: defaultIfEmptyString(branch.summary, 'Branch ' + (branchIndex + 1)),
								preLabel: branch.summary ? '' : branch.expr,
								id: module.id,
								branchIndex: branchIndex,
								modules: modules,
								eventHandlers: eventHandlers,
								...extra
							},
							position: { x: -1, y: -1 },
							type: 'branchOneStart'
						}

						nodes.push(startNode)

						addEdge(module.id, startNode.id, { type: 'empty' })

						processModules(branch.modules, startNode, endNode, currentOffset, localDisableMoveIds)
					})

					previousId = endNode.id
				} else {
					addNode(module, currentOffset, 'module', modules)

					previousId = module.id
				}

				if (index === 0) {
					addEdge(beforeNode.id, module.id, {
						subModules: modules,
						disableMoveIds
					})
				}

				if (index === modules.length - 1 && previousId) {
					addEdge(previousId, nextNode.id, {
						subModules: modules,
						disableMoveIds
					})
				}
			})
		}
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

					addEdge(pid, selectedId!, {
						customId: `dep-${pid}-${selectedId}-${input}-${index}`,
						type: 'dataflowedge'
					})
				})
			})

			Object.entries(deps.dependents).forEach((x, i) => {
				let pid = x[0]

				addEdge(selectedId!, pid, {
					customId: `dep-${selectedId}-${pid}-${i}`,
					type: 'dataflowedge'
				})
			})
		}
	}

	return { nodes, edges }
}
