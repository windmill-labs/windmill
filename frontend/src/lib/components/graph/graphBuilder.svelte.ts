import type { FlowModule, Job, PathScript, RawScript, Script } from '$lib/gen'
import { type Edge } from '@xyflow/svelte'
import { getAllModules, getDependeeAndDependentComponents } from '../flows/flowExplorer'
import { dfsByModule } from '../flows/previousResults'
import { defaultIfEmptyString } from '$lib/utils'
import type { GraphModuleState } from './model'
import { getFlowModuleAssets, type AssetWithAltAccessType } from '../assets/lib'
import { assetDisplaysAsOutputInFlowGraph } from './renderers/nodes/AssetNode.svelte'
import type { ModulesTestStates, ModuleTestState } from '../modulesTest.svelte'
import type { ModuleActionInfo } from '$lib/components/flows/flowDiff'

export type InsertKind =
	| 'script'
	| 'forloop'
	| 'whileloop'
	| 'branchone'
	| 'branchall'
	| 'flow'
	| 'trigger'
	| 'approval'
	| 'end'
	| 'aiagent'
	| 'mcpTool'
	| 'websearchTool'
	| 'aiAgentTool'

export type InlineScript = {
	language: RawScript['language']
	kind: Script['kind']
	subkind: 'pgsql' | 'flow'
	summary?: string
	instructions?: string
}

export type onSelectedIteration = (
	detail:
		| { id: string; index: number; manuallySet: true; moduleId: string }
		| { manuallySet: false; moduleId: string }
) => void

