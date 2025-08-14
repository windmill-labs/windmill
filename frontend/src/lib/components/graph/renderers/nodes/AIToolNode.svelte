<script module lang="ts">
	export const NODE_WITH_READ_ASSET_Y_OFFSET = 45
	export const NODE_WITH_WRITE_ASSET_Y_OFFSET = 45
	export const READ_ASSET_Y_OFFSET = -45
	export const WRITE_ASSET_Y_OFFSET = 64
	export const assetDisplaysAsInputInFlowGraph = (a: AssetWithAltAccessType) =>
		!getAccessType(a) || getAccessType(a) === 'r' || getAccessType(a) === 'rw'
	export const assetDisplaysAsOutputInFlowGraph = (a: AssetWithAltAccessType) =>
		getAccessType(a) === 'w' || getAccessType(a) === 'rw'

	let computeAIToolNodesCache:
		| [(Node & NodeLayout)[], ReturnType<typeof computeAIToolNodes>]
		| undefined

	export function computeAIToolNodes(
		[nodes, edges]: [(Node & NodeLayout)[], Edge[]],
		eventHandlers: GraphEventHandlers
	): [(Node & NodeLayout)[], Edge[]] {
		if (nodes === computeAIToolNodesCache?.[0]) return computeAIToolNodesCache[1]

		const MAX_ASSET_ROW_WIDTH = 300
		const ASSETS_OVERFLOWED_NODE_WIDTH = 25
		const allAssetNodes: (Node & NodeLayout)[] = []
		const allAssetEdges: Edge[] = []

		const yPosMap: Record<number, { r?: true; w?: true }> = {}

		for (const node of nodes) {
			if (node.type !== 'module' || node.data.module.value.type !== 'aiagent') continue

			const assets: {
				id: string
				name: string
			}[] = node.data.module.value.tools.map((t) => ({
				id: t.id,
				name: t.summary ?? t.id
			}))

			// const assets = node.data.assets ?? []

			// Each asset can be displayed at the top and bottom
			// i.e once (R or W) or twice (RW)
			const inputAssets = assets
			const displayedInputAssets = inputAssets.slice(0, 3)

			const overflowedInputAssets = inputAssets.slice(3)

			// This allows calculating which nodes to offset on the y axis to
			// make space for the asset nodes
			if (inputAssets.length) yPosMap[node.position.y] = yPosMap[node.position.y] ?? {}
			if (inputAssets.length) yPosMap[node.position.y].r = true

			// All asset nodes displayed on top
			const inputAssetNodes: (Node & AiToolN)[] = displayedInputAssets.map((asset, i) => {
				let inputAssetXGap = 12
				let inputAssetWidth = 150

				const targetRowW =
					MAX_ASSET_ROW_WIDTH -
					(overflowedInputAssets.length ? ASSETS_OVERFLOWED_NODE_WIDTH + inputAssetXGap / 2 : 0)
				let totalInputRowWidth = () =>
					inputAssetWidth * displayedInputAssets.length +
					inputAssetXGap * (displayedInputAssets.length - 1)
				if (totalInputRowWidth() > MAX_ASSET_ROW_WIDTH) {
					const mult = targetRowW / totalInputRowWidth()
					inputAssetWidth = inputAssetWidth * mult
					inputAssetXGap = inputAssetXGap * mult
				}
				return {
					type: 'aiTool' as const,
					parentId: node.id,
					data: { tool: asset.name, eventHandlers, moduleId: asset.id },
					id: `${node.id}-tool-${asset.id}`,
					width: inputAssetWidth,
					position: {
						x:
							displayedInputAssets.length === 1
								? (NODE.width - inputAssetWidth) / 2 - 10 // Ensure we see the edge
								: (inputAssetWidth + inputAssetXGap) * (i - displayedInputAssets.length / 2) +
									(NODE.width + inputAssetXGap) / 2 +
									(overflowedInputAssets.length
										? (-ASSETS_OVERFLOWED_NODE_WIDTH - inputAssetXGap) / 2
										: 0),
						y: READ_ASSET_Y_OFFSET
					}
				}
			})

			const inputAssetEdges: Edge[] = inputAssetNodes?.map((n) => ({
				id: `${n.id}-edge`,
				source: n.id ?? '',
				target: n.parentId ?? '',
				type: 'empty',
				data: { class: '!opacity-35 dark:!opacity-20' }
			}))

			allAssetEdges.push(...(inputAssetEdges ?? []))
			allAssetNodes.push(...(inputAssetNodes ?? []))

			// If there are more than 3 assets, we create an overflow node
			// if (overflowedInputAssets.length)
			allAssetNodes.push({
				type: 'newAiTool',
				data: { eventHandlers, agentModuleId: node.data.module.id },
				id: `${node.id}-assets-overflowed-in`,
				parentId: node.id,
				width: ASSETS_OVERFLOWED_NODE_WIDTH,
				position: {
					x: MAX_ASSET_ROW_WIDTH - ASSETS_OVERFLOWED_NODE_WIDTH - 14,
					y: READ_ASSET_Y_OFFSET
				}
			} satisfies Node & NewAiToolN)
			// allAssetEdges.push({
			// 	id: `${node.id}-assets-overflowed-in-edge`,
			// 	source: `${node.id}-assets-overflowed-in`,
			// 	target: node.id,
			// 	type: 'empty',
			// 	data: { class: '!opacity-35 dark:!opacity-20' }
			// })
		}

		// Shift all nodes to make space for the new asset nodes
		const existingAssetNodes = nodes.filter((n) => n.type === 'asset')
		const sortedNewNodes = clone(nodes)
			.filter((n) => n.type !== 'asset')
			.sort((a, b) => a.position.y - b.position.y)
		let currentYOffset = 0
		let prevYPos = NaN
		for (const node of sortedNewNodes) {
			if (node.position.y !== prevYPos) {
				if (yPosMap[prevYPos]?.w) currentYOffset += NODE_WITH_WRITE_ASSET_Y_OFFSET
				if (yPosMap[node.position.y]?.r) currentYOffset += NODE_WITH_READ_ASSET_Y_OFFSET

				prevYPos = node.position.y
			}

			node.position.y += currentYOffset
		}

		let ret: ReturnType<typeof computeAIToolNodes> = [
			[...sortedNewNodes, ...existingAssetNodes, ...allAssetNodes],
			[...edges, ...allAssetEdges]
		]
		computeAIToolNodesCache = [nodes, ret]
		return ret
	}
