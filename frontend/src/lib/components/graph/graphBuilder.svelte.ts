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
import { type GroupedModule } from './groupEditor.svelte'
import { findInsertIndex, isGroupItem } from './groupedModulesProxy.svelte'

/** Recursively collect FlowModules from nested groups (unwraps group wrappers only). */
function collectLeafModules(items: GroupedModule[]): FlowModule[] {
	return items.flatMap((m) => (isGroupItem(m) ? collectLeafModules(m.modules) : [m as FlowModule]))
}

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
	| GroupHeadN
	| GroupEndN

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
		eventHandlers: GraphEventHandlers
		label: string
		preLabel: string | undefined
		subflowId: string
		selected: boolean
		expanded: boolean
		module: FlowModule
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
		nameError?: string
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
		collapsed_by_default: boolean | undefined
		stepCount: number
		modules: FlowModule[]
		showNotes: boolean
		editMode: boolean
		eventHandlers: GraphEventHandlers
	}
}

export type GroupHeadN = {
	type: 'groupHead'
	data: {
		groupId: string
		summary: string | undefined
		note: string | undefined
		color: string | undefined
		collapsed_by_default: boolean | undefined
		editMode: boolean
		showNotes: boolean
		eventHandlers: GraphEventHandlers
		wrapperWidth?: number
	}
}

