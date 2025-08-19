<script module lang="ts">
	export function validateToolName(name: string) {
		return /^[a-zA-Z0-9_]+$/.test(name)
	}

	export const AI_TOOL_BASE_OFFSET = 5
	export const AI_TOOL_ROW_OFFSET = 30
	export const BELOW_ADDITIONAL_OFFSET = 19

	export const AI_TOOL_CALL_PREFIX = '_wm_ai_agent_tool_call_'
	export const AI_TOOL_MESSAGE_PREFIX = '_wm_ai_agent_message_'

	const ROW_WIDTH = 275
	const NEW_TOOL_NODE_WIDTH = 40
	const MAX_TOOLS_PER_ROW = 2

	let computeAIToolNodesCache:
		| [
				(Node & NodeLayout)[],
				Record<string, GraphModuleState> | undefined,
				ReturnType<typeof computeAIToolNodes>
		  ]
		| undefined

	export function getToolCallId(idx: number, moduleId?: string) {
		return moduleId ? AI_TOOL_CALL_PREFIX + idx + '_' + moduleId : AI_TOOL_MESSAGE_PREFIX + idx
	}

	export function computeAIToolNodes(
		[nodes, edges]: [(Node & NodeLayout)[], Edge[]],
		eventHandlers: GraphEventHandlers,
		insertable: boolean,
		flowModuleStates: Record<string, GraphModuleState> | undefined
	): [(Node & NodeLayout)[], Edge[]] {
		if (nodes === computeAIToolNodesCache?.[0] && flowModuleStates === computeAIToolNodesCache?.[1])
			return computeAIToolNodesCache[2]

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
				stateType?: GraphModuleState['type']
			}[] = node.data.module.value.tools.map((t) => ({
				id: t.id,
				name: t.summary ?? ''
			}))

			const agentActions = !insertable && flowModuleStates?.[node.id]?.agent_actions
			if (agentActions) {
				// should show tools below
				baseOffset = BELOW_ADDITIONAL_OFFSET + AI_TOOL_BASE_OFFSET
				rowOffset = AI_TOOL_ROW_OFFSET
				tools = agentActions.map((a, idx) => {
					if (a.type === 'tool_call') {
						const id = getToolCallId(idx, a.module_id)
						return {
							id,
							name: a.function_name
						}
					} else {
						return {
							id: getToolCallId(idx),
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
						eventHandlers,
						moduleId: tool.id,
						insertable,
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
					}
				}
			})

			const toolEdges: Edge[] = toolNodes?.map((n) => ({
				id: `${n.id}-edge`,
				source: agentActions ? (n.parentId ?? '') : (n.id ?? ''),
				target: agentActions ? (n.id ?? '') : (n.parentId ?? ''),
				type: 'empty',
				data: { class: '!opacity-35 dark:!opacity-20' }
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
						x: (ROW_WIDTH - NEW_TOOL_NODE_WIDTH) / 2,
						y: baseOffset + rowOffset
					}
				} satisfies Node & NewAiToolN)
			}
		}

		const existingAssetNodes = nodes.filter((n) => n.type === 'asset')
		const sortedNewNodes = clone(nodes)
			.filter((n) => n.type !== 'asset')
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

		let ret: ReturnType<typeof computeAIToolNodes> = [
			[...sortedNewNodes, ...existingAssetNodes, ...allToolNodes],
			[...edges, ...allToolEdges]
		]
		computeAIToolNodesCache = [nodes, flowModuleStates, ret]
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
	import { MessageCircle, Play, Wrench, X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { getContext } from 'svelte'
	import { clone } from '$lib/utils'
	import type { Edge, Node } from '@xyflow/svelte'

	import type { Writable } from 'svelte/store'
	import type { GraphModuleState } from '../../model'
	import { getStateColor, getStateHoverColor } from '../../util'

	let hover = $state(false)

	interface Props {
		data: AiToolN['data']
	}

	let { data }: Props = $props()

	const { selectedId } = getContext<{
		selectedId: Writable<string | undefined>
	}>('FlowGraphContext')

	const flowModuleState = $derived(data.flowModuleStates?.[data.moduleId])
</script>

<NodeWrapper>
	{#snippet children({ darkMode })}
		{@const bgColor = getStateColor(flowModuleState?.type, darkMode, true, false)}
		{@const bgHoverColor = getStateHoverColor(flowModuleState?.type, darkMode, true, false)}

		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="relative" onmouseenter={() => (hover = true)} onmouseleave={() => (hover = false)}>
			<button
				class={twMerge(
					'text-left bg-surface h-6 flex items-center gap-1.5 rounded-sm text-secondary overflow-clip w-full outline-offset-0 outline-slate-500 dark:outline-gray-400',
					$selectedId === data.moduleId ? 'outline outline-1' : 'active:outline active:outline-1'
				)}
				style={`background-color: ${hover ? bgHoverColor : bgColor};`}
				onclick={() => data.eventHandlers.select(data.moduleId)}
			>
				{#if data.moduleId.startsWith(AI_TOOL_MESSAGE_PREFIX)}
					<MessageCircle size={16} class="ml-1 shrink-0" />
				{:else if data.moduleId.startsWith(AI_TOOL_CALL_PREFIX)}
					<Play size={16} class="ml-1 shrink-0" />
				{:else}
					<Wrench size={16} class="ml-1 shrink-0" />
				{/if}

				<span
					class={twMerge(
						'text-3xs truncate flex-1',
						!validateToolName(data.tool) && 'text-red-400'
					)}
				>
					{data.tool || 'No tool name'}
				</span>
			</button>
			{#if data.insertable}
				<button
					class={twMerge(
						'absolute -top-[8px] -right-[8px] rounded-full h-[16px] w-[16px] center-center text-secondary outline-[1px] outline dark:outline-gray-500 outline-gray-300 bg-surface duration-0 hover:bg-red-400 hover:text-white',
						'group-active:!flex group-hover:!flex !hidden',
						$selectedId === data.moduleId ? '!flex' : ''
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
