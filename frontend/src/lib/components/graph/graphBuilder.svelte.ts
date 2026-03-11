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
	subkind: 'pgsql' | 'flow' | 'claudesandbox'
	summary?: string
	instructions?: string
}

export type OnSelectedIteration = (
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
	duplicate: (detail: { id: string }) => void
	selectedIteration: OnSelectedIteration
	changeId: (newId: string) => void
	simplifyFlow: (b: boolean) => void
	expandSubflow: (id: string, path: string) => void
	minimizeSubflow: (id: string) => void
	expandGroup: (groupId: string) => void
	expandContainer: (moduleId: string) => void
	collapseContainer: (moduleId: string) => void
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
	data: {}
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
	| CollapsedSubflowN
	| NoBranchN
	| TriggerN
	| AssetN
	| AssetsOverflowedN
	| AiToolN
	| NewAiToolN
	| CollapsedGroupN

export type InputN = {
	type: 'input2'
	data: {
		eventHandlers: GraphEventHandlers
		hasPreprocessor: boolean
		insertable: boolean
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
		module: FlowModule
		id: string
		parentIds: string[]
		eventHandlers: GraphEventHandlers
		flowModuleState: GraphModuleState | undefined
		testModuleState: ModuleTestState | undefined
		insertable: boolean
		editMode: boolean
		flowJob: Job | undefined
		isOwner: boolean
		assets: AssetWithAltAccessType[] | undefined
		moduleAction: ModuleActionInfo | undefined
		isCollapsedContainer?: boolean
		containerModules?: FlowModule[]
	}
}

export type BranchAllStartN = {
	type: 'branchAllStart'
	data: {
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
		id: string
		eventHandlers: GraphEventHandlers
		flowModuleState: GraphModuleState | undefined
	}
}

export type ForLoopEndN = {
	type: 'forLoopEnd'
	data: {
		id: string
		eventHandlers: GraphEventHandlers
		simplifiedTriggerView: boolean
		flowModuleState: GraphModuleState | undefined
	}
}

export type ForLoopStartN = {
	type: 'forLoopStart'
	data: {
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
		eventHandlers: GraphEventHandlers
	}
}

export type WhileLoopEndN = {
	type: 'whileLoopEnd'
	data: {}
}

export type BranchOneStartN = {
	type: 'branchOneStart'
	data: {
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
		id: string
		eventHandlers: GraphEventHandlers
		flowModuleState: GraphModuleState | undefined
	}
}

export type CollapsedSubflowN = {
	type: 'collapsedSubflow'
	data: {
		id: string
		module: FlowModule
		eventHandlers: GraphEventHandlers
		editMode: boolean
		expanded: boolean
		innerNodeIds?: string[]
	}
}

export type NoBranchN = {
	type: 'noBranch'
	data: {
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

export type CollapsedGroupN = {
	type: 'collapsedGroup'
	data: {
		groupId: string
		summary: string | undefined
		note: string | undefined
		color: string | undefined
		stepCount: number
		modules: FlowModule[]
		showNotes: boolean
		editMode: boolean
		eventHandlers: GraphEventHandlers
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
// collapsedSubflow: CollapsedSubflowNode,
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
	simplifiableFlow: SimplifiableFlow | undefined,
	flowPathForTriggerNode: string | undefined,
	expandedSubflows: Record<string, FlowModule[]>,
	collapsedGroups: Array<{
		id: string
		summary?: string
		note?: string
		color?: string
		collapsed?: boolean
		module_ids: string[]
	}>,
	collapsedContainers: Set<string>,
	showNotes: boolean
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

		// Build a map: module_id -> collapsed group (only for collapsed groups)
		const moduleToCollapsedGroup = new Map<string, (typeof collapsedGroups)[number]>()
		for (const group of collapsedGroups) {
			for (const moduleId of group.module_ids) {
				moduleToCollapsedGroup.set(moduleId, group)
			}
		}
		const emittedCollapsedGroups = new Set<string>()

		const nodes: NodeLayout[] = []
		const edges: Edge[] = []

		function addNode(
			module: FlowModule,
			extraData?: { isCollapsedContainer?: boolean; containerModules?: FlowModule[] }
		) {
			const duplicated = nodes.find((n) => n.id === module.id)
			if (duplicated) {
				console.log('Duplicated node detected: ', module, duplicated)
				throw new Error(`Duplicated node detected: ${module.id}`)
			}

			nodes.push({
				id: module.id,
				data: {
					module: module,
					id: module.id,
					parentIds: [],
					eventHandlers: eventHandlers,
					flowModuleState: extra.flowModuleStates?.[module.id],
					testModuleState: extra.testModuleStates?.states?.[module.id],
					insertable: extra.insertable && !module.id.startsWith('subflow:'),
					editMode: extra.editMode,
					isOwner: extra.isOwner,
					flowJob: extra.flowJob,
					assets: getFlowModuleAssets(module, extra.additionalAssetsMap),
					moduleAction: extra.moduleActions?.[module.id],
					...extraData
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

		function getContainerModules(module: FlowModule): FlowModule[] {
			const v = module.value
			if (v.type === 'branchall') return v.branches.flatMap((b) => b.modules)
			if (v.type === 'branchone') return [...v.default, ...v.branches.flatMap((b) => b.modules)]
			if (v.type === 'forloopflow' || v.type === 'whileloopflow') return v.modules
			return []
		}

		const inputAssets = extra.additionalAssetsMap?.['Input']
		const inputNode: NodeLayout = {
			id: 'Input',
			type: 'input2',
			data: {
				eventHandlers: eventHandlers,
				hasPreprocessor: !!preprocessorModule || flowPathForTriggerNode == undefined,
				insertable: extra.insertable,
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
			disableMoveIds: string[] = [],
			parentIndex?: string
		): string | undefined {
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
				return previousId
			} else {
				modules.forEach((module, index) => {
					// --- Collapsed group handling ---
					const collapsedGroup = moduleToCollapsedGroup.get(module.id)
					if (collapsedGroup) {
						if (emittedCollapsedGroups.has(collapsedGroup.id)) {
							// Already emitted — skip entirely, but wire final edge if last module
							if (index === modules.length - 1 && previousId && nextNode) {
								addEdge(previousId, nextNode.id, branch, prefix, {
									subModules: modules,
									disableMoveIds
								})
							}
							return
						}

						// First member — emit placeholder node
						emittedCollapsedGroups.add(collapsedGroup.id)
						const nodeId = `collapsed-group:${collapsedGroup.id}`
						nodes.push({
							id: nodeId,
							type: 'collapsedGroup',
							data: {
								groupId: collapsedGroup.id,
								summary: collapsedGroup.summary,
								note: collapsedGroup.note,
								color: collapsedGroup.color,
								stepCount: collapsedGroup.module_ids.length,
								modules: collapsedGroup.module_ids
									.map((id) => getAllModules(modules).find((m) => m.id === id))
									.filter((m): m is FlowModule => m != null),
								showNotes,
								editMode: extra.editMode,
								eventHandlers
							}
						} as NodeLayout)

						// Edge from previous → group node
						if (index === 0) {
							addEdge(beforeNode.id, nodeId, undefined, prefix, {
								subModules: modules,
								disableMoveIds,
								disableInsert: simplifiedTriggerView
							})
						} else if (previousId) {
							addEdge(previousId, nodeId, branch, prefix, {
								subModules: modules,
								disableMoveIds
							})
						}
						previousId = nodeId

						// Final edge if last module
						if (index === modules.length - 1 && nextNode) {
							addEdge(nodeId, nextNode.id, branch, prefix, {
								subModules: modules,
								disableMoveIds
							})
						}
						return
					}
					// --- End collapsed group handling ---

					// --- Collapsed container handling ---
					const isContainer =
						module.value.type === 'branchall' ||
						module.value.type === 'branchone' ||
						module.value.type === 'forloopflow' ||
						module.value.type === 'whileloopflow'
					if (isContainer && collapsedContainers.has(module.id)) {
						const containerModules = getContainerModules(module)
						addNode(module, { isCollapsedContainer: true, containerModules })

						if (index === 0) {
							addEdge(beforeNode.id, module.id, undefined, prefix, {
								subModules: modules,
								disableMoveIds,
								disableInsert: simplifiedTriggerView
							})
						} else if (previousId) {
							addEdge(previousId, module.id, branch, prefix, {
								subModules: modules,
								disableMoveIds
							})
						}

						if (index === modules.length - 1 && nextNode) {
							addEdge(module.id, nextNode.id, branch, prefix, {
								subModules: modules,
								disableMoveIds
							})
						}

						previousId = module.id
						return
					}
					// --- End collapsed container handling ---

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
						addNode(module)

						// "Collect result of each branch" node
						const endNode: NodeLayout = {
							id: `${module.id}-end`,
							data: {
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
									localDisableMoveIds,
									parentIndex ? `${parentIndex}-${index}-${branchIndex}` : `${index}-${branchIndex}`
								)
							})
						}

						previousId = endNode.id
					} else if (module.value.type === 'forloopflow') {
						if (!simplifiedTriggerView) {
							addNode(module)
						}

						const startNode: NodeLayout = {
							id: `${module.id}-start`,
							data: {
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
							localDisableMoveIds,
							parentIndex
								? `${parentIndex}-${index}-${selectedIterIndex ?? '?'}`
								: `${index}-${selectedIterIndex ?? '?'}`
						)

						previousId = endNode.id
					} else if (module.value.type === 'whileloopflow') {
						addNode(module)

						const startNode: NodeLayout = {
							id: `${module.id}-start`,
							data: {
								eventHandlers: eventHandlers
							},
							type: 'whileLoopStart'
						}
						addEdge(module.id, startNode.id, { rootId: module.id, branch: 0 }, prefix, {
							type: 'empty'
						})

						const endNode: NodeLayout = {
							id: `${module.id}-end`,
							data: { ...extra },
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
							localDisableMoveIds,
							parentIndex
								? `${parentIndex}-${index}-${selectedIterIndex ?? '?'}`
								: `${index}-${selectedIterIndex ?? '?'}`
						)

						previousId = endNode.id
					} else if (module.value.type === 'branchone') {
						addNode(module)

						const endNode: NodeLayout = {
							id: `${module.id}-end`,
							data: {
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
						// 		offset: 0,
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
							localDisableMoveIds,
							parentIndex ? `${parentIndex}-${index}` : index.toString()
						)

						module.value.branches.forEach((branch, branchIndex) => {
							// Start node by branch

							const startNode: NodeLayout = {
								id: `${module.id}-branch-${branchIndex}`,
								data: {
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
								localDisableMoveIds,
								parentIndex ? `${parentIndex}-${index}` : index.toString()
							)
						})

						previousId = endNode.id
					} else {
						let expanded = expandedSubflows[module.id]
						if (expanded) {
							expanded = $state.snapshot(expanded)

							// Emit subflow header node (replaces startNode)
							const headerNode: NodeLayout = {
								id: module.id,
								data: {
									label: `Start of subflow ${idWithoutPrefix}`,
									id: module.id,
									module: module,
									subflowId: module.id,
									eventHandlers: eventHandlers,
									editMode: extra.editMode,
									expanded: true
								},
								type: 'collapsedSubflow'
							}
							nodes.push(headerNode)

							if (previousId) {
								addEdge(previousId, headerNode.id, undefined, prefix, {
									type: 'empty'
								})
							} else {
								addEdge(beforeNode.id, headerNode.id, undefined, prefix, { type: 'empty' })
							}

							// Process inner modules — no end node, outer loop handles wiring
							const nodesBefore = nodes.length
							const lastInnerNodeId = processModules(
								expanded,
								undefined,
								headerNode,
								undefined,
								false,
								buildPrefix(prefix, module['oid'] ?? module.id),
								localDisableMoveIds
							)

							// Collect inner node IDs for border rendering
							;(headerNode.data as CollapsedSubflowN['data']).innerNodeIds = nodes
								.slice(nodesBefore)
								.map((n) => n.id)

							previousId = lastInnerNodeId ?? headerNode.id
						} else if (module.value.type === 'flow' && 'path' in module.value) {
							nodes.push({
								id: module.id,
								data: {
									id: module.id,
									module: module,
									eventHandlers: eventHandlers,
									editMode: extra.editMode,
									expanded: false
								},
								type: 'collapsedSubflow'
							} as NodeLayout)
							previousId = module.id
						} else {
							addNode(module)
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
			return previousId
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
				addNode({ ...failureModule, id: x[1] })
				addEdge(x[0], x[1], undefined, undefined, { type: 'empty' })
			})
		}

		if (preprocessorModule) {
			addNode(preprocessorModule)
			const id = JSON.parse(JSON.stringify(preprocessorModule.id))
			addEdge(id, 'Input', undefined, undefined, { type: 'empty' })
		}

		if (failureModule && !extra.flowModuleStates) {
			addNode(failureModule)
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