</script>

<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import type {
		AiToolN,
		AssetN,
		AssetsOverflowedN,
		GraphEventHandlers,
		NewAiToolN,
		NodeLayout
	} from '../../graphBuilder.svelte'
	import { AlertTriangle, PocketKnifeIcon } from 'lucide-svelte'
	import {
		assetEq,
		formatAssetKind,
		getAccessType,
		type AssetWithAltAccessType
	} from '$lib/components/assets/lib'
	import { twMerge } from 'tailwind-merge'
	import type { FlowGraphAssetContext } from '$lib/components/flows/types'
	import { getContext } from 'svelte'
	import ExploreAssetButton, { assetCanBeExplored } from '../../../ExploreAssetButton.svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { clone, pluralize } from '$lib/utils'
	import type { Edge, Node } from '@xyflow/svelte'

	import { NODE } from '../../util'
	import { userStore } from '$lib/stores'

	interface Props {
		data: AiToolN['data']
	}

	const flowGraphAssetsCtx = getContext<FlowGraphAssetContext | undefined>('FlowGraphAssetContext')

	let { data }: Props = $props()

	const isSelected = $derived(flowGraphAssetsCtx?.val.selectedTool === data.tool)
	// const isSelected = $derived(assetEq(flowGraphAssetsCtx?.val.selectedAsset, data.asset))
	// const cachedResourceMetadata = $derived(
	// 	flowGraphAssetsCtx?.val.resourceMetadataCache[data.asset.path]
	// )
</script>

<NodeWrapper>
	{#snippet children({ darkMode })}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<Tooltip>
			<div
				class={twMerge(
					'bg-surface h-6 flex items-center gap-1.5 rounded-sm text-tertiary border overflow-clip',
					isSelected ? 'bg-surface-secondary !border-surface-inverse' : 'border-transparent'
				)}
				onmouseenter={() => flowGraphAssetsCtx && (flowGraphAssetsCtx.val.selectedTool = data.tool)}
				onmouseleave={() => flowGraphAssetsCtx && (flowGraphAssetsCtx.val.selectedTool = undefined)}
				onclick={() => data.eventHandlers.select(data.moduleId)}
			>
				<PocketKnifeIcon size={16} />

				<span class="text-3xs truncate flex-1">
					{data.tool}
				</span>
			</div>
			<svelte:fragment slot="text">
				{data.tool}
			</svelte:fragment>
		</Tooltip>
	{/snippet}
</NodeWrapper>
