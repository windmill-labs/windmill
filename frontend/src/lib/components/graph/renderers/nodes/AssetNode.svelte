<script module lang="ts">
	export const NODE_WITH_READ_ASSET_Y_OFFSET = 45
	export const NODE_WITH_WRITE_ASSET_Y_OFFSET = 45
	export const READ_ASSET_Y_OFFSET = -45
	export const WRITE_ASSET_Y_OFFSET = 64
	export const assetDisplaysAsInputInFlowGraph = (a: AssetWithAltAccessType) =>
		!getAccessType(a) || getAccessType(a) === 'r' || getAccessType(a) === 'rw'
	export const assetDisplaysAsOutputInFlowGraph = (a: AssetWithAltAccessType) =>
		getAccessType(a) === 'w' || getAccessType(a) === 'rw'

	let computeAssetNodesCache:
		| [(Node & NodeLayout)[], ReturnType<typeof computeAssetNodes>]
		| undefined

	export function computeAssetNodes(
		nodes: (Node & NodeLayout)[],
		edges: Edge[]
	): [(Node & NodeLayout)[], Edge[]] {
		if (nodes === computeAssetNodesCache?.[0]) return computeAssetNodesCache[1]

		const MAX_ASSET_ROW_WIDTH = 300
		const ASSETS_OVERFLOWED_NODE_WIDTH = 25
		const allAssetNodes: (Node & NodeLayout)[] = []
		const allAssetEdges: Edge[] = []

		const yPosMap: Record<number, { r?: true; w?: true }> = {}

		for (const node of nodes) {
			if (node.type !== 'module' && node.type !== 'input2') continue
			const assets = node.data.assets ?? []

			// Each asset can be displayed at the top and bottom
			// i.e once (R or W) or twice (RW)
			const inputAssets = assets.filter(assetDisplaysAsInputInFlowGraph)
			const outputAssets = assets.filter(assetDisplaysAsOutputInFlowGraph)

			const displayedInputAssets = inputAssets.slice(0, 3)
			const displayedOutputAssets = outputAssets.slice(0, 3)

			const overflowedInputAssets = inputAssets.slice(3)
			const overflowedOutputAssets = outputAssets.slice(3)

			// This allows calculating which nodes to offset on the y axis to
			// make space for the asset nodes
			if (inputAssets.length || outputAssets.length)
				yPosMap[node.position.y] = yPosMap[node.position.y] ?? {}
			if (inputAssets.length) yPosMap[node.position.y].r = true
			if (outputAssets.length) yPosMap[node.position.y].w = true

			// All asset nodes displayed on top
			const inputAssetNodes: (Node & AssetN)[] = displayedInputAssets.map((asset, i) => {
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
					type: 'asset' as const,
					parentId: node.id,
					data: { asset },
					id: `${node.id}-asset-in-${asset.kind}-${asset.path}`,
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

			// All asset nodes displayed on the bottom
			const outputAssetNodes: (Node & AssetN)[] = displayedOutputAssets.map((asset, i) => {
				let outputAssetXGap = 12
				let outputAssetWidth = 150

				const targetRowW =
					MAX_ASSET_ROW_WIDTH -
					(overflowedOutputAssets.length ? ASSETS_OVERFLOWED_NODE_WIDTH + outputAssetXGap / 2 : 0)
				let totalOutputRowWidth = () =>
					outputAssetWidth * displayedOutputAssets.length +
					outputAssetXGap * (displayedOutputAssets.length - 1)
				if (totalOutputRowWidth() > MAX_ASSET_ROW_WIDTH) {
					const mult = targetRowW / totalOutputRowWidth()
					outputAssetWidth = outputAssetWidth * mult
					outputAssetXGap = outputAssetXGap * mult
				}
				return {
					type: 'asset' as const,
					parentId: node.id,
					data: { asset },
					id: `${node.id}-asset-out-${asset.kind}-${asset.path}`,
					width: outputAssetWidth,
					position: {
						x:
							displayedOutputAssets.length === 1
								? (NODE.width - outputAssetWidth) / 2 - 10 // Ensure we see the edge
								: (outputAssetWidth + outputAssetXGap) * (i - displayedOutputAssets.length / 2) +
									(NODE.width + outputAssetXGap) / 2 +
									(overflowedOutputAssets.length
										? (-ASSETS_OVERFLOWED_NODE_WIDTH - outputAssetXGap) / 2
										: 0),
						y: WRITE_ASSET_Y_OFFSET
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
			const outputAssetEdges: Edge[] = outputAssetNodes?.map((n) => ({
				id: `${n.id}-edge`,
				source: n.parentId ?? '',
				target: n.id ?? '',
				type: 'empty',
				data: { class: '!opacity-35 dark:!opacity-20' }
			}))

			allAssetEdges.push(...(outputAssetEdges ?? []), ...(inputAssetEdges ?? []))
			allAssetNodes.push(...(inputAssetNodes ?? []), ...(outputAssetNodes ?? []))

			// If there are more than 3 assets, we create an overflow node
			if (overflowedInputAssets.length)
				allAssetNodes.push({
					type: 'assetsOverflowed',
					data: { overflowedAssets: overflowedInputAssets },
					id: `${node.id}-assets-overflowed-in`,
					parentId: node.id,
					width: ASSETS_OVERFLOWED_NODE_WIDTH,
					position: {
						x: MAX_ASSET_ROW_WIDTH - ASSETS_OVERFLOWED_NODE_WIDTH - 14,
						y: READ_ASSET_Y_OFFSET
					}
				} satisfies Node & AssetsOverflowedN)
			allAssetEdges.push({
				id: `${node.id}-assets-overflowed-in-edge`,
				source: `${node.id}-assets-overflowed-in`,
				target: node.id,
				type: 'empty',
				data: { class: '!opacity-35 dark:!opacity-20' }
			})
			if (overflowedOutputAssets.length)
				allAssetNodes.push({
					type: 'assetsOverflowed',
					data: { overflowedAssets: overflowedOutputAssets },
					id: `${node.id}-assets-overflowed-out`,
					parentId: node.id,
					width: ASSETS_OVERFLOWED_NODE_WIDTH,
					position: {
						x: MAX_ASSET_ROW_WIDTH - ASSETS_OVERFLOWED_NODE_WIDTH - 14,
						y: WRITE_ASSET_Y_OFFSET
					}
				} satisfies Node & AssetsOverflowedN)
			allAssetEdges.push({
				id: `${node.id}-assets-overflowed-out-edge`,
				source: node.id,
				target: `${node.id}-assets-overflowed-out`,
				type: 'empty',
				data: { class: '!opacity-35 dark:!opacity-25' }
			})
		}

		// Shift all nodes to make space for the new asset nodes
		const sortedNewNodes = clone(nodes.sort((a, b) => a.position.y - b.position.y))
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

		let ret: ReturnType<typeof computeAssetNodes> = [
			[...sortedNewNodes, ...allAssetNodes],
			[...edges, ...allAssetEdges]
		]
		computeAssetNodesCache = [nodes, ret]
		return ret
	}
</script>

<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import type { AssetN, AssetsOverflowedN, NodeLayout } from '../../graphBuilder.svelte'
	import { AlertTriangle } from 'lucide-svelte'
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
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import type { Edge, Node } from '@xyflow/svelte'

	import { NODE } from '../../util'
	import { userStore } from '$lib/stores'

	interface Props {
		data: AssetN['data']
	}

	const flowGraphAssetsCtx = getContext<FlowGraphAssetContext | undefined>('FlowGraphAssetContext')

	let { data }: Props = $props()

	const isSelected = $derived(assetEq(flowGraphAssetsCtx?.val.selectedAsset, data.asset))
	const cachedResourceMetadata = $derived(
		flowGraphAssetsCtx?.val.resourceMetadataCache[data.asset.path]
	)
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
				onmouseenter={() =>
					flowGraphAssetsCtx && (flowGraphAssetsCtx.val.selectedAsset = data.asset)}
				onmouseleave={() =>
					flowGraphAssetsCtx && (flowGraphAssetsCtx.val.selectedAsset = undefined)}
			>
				<AssetGenericIcon
					assetKind={data.asset.kind}
					fill={''}
					class="shrink-0 ml-1 fill-tertiary stroke-tertiary"
					size="16px"
				/>
				<span class="text-3xs truncate flex-1">
					{data.asset.path}
				</span>
				{#if data.asset.kind === 'resource' && cachedResourceMetadata === undefined}
					<Tooltip class={'pr-1 flex items-center justify-center'}>
						<AlertTriangle size={16} class="text-orange-500" />
						<svelte:fragment slot="text">Could not find resource</svelte:fragment>
					</Tooltip>
				{:else if isSelected && assetCanBeExplored(data.asset, cachedResourceMetadata) && !$userStore?.operator}
					<ExploreAssetButton
						btnClasses="rounded-none"
						asset={data.asset}
						noText
						buttonVariant="contained"
						s3FilePicker={flowGraphAssetsCtx?.val.s3FilePicker}
						dbManagerDrawer={flowGraphAssetsCtx?.val.dbManagerDrawer}
						_resourceMetadata={cachedResourceMetadata}
					/>
				{/if}
			</div>
			<svelte:fragment slot="text">
				Used in {pluralize(
					flowGraphAssetsCtx?.val.computeAssetsCount?.(data.asset) ?? -1,
					'step'
				)}<br />
				<a
					href={undefined}
					class={twMerge(
						'text-xs',
						data.asset.kind === 'resource'
							? 'text-blue-400 cursor-pointer'
							: 'dark:text-tertiary text-tertiary-inverse'
					)}
					onclick={() => {
						if (data.asset.kind === 'resource')
							flowGraphAssetsCtx?.val.resourceEditorDrawer?.initEdit(data.asset.path)
					}}
				>
					{data.asset.path}
				</a><br />
				<span class="dark:text-tertiary text-tertiary-inverse text-xs"
					>{formatAssetKind({ ...data.asset, metadata: cachedResourceMetadata })}</span
				>
				<br />
				fdsfs
			</svelte:fragment>
		</Tooltip>
	{/snippet}
</NodeWrapper>