export type GroupEndN = {
	type: 'groupEnd'
	data: {
		groupId: string
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

function getContainerModules(module: FlowModule): FlowModule[] {
	return getAllModules([module]).filter((m) => m.id !== module.id)
}

export function graphBuilder(
	groupedModules: GroupedModule[] | undefined,
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
	collapsedContainers: Set<string>,
	showNotes: boolean,
	collapsedGroupIds: Set<string>
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

		function addNode(module: FlowModule, extraData?: Record<string, any>) {
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
				currentItems?: GroupedModule[]
				disableMoveIds?: string[]
			}
		) {
			parents[targetId] = [...(parents[targetId] ?? []), sourceId]

			let index: number
			if (options?.currentItems) {
				index = findInsertIndex(options.currentItems, targetId)
			} else {
				const mods = options?.subModules ?? modules
				const found = mods?.findIndex((m) => m.id === targetId) ?? -1
				index = found >= 0 ? found : (mods?.length ?? 0)
			}

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
					index,
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
			items: GroupedModule[],
			branch: { rootId: string; branch: number } | undefined,
			beforeNode: NodeLayout,
			nextNode: NodeLayout | undefined,
			simplifiedTriggerView: boolean,
			prefix: string | undefined,
			disableMoveIds: string[] = [],
			parentIndex?: string
		) {
			// For subflow prefix rewriting, collect the FlowModule items
			if (prefix != undefined) {
				items.forEach((item) => {
					if (isGroupItem(item)) return
					const m = item as FlowModule
					if (!m['oid']) {
						m['oid'] = m.id
					}
					m.id = 'subflow:' + prefix + m['oid']
				})
			}
			let previousId: string | undefined = undefined

			if (items.length === 0) {
				if (nextNode) {
					addEdge(beforeNode.id, nextNode.id, branch, prefix, {
						currentItems: items,
						disableMoveIds
					})
				}
			} else {
				items.forEach((item, index) => {
					// --- Group items ---
					if (isGroupItem(item)) {
						const g = item.group

						if (collapsedGroupIds.has(g.id)) {
							// Collapsed group: single node
							const nodeId = `collapsed-group:${g.id}`
							nodes.push({
								id: nodeId,
								data: {
									groupId: g.id,
									summary: g.summary,
									note: g.note,
									color: g.color,
									collapsed_by_default: g.collapsed_by_default,
									stepCount: item.moduleIds.length,
									modules: collectLeafModules(item.modules),
									showNotes,
									editMode: extra.editMode,
									eventHandlers
								},
								type: 'collapsedGroup',
								selectable: false
							})

							// Wire: previous → collapsedGroup
							if (index > 0 && previousId) {
								addEdge(previousId, nodeId, branch, prefix, {
									currentItems: items,
									disableMoveIds
								})
							}

							previousId = nodeId
						} else {
							// Expanded group: head → recurse → end
							const headId = `group:${g.id}`
							const endId = `group:${g.id}-end`

							const headNode: NodeLayout = {
								id: headId,
								data: {
									groupId: g.id,
									summary: g.summary,
									note: g.note,
									color: g.color,
									collapsed_by_default: g.collapsed_by_default,
									editMode: extra.editMode,
									showNotes,
									eventHandlers
								},
								type: 'groupHead',
								selectable: false
							}

							const endNode: NodeLayout = {
								id: endId,
								data: {
									groupId: g.id
								},
								type: 'groupEnd',
								selectable: false
							}

							nodes.push(headNode)
							nodes.push(endNode)

							// Wire: previous → headNode
							if (index > 0 && previousId) {
								addEdge(previousId, headId, branch, prefix, {
									currentItems: items,
									disableMoveIds
								})
							}

							// Recurse inner modules
							processModules(
								item.modules,
								{ rootId: headId, branch: 0 },
								headNode,
								endNode,
								simplifiedTriggerView,
								prefix,
								disableMoveIds,
								parentIndex
							)

							previousId = endId
						}

						// Shared first/last edge wiring for groups
						if (index === 0) {
							const entryId = collapsedGroupIds.has(g.id)
								? `collapsed-group:${g.id}`
								: `group:${g.id}`
							addEdge(beforeNode.id, entryId, undefined, prefix, {
								currentItems: items,
								disableMoveIds,
								disableInsert: simplifiedTriggerView
							})
						}

						if (index === items.length - 1 && previousId && nextNode) {
							addEdge(previousId, nextNode.id, branch, prefix, {
								currentItems: items,
								disableMoveIds
							})
						}

						return
					}

					// --- Regular FlowModule items ---
					const module = item as FlowModule
					const localDisableMoveIds = [...disableMoveIds, module.id]

					// Inter-module edge: connect previous → current (expanded subflows handle their own)
					if (index > 0 && previousId && expandedSubflows[module.id] == undefined) {
						addEdge(previousId, module.id, branch, prefix, {
							currentItems: items,
							disableMoveIds
						})
					}

					// Collapsed container check (before the type-based chain)
					const isContainer =
						module.value.type === 'branchall' ||
						module.value.type === 'branchone' ||
						module.value.type === 'forloopflow' ||
						module.value.type === 'whileloopflow'

					if (isContainer && collapsedContainers.has(module.id)) {
						addNode(module, {
							isCollapsedContainer: true,
							containerModules: getContainerModules(module)
						})
						previousId = module.id
					} else if (module.value.type === 'branchall') {
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
						let expandedMods = expandedSubflows[module.id]
						if (expandedMods) {
							expandedMods = $state.snapshot(expandedMods)
							const startId = `${module.id}`
							const idWithoutPrefix = module.id.startsWith('subflow:')
								? module.id.substring(8)
								: module.id
							const startNode: NodeLayout = {
								id: startId,
								data: {
									label: `Start of subflow ${idWithoutPrefix}`,
									id: startId,
									subflowId: module.id,
									eventHandlers: eventHandlers,
									preLabel: '',
									selected: false,
									expanded: true,
									module: module
								},
								type: 'collapsedSubflow'
							}

							nodes.push(startNode)

							if (previousId) {
								addEdge(previousId, startNode.id, branch, prefix, {
									currentItems: items,
									disableMoveIds
								})
							} else {
								addEdge(beforeNode.id, startNode.id, undefined, prefix, {
									currentItems: items,
									disableMoveIds
								})
							}

							const endId = `${module.id}-subflow-end`
							const endNode: NodeLayout = {
								id: endId,
								data: {
									label: `End of subflow ${idWithoutPrefix}`,
									id: endId,
									subflowId: module.id,
									eventHandlers: eventHandlers,
									preLabel: '',
									selected: false,
									expanded: true,
									module: module
								},
								type: 'collapsedSubflow'
							}

							nodes.push(endNode)

							processModules(
								expandedMods,
								undefined,
								startNode,
								endNode,
								false,
								buildPrefix(prefix, module['oid'] ?? module.id),
								localDisableMoveIds
							)

							previousId = endNode.id
						} else if (module.value.type === 'flow') {
							// Non-expanded flow module — show as collapsedSubflow
							nodes.push({
								id: module.id,
								data: {
									id: module.id,
									subflowId: module.id,
									eventHandlers: eventHandlers,
									label: module.summary || module.value['path'] || module.id,
									preLabel: '',
									selected: false,
									expanded: false,
									module: module
								},
								type: 'collapsedSubflow',
								selectable: true
							})
							previousId = module.id
						} else {
							addNode(module)
							previousId = module.id
						}
					}

					if (index === 0 && expandedSubflows[module.id] == undefined) {
						addEdge(beforeNode.id, module.id, undefined, prefix, {
							currentItems: items,
							disableMoveIds,
							disableInsert: simplifiedTriggerView
						})
					}

					if (index === items.length - 1 && previousId && nextNode) {
						addEdge(previousId, nextNode.id, branch, prefix, {
							currentItems: items,
							disableMoveIds
						})
					}
				})
			}
		}

		// Use groupedModules if available, otherwise wrap plain modules
		const topLevelItems: GroupedModule[] = groupedModules ?? modules

		if (simplifiableFlow?.simplifiedFlow === true && triggerNode) {
			processModules(topLevelItems, undefined, triggerNode, undefined, true, undefined)
		} else {
			processModules(topLevelItems, undefined, inputNode, resultNode, false, undefined)
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
