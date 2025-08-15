<script module lang="ts">
	export const NODE_WITH_AI_TOOL_BASE_OFFSET = 5
	export const NODE_WITH_AI_TOOL_ROW_OFFSET = 30
	// export const AI_TOOL_BEFORE_Y_OFFSET = -45

	export const AI_TOOL_BASE_OFFSET = -5
	export const AI_TOOL_ROW_OFFSET = -30

	// export const AI_TOOL_AFTER_Y_OFFSET = 64

	let computeAIToolNodesCache:
		| [(Node & NodeLayout)[], ReturnType<typeof computeAIToolNodes>]
		| undefined

	export function computeAIToolNodes(
		[nodes, edges]: [(Node & NodeLayout)[], Edge[]],
		eventHandlers: GraphEventHandlers
	): [(Node & NodeLayout)[], Edge[]] {
		if (nodes === computeAIToolNodesCache?.[0]) return computeAIToolNodesCache[1]

		const ROW_WIDTH = 275
		const NEW_TOOL_NODE_WIDTH = 40
		const MAX_TOOLS_PER_ROW = 2
		const allToolNodes: (Node & NodeLayout)[] = []
		const allToolEdges: Edge[] = []

		const yPosMap: Record<number, number> = {}

		for (const node of nodes) {
			if (node.type !== 'module' || node.data.module.value.type !== 'aiagent') continue

			const tools: {
				id: string
				name: string
			}[] = node.data.module.value.tools.map((t) => ({
				id: t.id,
				name: t.summary ?? ''
			}))

			// const tools = node.data.tools ?? []

			// Each tool can be displayed at the top and bottom
			const inputTools = tools

			// This allows calculating which nodes to offset on the y axis to
			// make space for the tool nodes

			const totalRows = Math.ceil(inputTools.length / MAX_TOOLS_PER_ROW) + 1
			yPosMap[node.position.y] = totalRows

			// All tool nodes displayed on top
			const inputToolNodes: (Node & AiToolN)[] = inputTools.map((tool, i) => {
				let inputToolXGap = 12
				let inputToolWidth = (ROW_WIDTH - inputToolXGap) / 2

				const row = Math.floor(i / MAX_TOOLS_PER_ROW) + 1
				return {
					type: 'aiTool' as const,
					parentId: node.id,
					data: { tool: tool.name, eventHandlers, moduleId: tool.id },
					id: `${node.id}-tool-${tool.id}`,
					width: inputToolWidth,
					position: {
						x:
							inputTools.length === 1
								? (ROW_WIDTH - inputToolWidth) / 2
								: (i + 1) % 2 === 0
									? inputToolWidth + inputToolXGap
									: row === totalRows - 1 && inputTools.length % 2 === 1
										? (ROW_WIDTH - inputToolWidth) / 2
										: 0,
						y:
							AI_TOOL_BASE_OFFSET +
							AI_TOOL_ROW_OFFSET * (totalRows - Math.floor(i / MAX_TOOLS_PER_ROW))
					}
				}
			})

			const inputToolEdges: Edge[] = inputToolNodes?.map((n) => ({
				id: `${n.id}-edge`,
				source: n.id ?? '',
				target: n.parentId ?? '',
				type: 'empty',
				data: { class: '!opacity-35 dark:!opacity-20' }
			}))

			allToolEdges.push(...(inputToolEdges ?? []))
			allToolNodes.push(...(inputToolNodes ?? []))

			// If there are more than 3 tools, we create an overflow node
			// if (overflowedInputTools.length)
			allToolNodes.push({
				type: 'newAiTool',
				data: { eventHandlers, agentModuleId: node.data.module.id },
				id: `${node.id}-tools-overflowed-in`,
				parentId: node.id,
				width: NEW_TOOL_NODE_WIDTH,
				position: {
					x: (ROW_WIDTH - NEW_TOOL_NODE_WIDTH) / 2,
					y: AI_TOOL_BASE_OFFSET + AI_TOOL_ROW_OFFSET
				}
			} satisfies Node & NewAiToolN)
		}

		const existingAssetNodes = nodes.filter((n) => n.type === 'asset')
		const sortedNewNodes = clone(nodes)
			.filter((n) => n.type !== 'asset')
			.sort((a, b) => a.position.y - b.position.y)
		let currentYOffset = 0
		let prevYPos = NaN
		for (const node of sortedNewNodes) {
			if (node.position.y !== prevYPos) {
				if (yPosMap[node.position.y])
					currentYOffset +=
						NODE_WITH_AI_TOOL_BASE_OFFSET + NODE_WITH_AI_TOOL_ROW_OFFSET * yPosMap[node.position.y]

				prevYPos = node.position.y
			}

			node.position.y += currentYOffset
		}

		let ret: ReturnType<typeof computeAIToolNodes> = [
			[...sortedNewNodes, ...existingAssetNodes, ...allToolNodes],
			[...edges, ...allToolEdges]
		]
		computeAIToolNodesCache = [nodes, ret]
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
	import { Wrench, X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { getContext } from 'svelte'
	import { clone } from '$lib/utils'
	import type { Edge, Node } from '@xyflow/svelte'

	import type { Writable } from 'svelte/store'
	import { validateToolName } from '$lib/components/flows/content/FlowAIAgent.svelte'

	interface Props {
		data: AiToolN['data']
	}

	let { data }: Props = $props()

	const { selectedId } = getContext<{
		selectedId: Writable<string | undefined>
	}>('FlowGraphContext')
</script>

<NodeWrapper>
	{#snippet children({ darkMode })}
		<div class="relative group">
			<button
				class={twMerge(
					'hover:bg-surface-hover text-left bg-surface h-6 flex items-center gap-1.5 rounded-sm text-tertiary border overflow-clip w-full',
					$selectedId === data.moduleId
						? 'bg-surface-secondary !border-surface-inverse'
						: 'border-transparent'
				)}
				onclick={() => data.eventHandlers.select(data.moduleId)}
			>
				<Wrench size={16} class="ml-1 shrink-0" />

				<span
					class={twMerge(
						'text-3xs truncate flex-1',
						!validateToolName(data.tool) && 'text-red-400'
					)}
				>
					{data.tool || 'No tool name'}
				</span>
			</button>
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
		</div>
	{/snippet}
</NodeWrapper>
