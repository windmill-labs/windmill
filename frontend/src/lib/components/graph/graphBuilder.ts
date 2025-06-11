import type { FlowModule } from '$lib/gen'
import { type Node, type Edge } from '@xyflow/svelte'
import { getDependeeAndDependentComponents } from '../flows/flowExplorer'
import { dfsByModule } from '../flows/previousResults'
import { defaultIfEmptyString } from '$lib/utils'
import type { GraphModuleState } from './model'

export type GraphEventHandlers = {
	insert: (detail) => void
	deleteBranch: (detail, label: string) => void
	select: (modId: string) => void
	delete: (detail, label: string) => void
	newBranch: (module: FlowModule) => void
	move: (module: FlowModule, modules: FlowModule[]) => void
	selectedIteration: (detail, moduleId: string) => void
	changeId: (newId: string) => void
	simplifyFlow: (detail: boolean) => void
	expandSubflow: (id: string, path: string) => void
	minimizeSubflow: (id: string) => void
	updateMock: () => void
}

export type SimplifiableFlow = { simplifiedFlow: boolean }

export function isTriggerStep(module: FlowModule | undefined): boolean {
	return (
		module?.value != undefined &&
		(module.value.type === 'script' || module.value.type === 'rawscript') &&
		module.value.is_trigger === true
	)
}