export type GraphEventHandlers = {
	insert: (detail: {
		agentId?: string
		sourceId?: string
		targetId?: string
		branch?: { rootId: string; branch: number }
		index: number
		kind?: string
		inlineScript?: InlineScript
		script?: PathScript
		flow?: { path: string; summary: string }
		isPreprocessor?: boolean
	}) => void
	deleteBranch: (detail: { id: string; index: number }, label: string) => void
	select: (mod: string | FlowModule) => void
	delete: (detail: { id: string }, label: string) => void
	newBranch: (id: string) => void
	move: (detail: { id: string }) => void
	selectedIteration: onSelectedIteration
	changeId: (newId: string) => void
	simplifyFlow: (b: boolean) => void
	expandSubflow: (id: string, path: string) => void
	minimizeSubflow: (id: string) => void
	updateMock: (detail: { mock: FlowModule['mock']; id: string }) => void
	testUpTo: (id: string) => void
	editInput: (moduleId: string, key: string) => void
	testFlow: () => void
	cancelTestFlow: () => void
	openPreview: () => void
	hideJobStatus: () => void
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

export type NodeLayout = {
	id: string
	parentIds?: string[]
	data: {
		offset?: number
	}
	selectable?: boolean
} & FlowNode

export type FlowNode =
	| InputN
	| ModuleN
	| BranchAllStartN
	| BranchAllEndN
	| ForLoopEndN
	| ForLoopStartN
	| ResultN
	| WhileLoopStartN
	| WhileLoopEndN
	| BranchOneStartN
	| BranchOneEndN
	| SubflowBoundN
	| NoBranchN
	| TriggerN
	| AssetN
	| AssetsOverflowedN
	| AiToolN
	| NewAiToolN

export type InputN = {
	type: 'input2'
	data: {
		eventHandlers: GraphEventHandlers
		hasPreprocessor: boolean
		insertable: boolean
		moving: string | undefined
		disableAi: boolean
		cache: boolean
		earlyStop: boolean
		editMode: boolean
		isRunning: boolean
		individualStepTests: boolean
		flowJob: Job | undefined
		showJobStatus: boolean
		flowHasChanged: boolean
		chatInputEnabled: boolean
		moduleAction?: ModuleActionInfo
		assets?: AssetWithAltAccessType[] | undefined
	}
}

export type ModuleN = {
	type: 'module'
	data: {
		offset: number
		module: FlowModule
		id: string
		parentIds: string[]
		eventHandlers: GraphEventHandlers
		moving: string | undefined
		flowModuleState: GraphModuleState | undefined
		testModuleState: ModuleTestState | undefined
		insertable: boolean
		editMode: boolean
		flowJob: Job | undefined
		isOwner: boolean
		assets: AssetWithAltAccessType[] | undefined
		moduleAction: ModuleActionInfo | undefined
	}
}

export type BranchAllStartN = {
	type: 'branchAllStart'
	data: {
		offset: number
		label: string
		id: string
		branchIndex: number
		eventHandlers: GraphEventHandlers
		flowModuleState: GraphModuleState | undefined
		insertable: boolean
		branchOne: boolean
	}
}

export type BranchAllEndN = {
	type: 'branchAllEnd'
	data: {
		offset: number
		id: string
		eventHandlers: GraphEventHandlers
		flowModuleState: GraphModuleState | undefined
	}
}

export type ForLoopEndN = {
	type: 'forLoopEnd'
	data: {
		offset: number
		id: string
		eventHandlers: GraphEventHandlers
		simplifiedTriggerView: boolean
		flowModuleState: GraphModuleState | undefined
	}
}

export type ForLoopStartN = {
	type: 'forLoopStart'
	data: {
		offset: number
		id: string
		eventHandlers: GraphEventHandlers
		flowModuleState: GraphModuleState | undefined
		selectedId: string | undefined
		editMode: boolean
		simplifiedTriggerView: boolean
		module: FlowModule
	}
}

export type ResultN = {
	type: 'result'
	data: {
		success: boolean | undefined
		eventHandlers: GraphEventHandlers
		editMode: boolean
		job: Job | undefined
		showJobStatus: boolean
	}
}

export type WhileLoopStartN = {
	type: 'whileLoopStart'
	data: {
		offset: number
		eventHandlers: GraphEventHandlers
	}
}

export type WhileLoopEndN = {
	type: 'whileLoopEnd'
	data: {
		offset: number
	}
}

export type BranchOneStartN = {
	type: 'branchOneStart'
	data: {
		offset: number
		id: string
		eventHandlers: GraphEventHandlers
		flowModuleState: GraphModuleState | undefined
		selected: boolean
		insertable: boolean
		label: string
		preLabel: string | undefined
		branchIndex: number
		modules: FlowModule[]
	}
}

export type BranchOneEndN = {
	type: 'branchOneEnd'
	data: {
		offset: number
		id: string
		eventHandlers: GraphEventHandlers
		flowModuleState: GraphModuleState | undefined
	}
}

export type SubflowBoundN = {
	type: 'subflowBound'
	data: {
		offset: number
		id: string
		eventHandlers: GraphEventHandlers
		label: string
		preLabel: string | undefined
		subflowId: string
		selected: boolean
	}
}

export type NoBranchN = {
	type: 'noBranch'
	data: {
		offset: number
		id: string
		eventHandlers: GraphEventHandlers
		flowModuleState: GraphModuleState | undefined
		branchOne: boolean
		label: string
		branchIndex: number
	}
}

export type TriggerN = {
	type: 'trigger'
	data: {
		simplifiableFlow: SimplifiableFlow | undefined
		path: string
		newFlow: boolean
		isEditor: boolean
		eventHandlers: GraphEventHandlers
		disableAi: boolean
	}
}

export type AssetN = {
	type: 'asset'
	data: {
		asset: AssetWithAltAccessType
		displayedAccessType: 'r' | 'w'
	}
}

export type AssetsOverflowedN = {
	type: 'assetsOverflowed'
	data: {
		overflowedAssets: AssetWithAltAccessType[]
		displayedAccessType: 'r' | 'w'
	}
}

export type AiToolN = {
	type: 'aiTool'
	data: {
		tool: string
		type?: string
		eventHandlers: GraphEventHandlers
		moduleId: string
		insertable: boolean
		flowModuleStates: Record<string, GraphModuleState> | undefined
	}
}

export type NewAiToolN = {
	type: 'newAiTool'
	data: {
		eventHandlers: GraphEventHandlers
		agentModuleId: string
	}
}

export function topologicalSort(
	nodes: { id: string; parentIds?: string[] }[]
): { id: string; parentIds?: string[] }[] {
	const nodeMap = new Map(nodes.map((n) => [n.id, n]))
	const result: { id: string; parentIds?: string[] }[] = []
	const visited = new Set<string>()

	function visit(id: string): void {
		if (visited.has(id)) return
		visited.add(id)

		const node = nodeMap.get(id)!
		node.parentIds?.forEach(visit)
		result.push(node)
	}

	nodes.forEach((n) => visit(n.id))
	return result.reverse()
}

// input2: InputNode,
// module: ModuleNode,
// branchAllStart: BranchAllStart,
// branchAllEnd: BranchAllEndNode,
// forLoopEnd: ForLoopEndNode,
// forLoopStart: ForLoopStartNode,
// result: ResultNode,
// whileLoopStart: ForLoopStartNode,
// whileLoopEnd: ForLoopEndNode,
// branchOneStart: BranchOneStart,
// branchOneEnd: BranchOneEndNode,
// subflowBound: SubflowBound,
// noBranch: NoBranchNode,
// trigger: TriggersNode

export function graphBuilder(
	modules: FlowModule[] | undefined,
	extra: {
		disableAi: boolean
		insertable: boolean
		flowModuleStates: Record<string, GraphModuleState> | undefined
		testModuleStates: ModulesTestStates | undefined
		moduleActions?: Record<string, ModuleActionInfo>
		selectedId: string | undefined
		path: string | undefined
		newFlow: boolean
		cache: boolean
		earlyStop: boolean
		editMode: boolean
		isOwner: boolean
		isRunning: boolean
		individualStepTests: boolean
		flowJob: Job | undefined
		showJobStatus: boolean
		suspendStatus: Record<string, { job: Job; nb: number }>
		flowHasChanged: boolean
		chatInputEnabled: boolean
		additionalAssetsMap?: Record<string, AssetWithAltAccessType[]>
	},
	failureModule: FlowModule | undefined,
	preprocessorModule: FlowModule | undefined,
	eventHandlers: GraphEventHandlers,
	success: boolean | undefined,
	useDataflow: boolean | undefined,
	selectedId: string | undefined,
	moving: string | undefined,
	simplifiableFlow: SimplifiableFlow | undefined,
	flowPathForTriggerNode: string | undefined,
	expandedSubflows: Record<string, FlowModule[]>
	// triggerProps?: {
	// 	path?: string
	// 	flowIsSimplifiable?: boolean
	// }
): {
	nodes: { [key: string]: NodeLayout }
	edges: Edge[]
	error?: string | undefined
} {
	console.debug('Building graph')

	try {
		if (!modules) {
			return { nodes: {}, edges: [] }
		}

		const nodes: NodeLayout[] = []
		const edges: Edge[] = []

		function addNode(module: FlowModule, offset: number) {
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
					offset: offset,
					module: module,
					id: module.id,
					parentIds: [],
					eventHandlers: eventHandlers,
					moving: moving,
					flowModuleState: extra.flowModuleStates?.[module.id],
					testModuleState: extra.testModuleStates?.states?.[module.id],
					insertable: extra.insertable,
					editMode: extra.editMode,
					isOwner: extra.isOwner,
					flowJob: extra.flowJob,
					assets: getFlowModuleAssets(module, extra.additionalAssetsMap),
					moduleAction: extra.moduleActions?.[module.id]
				},
				type: 'module',
				selectable: true
			})

			return module.id
		}

		// TODO : Do better than this
		const nodeIdsWithOutputAssets = new Set(
			getAllModules(modules)
				.filter((m) =>
					getFlowModuleAssets(m, extra.additionalAssetsMap)?.some(assetDisplaysAsOutputInFlowGraph)
				)
				.map((m) => m.id)
		)

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
			branch: { rootId: string; branch: number } | undefined,
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
					sourceId,
					targetId,
					branch,
					moving,
					eventHandlers,
					simplifiedTriggerView: simplifiableFlow?.simplifiedFlow,
					disableMoveIds: options?.disableMoveIds,
					enableTrigger: sourceId === 'Input',
					// If the index is -1, it means that the target module is not in the modules array, so we set it to the length of the array
					index: index >= 0 ? index : (mods?.length ?? 0),
					...extra,
					insertable: extra.insertable && !options?.disableInsert && prefix == undefined,
					shouldOffsetInsertBtnDueToAssetNode: nodeIdsWithOutputAssets.has(sourceId)
				},
				selectable: false
			})
		}

		const inputAssets = extra.additionalAssetsMap?.['Input']
		const inputNode: NodeLayout = {
			id: 'Input',
			type: 'input2',
			data: {
				eventHandlers: eventHandlers,
				hasPreprocessor: !!preprocessorModule || flowPathForTriggerNode == undefined,
				insertable: extra.insertable,
				moving: moving,
				disableAi: extra.disableAi,
				cache: extra.cache,
				earlyStop: extra.earlyStop,
				editMode: extra.editMode,
				isRunning: extra.isRunning,
				individualStepTests: extra.individualStepTests,
				flowJob: extra.flowJob,
				showJobStatus: extra.showJobStatus,
				flowHasChanged: extra.flowHasChanged,
				chatInputEnabled: extra.chatInputEnabled,
				moduleAction: extra.moduleActions?.['Input'],
				...(inputAssets ? { assets: inputAssets } : {})
			}
		}

		let triggerNode: NodeLayout | undefined = undefined
		if (flowPathForTriggerNode) {
			triggerNode = {
				id: 'Trigger',
				type: 'trigger',
				data: {
					simplifiableFlow: simplifiableFlow,
					path: flowPathForTriggerNode,
					newFlow: extra.newFlow,
					eventHandlers: eventHandlers,
					isEditor: extra.insertable,
					disableAi: extra.disableAi
				}
			}
			nodes.push(triggerNode)
			if (preprocessorModule != null && preprocessorModule != undefined) {
				addEdge('Trigger', preprocessorModule.id, undefined, undefined, {
					type: 'empty'
				})
			} else {
				addEdge('Trigger', 'Input', undefined, undefined, {
					type: 'empty'
				})
			}
		}

		const resultNode: NodeLayout = {
			id: 'Result',
			data: {
				eventHandlers: eventHandlers,
				success: success,
				editMode: extra.editMode,
				job: extra.flowJob,
				showJobStatus: extra.showJobStatus
			},
			type: 'result'
		}

		if (simplifiableFlow?.simplifiedFlow !== true) {
			nodes.push(inputNode)
			nodes.push(resultNode)
		}

		function processModules(
			modules: FlowModule[],
			branch: { rootId: string; branch: number } | undefined,
			beforeNode: NodeLayout,
			nextNode: NodeLayout | undefined,
			simplifiedTriggerView: boolean,
			prefix: string | undefined,
			currentOffset = 0,
			disableMoveIds: string[] = [],
			parentIndex?: string
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
					addEdge(beforeNode.id, nextNode.id, branch, prefix, {
						subModules: modules,
						disableMoveIds
					})
				}
			} else {
				modules.forEach((module, index) => {
					const localDisableMoveIds = [...disableMoveIds, module.id]

					// Add the edge between the previous node and the current one
					if (index > 0 && previousId && expandedSubflows[module.id] == undefined) {
						addEdge(previousId, module.id, branch, prefix, {
							subModules: modules,
							disableMoveIds
						})
					}

					if (module.value.type === 'branchall') {
						// Start
						addNode(module, currentOffset)

						// "Collect result of each branch" node
						const endNode: NodeLayout = {
							id: `${module.id}-end`,
							data: {
								offset: currentOffset,
								id: module.id,
								eventHandlers: eventHandlers,
								flowModuleState: extra.flowModuleStates?.[module.id]
							},
							type: 'branchAllEnd'
						}

						nodes.push(endNode)

						if (module.value.branches.length === 0) {
							// Add a "No branches" node
							const startNode: NodeLayout = {
								id: `${module.id}-branch-0`,
								data: {
									offset: currentOffset,
									id: module.id,
									branchIndex: -1,
									eventHandlers: eventHandlers,
									flowModuleState: extra.flowModuleStates?.[module.id],
									branchOne: false,
									label: 'No branches'
								},
								type: 'noBranch'
							}

							nodes.push(startNode)

							addEdge(module.id, startNode.id, { rootId: module.id, branch: 0 }, prefix, {
								type: 'empty'
							})
							addEdge(startNode.id, endNode.id, { rootId: module.id, branch: 0 }, prefix, {
								type: 'empty'
							})
						} else {
							module.value.branches.forEach((branch, branchIndex) => {
								// Start node by branch

								const startNode: NodeLayout = {
									id: `${module.id}-branch-${branchIndex}`,
									data: {
										offset: currentOffset,
										label: defaultIfEmptyString(branch.summary, `Branch ${branchIndex + 1}`),
										id: module.id,
										branchIndex: branchIndex,
										eventHandlers: eventHandlers,
										flowModuleState: extra.flowModuleStates?.[module.id],
										insertable: extra.insertable,
										branchOne: false
									},
									type: 'branchAllStart'
								}

								nodes.push(startNode)

								addEdge(
									module.id,
									startNode.id,
									{ rootId: module.id, branch: branchIndex },
									prefix,
									{
										type: 'empty'
									}
								)

								processModules(
									branch.modules,
									{ rootId: module.id, branch: branchIndex },
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
							addNode(module, currentOffset)
						}

						const startNode: NodeLayout = {
							id: `${module.id}-start`,
							data: {
								offset: currentOffset + 25,
								id: module.id,
								module: module,
								simplifiedTriggerView,
								eventHandlers: eventHandlers,
								editMode: extra.editMode,
								flowModuleState: extra.flowModuleStates?.[module.id],
								selectedId: extra.selectedId
							},
							type: 'forLoopStart'
						}

						if (!simplifiedTriggerView) {
							addEdge(module.id, startNode.id, undefined, prefix, {
								type: 'empty'
							})
						} else if (previousId) {
							addEdge(previousId, startNode.id, undefined, prefix, {
								type: 'empty'
							})
						}

						const endNode: NodeLayout = {
							id: `${module.id}-end`,
							data: {
								offset: currentOffset,
								id: module.id,
								eventHandlers: eventHandlers,
								simplifiedTriggerView,
								flowModuleState: extra.flowModuleStates?.[module.id]
							},
							type: 'forLoopEnd'
						}

						nodes.push(startNode)
						nodes.push(endNode)

						const selectedIterIndex = extra.flowModuleStates?.[module.id]?.selectedForloopIndex

						processModules(
							module.value.modules,
							{ rootId: module.id, branch: 0 },
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
						addNode(module, currentOffset)

						const startNode: NodeLayout = {
							id: `${module.id}-start`,
							data: {
								offset: currentOffset + 25,
								eventHandlers: eventHandlers
							},
							type: 'whileLoopStart'
						}
						addEdge(module.id, startNode.id, { rootId: module.id, branch: 0 }, prefix, {
							type: 'empty'
						})

						const endNode: NodeLayout = {
							id: `${module.id}-end`,
							data: { offset: currentOffset, ...extra },
							type: 'whileLoopEnd'
						}

						nodes.push(startNode)
						nodes.push(endNode)

						const selectedIterIndex = extra.flowModuleStates?.[module.id]?.selectedForloopIndex

						processModules(
							module.value.modules,
							{ rootId: module.id, branch: 0 },
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
						addNode(module, currentOffset)

						const endNode: NodeLayout = {
							id: `${module.id}-end`,
							data: {
								offset: currentOffset,
								eventHandlers: eventHandlers,
								flowModuleState: extra.flowModuleStates?.[module.id],
								id: module.id
							},
							type: 'branchOneEnd'
						}
						nodes.push(endNode)

						// // Add default branch
						// const defaultBranch: NodeLayout = {
						// 	id: `${module.id}-default`,
						// 	data: {
						// 		offset: currentOffset,
						// 		label: 'Default',
						// 		id: module.id,
						// 		branchIndex: -1,
						// 		eventHandlers: eventHandlers,
						// 		branchOne: true,
						// 		...extra
						// 	},
						// 	type: 'noBranch'
						// }

						const defaultBranch: NodeLayout = {
							id: `${module.id}-branch-default`,
							data: {
								offset: currentOffset,
								label: 'Default',
								id: module.id,
								branchIndex: -1,
								eventHandlers: eventHandlers,
								insertable: extra.insertable,
								preLabel: undefined,
								flowModuleState: extra.flowModuleStates?.[module.id],
								selected: false,
								modules: module.value.default
							},
							type: 'branchOneStart'
						}

						nodes.push(defaultBranch)

						addEdge(module.id, defaultBranch.id, { rootId: module.id, branch: 0 }, prefix, {
							type: 'empty'
						})

						processModules(
							module.value.default,
							{ rootId: module.id, branch: 0 },
							defaultBranch,
							endNode,
							false,
							prefix,
							currentOffset,
							localDisableMoveIds,
							parentIndex ? `${parentIndex}-${index}` : index.toString()
						)

						module.value.branches.forEach((branch, branchIndex) => {
							// Start node by branch

							const startNode: NodeLayout = {
								id: `${module.id}-branch-${branchIndex}`,
								data: {
									offset: currentOffset,
									label: defaultIfEmptyString(branch.summary, 'Branch ' + (branchIndex + 1)),
									preLabel: branch.summary ? '' : branch.expr,
									id: module.id,
									branchIndex: branchIndex,
									eventHandlers: eventHandlers,
									insertable: extra.insertable,
									flowModuleState: extra.flowModuleStates?.[module.id],
									selected: false,
									modules: branch.modules
								},
								type: 'branchOneStart'
							}

							nodes.push(startNode)

							addEdge(module.id, startNode.id, { rootId: module.id, branch: branchIndex }, prefix, {
								type: 'empty'
							})

							processModules(
								branch.modules,
								{ rootId: module.id, branch: branchIndex + 1 },
								startNode,
								endNode,
								false,
								prefix,
								currentOffset,
								localDisableMoveIds,
								parentIndex ? `${parentIndex}-${index}` : index.toString()
							)
						})

						previousId = endNode.id
					} else {
						let expanded = expandedSubflows[module.id]
						if (expanded) {
							expanded = $state.snapshot(expanded)
							const startId = `${module.id}`
							const idWithoutPrefix = module.id.startsWith('subflow:')
								? module.id.substring(8)
								: module.id
							const startNode: NodeLayout = {
								id: startId,
								data: {
									offset: currentOffset,
									label: `Start of subflow ${idWithoutPrefix}`,
									id: startId,
									subflowId: module.id,
									eventHandlers: eventHandlers,
									preLabel: '',
									selected: false
								},
								type: 'subflowBound'
							}

							nodes.push(startNode)

							if (previousId) {
								addEdge(previousId!, startNode.id, undefined, prefix, {
									type: 'empty'
								})
							} else {
								addEdge(beforeNode.id, startNode.id, undefined, prefix, { type: 'empty' })
							}

							const endId = `${module.id}-subflow-end`
							const endNode: NodeLayout = {
								id: endId,
								data: {
									offset: currentOffset,
									label: `End of subflow ${idWithoutPrefix}`,
									id: endId,
									subflowId: module.id,
									eventHandlers: eventHandlers,
									preLabel: '',
									selected: false
								},
								type: 'subflowBound'
							}

							nodes.push(endNode)

							processModules(
								expanded,
								undefined,
								startNode,
								endNode,
								false,
								buildPrefix(prefix, module['oid'] ?? module.id),
								currentOffset,
								localDisableMoveIds
							)

							previousId = endNode.id
						} else {
							addNode(module, currentOffset)
							previousId = module.id
						}
					}

					if (index === 0 && expandedSubflows[module.id] == undefined) {
						addEdge(beforeNode.id, module.id, undefined, prefix, {
							subModules: modules,
							disableMoveIds,
							disableInsert: simplifiedTriggerView
						})
					}

					if (index === modules.length - 1 && previousId && nextNode) {
						addEdge(previousId, nextNode.id, branch, prefix, {
							subModules: modules,
							disableMoveIds
						})
					}
				})
			}
		}

		if (simplifiableFlow?.simplifiedFlow === true && triggerNode) {
			processModules(modules, undefined, triggerNode, undefined, true, undefined)
		} else {
			processModules(modules, undefined, inputNode, resultNode, false, undefined)
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
				addNode({ ...failureModule, id: x[1] }, 0)
				addEdge(x[0], x[1], undefined, undefined, { type: 'empty' })
			})
		}

		if (preprocessorModule) {
			addNode(preprocessorModule, 0)
			const id = JSON.parse(JSON.stringify(preprocessorModule.id))
			addEdge(id, 'Input', undefined, undefined, { type: 'empty' })
		}

		if (failureModule && !extra.flowModuleStates) {
			addNode(failureModule, 0)
		}

		Object.keys(parents).forEach((key) => {
			const node = nodes.find((n) => n.id === key)

			if (node) {
				node.parentIds = parents[key]
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
							const parent = dfsByModule(selectedId, modules ?? [])?.pop()

							if (parent?.id) {
								pid = parent.id
							}
						}

						addEdge(pid, selectedId, undefined, undefined, {
							customId: `dep-${pid}-${selectedId}-${input}-${index}`,
							type: 'dataflowedge'
						})
					})
				})

				Object.entries(deps.dependents).forEach((x, i) => {
					let pid = x[0]

					addEdge(selectedId, pid, undefined, undefined, {
						customId: `dep-${selectedId}-${pid}-${i}`,
						type: 'dataflowedge'
					})
				})
			}
		}

		return { nodes: Object.fromEntries(nodes.map((n) => [n.id, n])), edges }
	} catch (e) {
		return {
			nodes: {},
			edges: [],
			error: e
		}
	}
}
