<script module lang="ts">
	import { getToolNameError, type AgentTool } from '$lib/components/flows/agentToolUtils'

	export const AI_TOOL_BASE_OFFSET = 5
	export const AI_TOOL_ROW_OFFSET = 30
	export const BELOW_ADDITIONAL_OFFSET = 19

	export const AI_TOOL_CALL_PREFIX = '_wm_ai_agent_tool_call'
	export const AI_MCP_TOOL_CALL_PREFIX = '_wm_ai_mcp_tool_call'
	export const AI_TOOL_MESSAGE_PREFIX = '_wm_ai_agent_message'
	export const AI_WEBSEARCH_PREFIX = '_wm_ai_websearch'

	const ROW_WIDTH = 275
	const NEW_TOOL_NODE_WIDTH = 50
	const MAX_TOOLS_PER_ROW = 2

	let computeAIToolNodesCache:
		| {
				nodes: (Node & NodeLayout)[]
				agentActions: Record<string, unknown>
				linkedAgentTools: Record<string, AgentTool[]> | undefined
				ret: ReturnType<typeof computeAIToolNodes>
		  }
		| undefined

	export function getToolCallId(idx: number, agentModuleId: string, moduleId?: string) {
		return moduleId
			? AI_TOOL_CALL_PREFIX + '-' + agentModuleId + '-' + idx + '-' + moduleId
			: AI_TOOL_MESSAGE_PREFIX + '-' + agentModuleId + '-' + idx
	}

	export type AgentAction = NonNullable<GraphModuleState['agent_actions']>[number]

	function bareResourcePath(path: string | undefined): string | undefined {
		return path?.startsWith('$res:') ? path.slice('$res:'.length) : path
	}

	/** Whether a run's action was a call of this declared tool. The editor keeps one node per
	 * declared tool, so each kind of action has to find its way back to the right one: a flow
	 * module by id, web search by being the only one of its kind, an MCP server by its resource
	 * path. A miss leaves the node undecorated rather than decorating the wrong tool. */
	export function agentActionMatchesTool(
		action: AgentAction,
		tool: { moduleId: string; type?: string; resourcePath?: string }
	): boolean {
		switch (action.type) {
			case 'tool_call':
				return action.module_id === tool.moduleId
			case 'web_search':
				return tool.type === 'websearch'
			case 'mcp_tool_call':
				// One MCP node stands for a whole server and many function names, so the server path
				// is the only join that holds. The worker strips `$res:` before building the action,
				// while a flow authored outside the resource picker can still carry it.
				return (
					tool.type === 'mcp' &&
					bareResourcePath(action.resource_path) === bareResourcePath(tool.resourcePath)
				)
			case 'message':
				return false
		}
	}

	/** The one id an agent action's state is written and read under. Every writer and reader has
	 * to rebuild the same key, so they all come here; a switch with no default makes a new action
	 * kind a compile error rather than a status that silently never resolves. */
	export function getAgentActionStateId(
		idx: number,
		agentModuleId: string,
		action: AgentAction
	): string {
		switch (action.type) {
			case 'tool_call':
				return getToolCallId(idx, agentModuleId, action.module_id)
			case 'mcp_tool_call':
				return AI_MCP_TOOL_CALL_PREFIX + '-' + agentModuleId + '-' + idx
			case 'web_search':
				return AI_WEBSEARCH_PREFIX + '-' + agentModuleId + '-' + idx
			case 'message':
				return getToolCallId(idx, agentModuleId)
		}
	}

	function getComparableNode(node: Node & NodeLayout): Node & NodeLayout {
		if (node.type === 'module' && node.data.module.value.type === 'aiagent') {
			return {
				...node,
				data: {
					...node.data,
					module: $state.snapshot(node.data.module) // module is a proxy object so we need to snapshot to be able to compare
				}
			}
		} else {
			return node
		}
	}

	function agentActionsOf(
		nodes: (Node & NodeLayout)[],
		flowModuleStates: Record<string, GraphModuleState> | undefined,
		insertable: boolean
	): Record<string, unknown> {
		const actions: Record<string, unknown> = {}
		// The editor renders the static tool set and ignores the run's actions, so snapshotting
		// them there would deep-clone a value that changes every poll and never matches.
		if (insertable) return actions
		for (const node of nodes) {
			if (node.type !== 'module' || node.data.module.value.type !== 'aiagent') continue
			actions[node.id] = $state.snapshot(flowModuleStates?.[node.id]?.agent_actions)
		}
		return actions
	}

	export function computeAIToolNodes(
		nodes: (Node & NodeLayout)[],
		eventHandlers: GraphEventHandlers,
		insertable: boolean,
		flowModuleStates: Record<string, GraphModuleState> | undefined,
		// Tools resolved from linked agents' resources, keyed by agent module id. Linked steps carry
		// no tools of their own, so their tool nodes come from here.
		linkedAgentTools?: Record<string, AgentTool[]>
	): {
		toolNodes: (Node & NodeLayout)[]
		toolEdges: Edge[]
	} {
		if (
			computeAIToolNodesCache &&
			deepEqual(
				agentActionsOf(nodes, flowModuleStates, insertable),
				computeAIToolNodesCache.agentActions
			) &&
			deepEqual(nodes.map(getComparableNode), computeAIToolNodesCache.nodes) &&
			deepEqual(linkedAgentTools, computeAIToolNodesCache.linkedAgentTools)
		) {
			return computeAIToolNodesCache.ret
		}

		const allToolNodes: (Node & NodeLayout)[] = []
		const allToolEdges: Edge[] = []

		for (const node of nodes) {
			if (node.type !== 'module' || node.data.module.value.type !== 'aiagent') continue
			// by default we assume we will show tools above
			let baseOffset = -AI_TOOL_BASE_OFFSET
			let rowOffset = -AI_TOOL_ROW_OFFSET
			// A linked step's tools come from its resource (resolved into linkedAgentTools), not the
			// module, whose own `tools` is empty.
			const isLinkedAgent = !!node.data.module.value.agent
			const sourceTools = isLinkedAgent
				? (linkedAgentTools?.[node.data.module.id] ?? [])
				: (node.data.module.value.tools ?? [])
			let tools: {
				id: string
				name: string
				type?: string
				stateType?: GraphModuleState['type']
				resourcePath?: string
			}[] = sourceTools.map((t, idx) => {
				// Handle FlowModule, MCP, and Websearch tools
				const toolType =
					t.value.tool_type === 'mcp'
						? 'mcp'
						: t.value.tool_type === 'websearch'
							? 'websearch'
							: t.value.tool_type === 'flowmodule'
								? t.value.type
								: undefined
				return {
					id: t.id,
					name: t.summary ?? '',
					type: toolType,
					resourcePath: t.value.tool_type === 'mcp' ? t.value.resource_path : undefined
				}
			})

			const agentActions = !insertable && flowModuleStates?.[node.id]?.agent_actions
			if (agentActions) {
				// should show tools below
				baseOffset = BELOW_ADDITIONAL_OFFSET + AI_TOOL_BASE_OFFSET
				rowOffset = AI_TOOL_ROW_OFFSET
				tools = agentActions.map((a, idx) => {
					const id = getAgentActionStateId(idx, node.id, a)
					if (a.type === 'tool_call' || a.type === 'mcp_tool_call') {
						return { id, name: a.function_name }
					} else if (a.type === 'web_search') {
						return { id, name: 'Web Search', type: 'websearch' }
					} else {
						return { id, name: 'Message' }
					}
				})
			}

			// A linked agent shows no "add tool" node, so its rows must not reserve one; otherwise the
			// tools float up by a row, leaving a gap above the agent where the add node would have been.
			// When its tools aren't resolved (e.g. viewers that don't fetch the resource), it simply
			// shows none — the node label already carries the linked resource path.
			const showAddToolNode = insertable && !isLinkedAgent
			const totalRows = Math.ceil(tools.length / MAX_TOOLS_PER_ROW) + (showAddToolNode ? 1 : 0)

			const siblingNames = tools.map((t) => t.name)
			const toolNodes: (Node & AiToolN)[] = tools.map((tool, i) => {
				let inputToolXGap = 12
				let inputToolWidth = (ROW_WIDTH - inputToolXGap) / 2

				const row = Math.floor(i / MAX_TOOLS_PER_ROW) + 1

				const isLastRow = showAddToolNode ? row === totalRows - 1 : row === totalRows
				return {
					type: 'aiTool' as const,
					parentId: node.id,
					data: {
						tool: tool.name,
						type: tool.type,
						// agentActions are runtime tool calls: the same tool called multiple times
						// yields duplicate names, which is expected and must not read as a Failure.
						// Only validate names in the editor, where they define the static tool set.
						nameError: agentActions
							? undefined
							: getToolNameError(tool.name, tool.type, siblingNames),
						eventHandlers,
						moduleId: tool.id,
						// A linked agent's tools are display-only: their resource-owned ids are not
						// flow-unique, so they must not drive selection. Clicking one selects the agent
						// step instead. Kept separate from moduleId — aliasing the module id here would
						// misroute agent-node clicks into the graph's manual aiTool selection path.
						selectTarget: isLinkedAgent && !agentActions ? node.id : undefined,
						insertable,
						readOnly: isLinkedAgent,
						agentModuleId: node.id,
						resourcePath: tool.resourcePath
					},
					id: `${node.id}-tool-${tool.id}`,
					width: inputToolWidth,
					position: {
						x:
							tools.length === 1
								? (ROW_WIDTH - inputToolWidth) / 2
								: (i + 1) % 2 === 0
									? inputToolWidth + inputToolXGap
									: isLastRow && tools.length % 2 === 1
										? (ROW_WIDTH - inputToolWidth) / 2
										: 0,
						y:
							baseOffset +
							rowOffset *
								(agentActions
									? Math.floor(i / MAX_TOOLS_PER_ROW) + 1
									: totalRows - Math.floor(i / MAX_TOOLS_PER_ROW))
					},
					selectable: false
				}
			})

			const toolEdges: Edge[] = toolNodes?.map((n) => ({
				id: `${n.id}-edge`,
				source: agentActions ? (n.parentId ?? '') : (n.id ?? ''),
				target: agentActions ? (n.id ?? '') : (n.parentId ?? ''),
				type: 'empty',
				data: { class: '!opacity-35 dark:!opacity-20' },
				selectable: false
			}))

			allToolEdges.push(...(toolEdges ?? []))
			allToolNodes.push(...(toolNodes ?? []))

			// A linked agent is rigid: its tools come from the resource and can't be edited here, so
			// don't offer the "add tool" node (unlink/fork the step to change tools).
			if (showAddToolNode) {
				allToolNodes.push({
					type: 'newAiTool',
					data: { eventHandlers, agentModuleId: node.data.module.id },
					id: `${node.id}-tools-overflowed-in`,
					parentId: node.id,
					width: NEW_TOOL_NODE_WIDTH,
					position: {
						x: (ROW_WIDTH - NEW_TOOL_NODE_WIDTH) / 2,
						y: baseOffset + rowOffset
					},
					selectable: false
				} satisfies Node & NewAiToolN)
			}
		}

		let ret: ReturnType<typeof computeAIToolNodes> = {
			toolNodes: allToolNodes,
			toolEdges: allToolEdges
		}

		computeAIToolNodesCache = {
			nodes: nodes.map(getComparableNode),
			agentActions: agentActionsOf(nodes, flowModuleStates, insertable),
			linkedAgentTools: $state.snapshot(linkedAgentTools),
			ret
		}
		return ret
	}
