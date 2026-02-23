<script module lang="ts">
	export function validateToolName(name: string, type?: string) {
		if (type === 'websearch') return true
		if (type === 'mcp') {
			return name.length > 0
		}
		return /^[a-zA-Z0-9_]+$/.test(name)
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
		flowModuleStates: Record<string, GraphModuleState> | undefined
	): {
		toolNodes: (Node & NodeLayout)[]
		toolEdges: Edge[]
		newNodePositions: Record<string, { x: number; y: number }>
	} {
		if (
			computeAIToolNodesCache &&
			!!flowModuleStates === computeAIToolNodesCache.hasFlowModuleStates &&
			deepEqual(nodes.map(getComparableNode), computeAIToolNodesCache.nodes)
		) {
			return computeAIToolNodesCache.ret
		}

		const allToolNodes: (Node & NodeLayout)[] = []
		const allToolEdges: Edge[] = []

		const yPosMap: Record<
			number,
			{
				rows: number
				placement: 'above' | 'below'
			}
		> = {}

		for (const node of nodes) {
			if (node.type !== 'module' || node.data.module.value.type !== 'aiagent') continue
			// by default we assume we will show tools above
			let baseOffset = -AI_TOOL_BASE_OFFSET
			let rowOffset = -AI_TOOL_ROW_OFFSET
			let tools: {
				id: string
				name: string
				type?: string
				stateType?: GraphModuleState['type']
			}[] = node.data.module.value.tools.map((t, idx) => {
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

			const totalRows = Math.ceil(tools.length / MAX_TOOLS_PER_ROW) + (insertable ? 1 : 0) // + 1 for add tool node when insertable
			if (agentActions) {
				yPosMap[node.position.y] = {
					rows: totalRows,
					placement: 'below'
				}
			} else {
				yPosMap[node.position.y] = {
					rows: totalRows,
					placement: 'above'
				}
			}

			const toolNodes: (Node & AiToolN)[] = tools.map((tool, i) => {
				let inputToolXGap = 12
				let inputToolWidth = (ROW_WIDTH - inputToolXGap) / 2

				const row = Math.floor(i / MAX_TOOLS_PER_ROW) + 1

				const isLastRow = insertable ? row === totalRows - 1 : row === totalRows
				return {
					type: 'aiTool' as const,
					parentId: node.id,
					data: {
						tool: tool.name,
						type: tool.type,
						eventHandlers,
						moduleId: tool.id,
						insertable,
						flowModuleStates
					},
					id: `${node.id}-tool-${tool.id}`,
					width: inputToolWidth,
					position: {
						x:
							(tools.length === 1
								? (ROW_WIDTH - inputToolWidth) / 2
								: (i + 1) % 2 === 0
									? inputToolWidth + inputToolXGap
									: isLastRow && tools.length % 2 === 1
										? (ROW_WIDTH - inputToolWidth) / 2
										: 0) + node.data.offset,
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

			if (insertable) {
				allToolNodes.push({
					type: 'newAiTool',
					data: { eventHandlers, agentModuleId: node.data.module.id },
					id: `${node.id}-tools-overflowed-in`,
					parentId: node.id,
					width: NEW_TOOL_NODE_WIDTH,
					position: {
						x: (ROW_WIDTH - NEW_TOOL_NODE_WIDTH) / 2 + node.data.offset,
						y: baseOffset + rowOffset
					},
					selectable: false
				} satisfies Node & NewAiToolN)
			}
		}

		const sortedNewNodes = nodes
			.filter((n) => n.type !== 'asset')
			.map((n) => ({ id: n.id, position: $state.snapshot(n.position) }))
			.sort((a, b) => a.position.y - b.position.y)
		let currentYOffset = 0
		let prevYPos = NaN
		for (const node of sortedNewNodes) {
			if (node.position.y !== prevYPos) {
				// if agent actions, we need to shift the node above
				if (yPosMap[prevYPos]?.placement === 'below') {
					currentYOffset += AI_TOOL_BASE_OFFSET + AI_TOOL_ROW_OFFSET * yPosMap[prevYPos].rows
				}
				if (yPosMap[node.position.y]?.placement === 'above') {
					currentYOffset += AI_TOOL_BASE_OFFSET + AI_TOOL_ROW_OFFSET * yPosMap[node.position.y].rows
				}

				prevYPos = node.position.y
			}

			node.position.y += currentYOffset
		}

		let ret: ReturnType<typeof computeAIToolNodes> = {
			toolNodes: allToolNodes,
			toolEdges: allToolEdges,
			newNodePositions: Object.fromEntries(
				sortedNewNodes.map((n) => {
					return [n.id, n.position]
				})
			)
		}

		computeAIToolNodesCache = {
			nodes: nodes.map(getComparableNode),
			hasFlowModuleStates: !!flowModuleStates,
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
	}

	let { data }: Props = $props()

	const { selectionManager } = getGraphContext()

	const flowModuleState = $derived(data.flowModuleStates?.[data.moduleId])
	let colorClasses = $derived(
		getNodeColorClasses(
			!validateToolName(data.tool, data.type) ? 'Failure' : flowModuleState?.type,
			selectionManager?.getSelectedId() === data.moduleId
		)
	)
</script>

<NodeWrapper>
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

				<span
					class={twMerge(
						'text-3xs truncate flex-1',
						!validateToolName(data.tool, data.type) && 'text-red-400'
					)}
				>
					{data.tool || 'Missing name'}
				</span>
			</button>
			{#if data.insertable}
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
