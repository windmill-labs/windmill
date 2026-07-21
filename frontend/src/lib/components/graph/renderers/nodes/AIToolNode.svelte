<script module lang="ts">
	import { forbiddenIds } from '$lib/components/flows/idUtils'
	import type { AgentTool } from '$lib/components/flows/agentToolUtils'

	export function getToolNameError(
		name: string,
		type?: string,
		siblingNames?: string[]
	): string | undefined {
		if (type === 'websearch') return undefined
		if (type === 'mcp') {
			return name.length > 0 ? undefined : 'Tool name must not be empty'
		}
		if (!/^[a-zA-Z0-9_]+$/.test(name)) {
			return 'Tool name must only contain letters, numbers and underscores'
		}
		if (forbiddenIds.includes(name)) {
			return `'${name}' is a reserved name`
		}
		if (siblingNames && siblingNames.filter((n) => n === name).length > 1) {
			return 'Duplicate tool name'
		}
		return undefined
	}

	export function validateToolName(name: string, type?: string) {
		return getToolNameError(name, type) === undefined
	}

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
				hasFlowModuleStates: boolean
				linkedAgentTools: Record<string, AgentTool[]> | undefined
				ret: ReturnType<typeof computeAIToolNodes>
		  }
		| undefined

	export function getToolCallId(idx: number, agentModuleId: string, moduleId?: string) {
		return moduleId
			? AI_TOOL_CALL_PREFIX + '-' + agentModuleId + '-' + idx + '-' + moduleId
			: AI_TOOL_MESSAGE_PREFIX + '-' + agentModuleId + '-' + idx
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
			!!flowModuleStates === computeAIToolNodesCache.hasFlowModuleStates &&
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
			const isLinkedAgent = !!(node.data.module.value as { agent?: string }).agent
			const sourceTools = isLinkedAgent
				? (linkedAgentTools?.[node.data.module.id] ?? [])
				: node.data.module.value.tools
			let tools: {
				id: string
				name: string
				type?: string
				stateType?: GraphModuleState['type']
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
					type: toolType
				}
			})

			const agentActions = !insertable && flowModuleStates?.[node.id]?.agent_actions
			if (agentActions) {
				// should show tools below
				baseOffset = BELOW_ADDITIONAL_OFFSET + AI_TOOL_BASE_OFFSET
				rowOffset = AI_TOOL_ROW_OFFSET
				tools = agentActions.map((a, idx) => {
					if (a.type === 'tool_call' || a.type === 'mcp_tool_call') {
						const id =
							a.type === 'tool_call'
								? getToolCallId(idx, node.id, a.module_id)
								: AI_MCP_TOOL_CALL_PREFIX + '-' + node.id + '-' + idx
						return {
							id,
							name: a.function_name
						}
					} else if (a.type === 'web_search') {
						return {
							id: AI_WEBSEARCH_PREFIX + '-' + node.id + '-' + idx,
							name: 'Web Search',
							type: 'websearch'
						}
					} else {
						return {
							id: getToolCallId(idx, node.id),
							name: 'Message'
						}
					}
				})
			}

			// A linked agent shows no "add tool" node, so its rows must not reserve one; otherwise the
			// tools float up by a row, leaving a gap above the agent where the add node would have been.
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
						insertable,
						readOnly: isLinkedAgent,
						flowModuleStates
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
			hasFlowModuleStates: !!flowModuleStates,
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

	let hover = $state(false)

	interface Props {
		data: AiToolN['data']
		id?: string
	}

	let { data, id }: Props = $props()

	const { selectionManager } = getGraphContext()

	const flowModuleState = $derived(data.flowModuleStates?.[data.moduleId])
	let colorClasses = $derived(
		getNodeColorClasses(
			data.nameError ? 'Failure' : flowModuleState?.type,
			selectionManager?.getSelectedId() === data.moduleId
		)
	)
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
				onclick={() => data.eventHandlers.select(data.moduleId)}
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