export function buildPrefix(prefix: string | undefined, id: string): string {
	return (prefix ?? '') + id + ':'
}
export function graphBuilder(
	modules: FlowModule[] | undefined,
	extra: Record<string, any>,
	failureModule: FlowModule | undefined,
	preprocessorModule: FlowModule | undefined,
	eventHandlers: GraphEventHandlers,
	success: boolean | undefined,
	useDataflow: boolean | undefined,
	selectedId: string | undefined,
	moving: string | undefined,
	simplifiableFlow: SimplifiableFlow | undefined,
	flowPathForTriggerNode: string | undefined,
	expandedSubflows: Record<string, FlowModule[]>,
	onTestUpTo: ((id: string) => void) | undefined
	// triggerProps?: {
	// 	path?: string
	// 	flowIsSimplifiable?: boolean
	// }
): {
	nodes: Node[]
	edges: Edge[]
	error?: string | undefined
} {
	console.debug('Building graph')
	const nodes: Node[] = []
	const edges: Edge[] = []
	try {
		if (!modules) {
			return { nodes, edges }
		}

		function addNode(module: FlowModule, offset: number, type: string, subModules?: FlowModule[]) {
			const duplicated = nodes.find((n) => n.id === module.id)
			if (duplicated) {
				console.log('Duplicated node detected: ', module, duplicated)
				throw new Error(`Duplicated node detected: ${module.id}`)
			}

			if (module.id.startsWith('subflow:')) {
				extra.insertable = false
			}

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
					onTestUpTo: onTestUpTo,
					...extra
				},
				position: { x: -1, y: -1 },
				type: type
			})

			return module.id
		}

		const parents: { [key: string]: string[] } = {}

		//
		function detectCycle(nodeId: string, visited: Set<string>, currentPath: Set<string>): boolean {
			// If the node hasn't been visited yet
			if (!visited.has(nodeId)) {
				visited.add(nodeId)
				currentPath.add(nodeId)

				// If the current node has parent nodes
				if (parents[nodeId]) {
					// Check each parent node
					// Nodes can have multiple parents: the node that gathers the result for branches or loops for instance
					for (const parentNode of parents[nodeId]) {
						// If the parentNode hasn't been visited and a cycle is detected in its path
						if (!visited.has(parentNode) && detectCycle(parentNode, visited, currentPath)) {
							return true // Cycle detected
						}
						// If the parentNode is already in the current path, it's a cycle
						else if (currentPath.has(parentNode)) {
							return true // Cycle detected
						}
					}
				}
			}
			// Remove the node from the current path as we're done processing it
			currentPath.delete(nodeId)
			// No cycle detected for this path
			return false
		}

		function addEdge(
			sourceId: string,
			targetId: string,
			prefix: string | undefined,
			options?: {
				disableInsert?: boolean
				customId?: string
				type?: string
				subModules?: FlowModule[]
				disableMoveIds?: string[]
			}
		) {
			parents[targetId] = [...(parents[targetId] ?? []), sourceId]

			const mods = options?.subModules ?? modules

			let index = mods?.findIndex((m) => m.id === targetId) ?? -1

			const visited = new Set<string>()
			const recStack = new Set<string>()

			if (detectCycle(sourceId, visited, recStack)) {
				throw new Error(
					`Cycle detected: adding edge from '${sourceId}' to '${targetId}' would create a cycle.`
				)
			}

			edges.push({
				id: options?.customId || `edge:${sourceId}->${targetId}`,
				source: sourceId,
				target: targetId,
				type: options?.type ?? 'edge',
				data: {
					modules: options?.subModules ?? modules,
					sourceId,
					targetId,
					moving,
					eventHandlers,
					simplifiedTriggerView: simplifiableFlow?.simplifiedFlow,
					disableMoveIds: options?.disableMoveIds,
					enableTrigger: sourceId === 'Input',
					// If the index is -1, it means that the target module is not in the modules array, so we set it to the length of the array
					index: index >= 0 ? index : (mods?.length ?? 0),
					...extra,
					insertable: extra.insertable && !options?.disableInsert && prefix == undefined
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
				hasPreprocessor: !!preprocessorModule || flowPathForTriggerNode == undefined,
				...extra
			}
		}

		let triggerNode: Node | undefined = undefined
		if (flowPathForTriggerNode) {
			triggerNode = {
				id: 'Trigger',
				position: { x: -1, y: -1 },
				type: 'trigger',
				data: {
					simplifiableFlow: simplifiableFlow,
					path: flowPathForTriggerNode,
					newFlow: extra.newFlow,
					eventHandlers: eventHandlers,
					modules: modules,
					isEditor: extra.insertable
				}
			}
			nodes.push(triggerNode)
			if (preprocessorModule != null && preprocessorModule != undefined) {
				addEdge('Trigger', preprocessorModule.id, undefined, {
					type: 'empty'
				})
			} else {
				addEdge('Trigger', 'Input', undefined, {
					type: 'empty'
				})
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

		if (simplifiableFlow?.simplifiedFlow !== true) {
			nodes.push(inputNode)
			nodes.push(resultNode)
		}

		function processModules(
			modules: FlowModule[],
			beforeNode: Node,
			nextNode: Node | undefined,
			simplifiedTriggerView: boolean,
			prefix: string | undefined,
			currentOffset = 0,
			disableMoveIds: string[] = [],
			parentIndex?: string,
			branchChosen?: boolean
		) {
			if (prefix != undefined) {
				modules.forEach((m) => {
					if (!m['oid']) {
						m['oid'] = m.id
					}
					m.id = 'subflow:' + prefix + m['oid']
				})
			}
			let previousId: string | undefined = undefined

			if (modules.length === 0) {
				if (nextNode) {
					addEdge(beforeNode.id, nextNode.id, prefix, {
						subModules: modules,
						disableMoveIds
					})
				}
			} else {
				modules.forEach((module, index) => {
					const localDisableMoveIds = [...disableMoveIds, module.id]

					// Add the edge between the previous node and the current one
					if (index > 0 && previousId && expandedSubflows[module.id] == undefined) {
						addEdge(previousId, module.id, prefix, {
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

							addEdge(module.id, startNode.id, prefix, {
								type: 'empty'
							})
							addEdge(startNode.id, endNode.id, prefix, {
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

								addEdge(module.id, startNode.id, prefix, {
									type: 'empty'
								})

								processModules(
									branch.modules,
									startNode,
									endNode,
									false,
									prefix,
									currentOffset,
									localDisableMoveIds,
									parentIndex ? `${parentIndex}-${index}-${branchIndex}` : `${index}-${branchIndex}`
								)
							})
						}

						previousId = endNode.id
					} else if (module.value.type === 'forloopflow') {
						if (!simplifiedTriggerView) {
							addNode(module, currentOffset, 'module', modules)
						}

						const startNode = {
							id: `${module.id}-start`,
							data: {
								offset: currentOffset + 25,
								id: module.id,
								module: module,
								modules: modules,
								simplifiedTriggerView,
								eventHandlers: eventHandlers,
								...extra
							},
							position: { x: -1, y: -1 },
							type: 'forLoopStart'
						}

						if (!simplifiedTriggerView) {
							addEdge(module.id, startNode.id, prefix, {
								type: 'empty'
							})
						} else if (previousId) {
							addEdge(previousId, startNode.id, prefix, {
								type: 'empty'
							})
						}

						const endNode = {
							id: `${module.id}-end`,
							data: {
								offset: currentOffset,
								id: module.id,
								module: module,
								modules: modules,
								eventHandlers: eventHandlers,
								simplifiedTriggerView,
								...extra
							},
							position: { x: -1, y: -1 },
							type: 'forLoopEnd'
						}

						nodes.push(startNode)
						nodes.push(endNode)

						const selectedIterIndex = extra.flowModuleStates?.[module.id]?.selectedForloopIndex

						processModules(
							module.value.modules,
							startNode,
							endNode,
							false,
							prefix,
							currentOffset + 25,
							localDisableMoveIds,
							parentIndex
								? `${parentIndex}-${index}-${selectedIterIndex ?? '?'}`
								: `${index}-${selectedIterIndex ?? '?'}`
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
						addEdge(module.id, startNode.id, prefix, {
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

						const selectedIterIndex = extra.flowModuleStates?.[module.id]?.selectedForloopIndex

						processModules(
							module.value.modules,
							startNode,
							endNode,
							false,
							prefix,
							currentOffset + 25,
							localDisableMoveIds,
							parentIndex
								? `${parentIndex}-${index}-${selectedIterIndex ?? '?'}`
								: `${index}-${selectedIterIndex ?? '?'}`
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

						addEdge(module.id, defaultBranch.id, prefix, { type: 'empty' })

						const branchChosen = extra.flowModuleStates?.[module.id]?.branchChosen
						processModules(
							module.value.default,
							defaultBranch,
							endNode,
							false,
							prefix,
							currentOffset,
							localDisableMoveIds,
							parentIndex ? `${parentIndex}-${index}` : index.toString(),
							branchChosen == 0
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

							addEdge(module.id, startNode.id, prefix, { type: 'empty' })

							processModules(
								branch.modules,
								startNode,
								endNode,
								false,
								prefix,
								currentOffset,
								localDisableMoveIds,
								parentIndex ? `${parentIndex}-${index}` : index.toString(),
								branchChosen == branchIndex + 1
							)
						})

						previousId = endNode.id
					} else {
						let expanded = expandedSubflows[module.id]
						if (expanded) {
							const startId = `${module.id}-subflow-start`
							const idWithoutPrefix = module.id.startsWith('subflow:')
								? module.id.substring(8)
								: module.id
							const startNode = {
								id: startId,
								data: {
									offset: currentOffset,
									label: `Start of subflow ${idWithoutPrefix}`,
									id: startId,
									subflowId: module.id,
									modules: modules,
									eventHandlers: eventHandlers,
									...extra
								},
								position: { x: -1, y: -1 },
								type: 'subflowBound'
							}

							nodes.push(startNode)

							if (previousId) {
								addEdge(previousId!, startNode.id, prefix, { type: 'empty' })
							} else {
								addEdge(beforeNode.id, startNode.id, prefix, { type: 'empty' })
							}

							const endId = `${module.id}-subflow-end`
							const endNode = {
								id: endId,
								data: {
									offset: currentOffset,
									label: `End of subflow ${idWithoutPrefix}`,
									id: endId,
									subflowId: module.id,
									modules: modules,
									eventHandlers: eventHandlers,
									...extra
								},
								position: { x: -1, y: -1 },
								type: 'subflowBound'
							}

							nodes.push(endNode)

							processModules(
								expanded,
								startNode,
								endNode,
								false,
								buildPrefix(prefix, module['oid'] ?? module.id),
								currentOffset,
								localDisableMoveIds
							)

							previousId = endNode.id
						} else {
							addNode(module, currentOffset, 'module', modules)
							previousId = module.id
						}
					}

					if (index === 0 && expandedSubflows[module.id] == undefined) {
						addEdge(beforeNode.id, module.id, prefix, {
							subModules: modules,
							disableMoveIds,
							disableInsert: simplifiedTriggerView
						})
					}

					if (index === modules.length - 1 && previousId && nextNode) {
						addEdge(previousId, nextNode.id, prefix, {
							subModules: modules,
							disableMoveIds
						})
					}
				})
			}
		}

		if (simplifiableFlow?.simplifiedFlow === true && triggerNode) {
			processModules(modules, triggerNode, undefined, true, undefined)
		} else {
			processModules(modules, inputNode, resultNode, false, undefined)
		}

		if (failureModule) {
			let toAdd: Record<string, string> = {}
			Object.keys(extra.flowModuleStates ?? {}).forEach((id) => {
				if (id.startsWith('failure')) {
					const failureState = extra.flowModuleStates?.[id] as GraphModuleState | undefined
					if (failureState?.parent_module) {
						toAdd[failureState.parent_module] = id
					}
				}
			})

			Object.entries(toAdd).forEach((x) => {
				addNode({ ...failureModule, id: x[1] }, 0, 'module')
				addEdge(x[0], x[1], undefined, { type: 'empty' })
			})
		}

		if (preprocessorModule) {
			addNode(preprocessorModule, 0, 'module')
			const id = JSON.parse(JSON.stringify(preprocessorModule.id))
			addEdge(id, 'Input', undefined, { type: 'empty' })
		}

		if (failureModule && !extra.flowModuleStates) {
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

						addEdge(pid, selectedId!, undefined, {
							customId: `dep-${pid}-${selectedId}-${input}-${index}`,
							type: 'dataflowedge'
						})
					})
				})

				Object.entries(deps.dependents).forEach((x, i) => {
					let pid = x[0]

					addEdge(selectedId!, pid, undefined, {
						customId: `dep-${selectedId}-${pid}-${i}`,
						type: 'dataflowedge'
					})
				})
			}
		}

		return { nodes, edges }
	} catch (e) {
		return {
			nodes: [],
			edges: [],
			error: e
		}
	}
}