</script>

<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import type {
		AiToolN,
		GraphEventHandlers,
		NewAiToolN,
		NodeLayout
	} from '../../graphBuilder.svelte'
	import { Bot, Globe, MessageCircle, Play, Plug, Wrench, X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { Edge, Node } from '@xyflow/svelte'

	import type { GraphModuleState } from '../../model'
	import { getNodeColorClasses } from '../../util'
	import { deepEqual } from 'fast-equals'
	import { getGraphContext } from '../../graphContext'
	import { getFlowRunStatusContext } from '../../flowRunStatus.svelte'

	let hover = $state(false)

	interface Props {
		data: AiToolN['data']
		id?: string
	}

	let { data, id }: Props = $props()
	const flowRunStatus = getFlowRunStatusContext()

	const { selectionManager } = getGraphContext()

	const flowModuleState = $derived(flowRunStatus?.getModuleState(data.moduleId))

	/**
	 * The editor draws the agent's declared tools, one node per tool, while a run keys its state
	 * per call. Roll every call of this tool into the one node it already has, so a run shows up
	 * here without adding nodes and shifting the graph. A run graph looks its own state up
	 * directly, so it never gets here.
	 */
	const toolCalls = $derived.by(() => {
		if (flowModuleState) return undefined
		const agentState = flowRunStatus?.getModuleState(data.agentModuleId)
		const actions = agentState?.agent_actions
		if (!actions) return undefined
		let count = 0
		let failed = 0
		let pending = 0
		actions.forEach((action, index) => {
			if (
				!agentActionMatchesTool(action, {
					moduleId: data.moduleId,
					type: data.type,
					resourcePath: data.resourcePath
				})
			)
				return
			count++
			const success = agentState?.agent_actions_success?.[index]
			if (success === undefined) pending++
			else if (!success) failed++
		})
		if (count === 0) return undefined
		const type = pending > 0 ? 'InProgress' : failed > 0 ? 'Failure' : 'Success'
		return { type, count } as const
	})

	let colorClasses = $derived(
		getNodeColorClasses(
			data.nameError ? 'Failure' : (flowModuleState?.type ?? toolCalls?.type),
			selectionManager?.getSelectedId() === (data.selectTarget ?? data.moduleId)
		)
	)

	// Display-only tools (linked agents) select their agent step instead of themselves. The manager
	// is set here rather than via the graph's select handler: this click never creates a svelte-flow
	// node selection (the tool isn't selectable), so a manual select is safe, whereas routing the
	// agent's module id through the manual path would race the agent node's own click selection.
	function onSelect() {
		if (data.selectTarget) {
			selectionManager?.selectId(data.selectTarget, { openPanel: true })
		}
		data.eventHandlers.select(data.selectTarget ?? data.moduleId, { openPanel: true })
	}
</script>

<NodeWrapper nodeId={id}>
	{#snippet children({ darkMode })}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="relative bg-surface-secondary rounded-md"
			onmouseenter={() => (hover = true)}
			onmouseleave={() => (hover = false)}
		>
			<button
				class={twMerge(
					'text-left h-6 flex items-center gap-1.5 rounded-md overflow-clip w-full outline-offset-0 drop-shadow-base',
					colorClasses.outline,
					colorClasses.text,
					colorClasses.bg
				)}
				onclick={onSelect}
			>
				{#if data.moduleId.startsWith(AI_TOOL_MESSAGE_PREFIX)}
					<MessageCircle size={16} class="ml-1 shrink-0" />
				{:else if data.moduleId.startsWith(AI_TOOL_CALL_PREFIX) || data.moduleId.startsWith(AI_MCP_TOOL_CALL_PREFIX)}
					<Play size={16} class="ml-1 shrink-0" />
				{:else if data.type === 'websearch'}
					<Globe size={16} class="ml-1 shrink-0" />
				{:else if data.type === 'mcp'}
					<Plug size={16} class="ml-1 shrink-0" />
				{:else if data.type === 'aiagent'}
					<Bot size={16} class="ml-1 shrink-0" />
				{:else}
					<Wrench size={16} class="ml-1 shrink-0" />
				{/if}

				<span class={twMerge('text-3xs truncate flex-1', data.nameError && 'text-red-400')}>
					{data.tool || 'Missing name'}
				</span>

				<!-- Sits inside the button's fixed width, so the label truncates instead of the node
				     growing and pushing the graph around. -->
				{#if toolCalls && toolCalls.count > 1}
					<span
						class="text-3xs tabular-nums shrink-0 mr-1 px-1 rounded bg-surface/70 text-secondary"
						title={`Called ${toolCalls.count} times in this run`}
					>
						{toolCalls.count}
					</span>
				{/if}
			</button>
			{#if data.insertable && !data.readOnly}
				<button
					class={twMerge(
						'absolute -top-[8px] -right-[8px] rounded-full h-[16px] w-[16px] center-center text-secondary outline-[1px] outline dark:outline-gray-500 outline-gray-300 bg-surface duration-0 hover:bg-red-400 hover:text-white !hidden',
						selectionManager?.getSelectedId() === data.moduleId || hover ? '!flex' : ''
					)}
					title="Delete"
					onclick={() => data.eventHandlers.delete({ id: data.moduleId }, '')}
				>
					<X size={12} strokeWidth={2} />
				</button>
			{/if}
		</div>
	{/snippet}
</NodeWrapper>
