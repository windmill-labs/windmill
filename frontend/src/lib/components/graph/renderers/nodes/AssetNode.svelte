<script module lang="ts">
	export const NODE_WITH_READ_ASSET_Y_OFFSET = 45
	export const NODE_WITH_WRITE_ASSET_Y_OFFSET = 45
	export const READ_ASSET_Y_OFFSET = -45
	export const WRITE_ASSET_Y_OFFSET = 64
	export const assetDisplaysAsInputInFlowGraph = (a: AssetWithAltAccessType) =>
		!getAccessType(a) || getAccessType(a) === 'r' || getAccessType(a) === 'rw'
	export const assetDisplaysAsOutputInFlowGraph = (a: AssetWithAltAccessType) =>
		getAccessType(a) === 'w' || getAccessType(a) === 'rw'

	let computeAssetNodesCache: [NodeDep[], ReturnType<typeof computeAssetNodes>] | undefined

	type NodeDep = {
		data: object & { assets?: AssetWithAltAccessType[] | undefined; offset?: number }
		id: string
		position: { x: number; y: number }
	}

	export function computeAssetNodes(nodes: NodeDep[]): {
		newAssetNodes: (Node & NodeLayout)[]
		newAssetEdges: Edge[]
		// Nodes need to be offset on the y axis to make space for the asset nodes
		newNodePositions: Record<string, { x: number; y: number }>
	} {
		if (computeAssetNodesCache && deepEqual(nodes, computeAssetNodesCache[0])) {
			return computeAssetNodesCache[1]
		}
		const MAX_ASSET_ROW_WIDTH = 300
		const ASSETS_OVERFLOWED_NODE_WIDTH = 25
		const allAssetNodes: (Node & NodeLayout)[] = []
		const allAssetEdges: Edge[] = []

		const yPosMap: Record<number, { r?: true; w?: true }> = {}

		for (const node of nodes) {
			const assets = node.data.assets ?? []
			if (!assets.length) continue

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
				let inputAssetWidth = 165

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
					data: { asset, displayedAccessType: 'r' },
					id: `${node.id}-asset-in-${asset.kind}-${asset.path}`,
					width: inputAssetWidth,
					position: {
						x:
							(node.data.offset ?? 0) +
							(displayedInputAssets.length === 1
								? (NODE.width - inputAssetWidth) / 2 - 10 // Ensure we see the edge
								: (inputAssetWidth + inputAssetXGap) * (i - displayedInputAssets.length / 2) +
									(NODE.width + inputAssetXGap) / 2 +
									(overflowedInputAssets.length
										? (-ASSETS_OVERFLOWED_NODE_WIDTH - inputAssetXGap) / 2
										: 0)),
						y: READ_ASSET_Y_OFFSET
					},
					selectable: false
				}
			})

			// All asset nodes displayed on the bottom
			const outputAssetNodes: (Node & AssetN)[] = displayedOutputAssets.map((asset, i) => {
				let outputAssetXGap = 12
				let outputAssetWidth = 165

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
					data: { asset, displayedAccessType: 'w' },
					id: `${node.id}-asset-out-${asset.kind}-${asset.path}`,
					width: outputAssetWidth,
					position: {
						x:
							(node.data.offset ?? 0) +
							(displayedOutputAssets.length === 1
								? (NODE.width - outputAssetWidth) / 2 - 10 // Ensure we see the edge
								: (outputAssetWidth + outputAssetXGap) * (i - displayedOutputAssets.length / 2) +
									(NODE.width + outputAssetXGap) / 2 +
									(overflowedOutputAssets.length
										? (-ASSETS_OVERFLOWED_NODE_WIDTH - outputAssetXGap) / 2
										: 0)),
						y: WRITE_ASSET_Y_OFFSET
					},
					selectable: false
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
					data: { overflowedAssets: overflowedInputAssets, displayedAccessType: 'r' },
					id: `${node.id}-assets-overflowed-in`,
					parentId: node.id,
					width: ASSETS_OVERFLOWED_NODE_WIDTH,
					position: {
						x: (node.data.offset ?? 0) + MAX_ASSET_ROW_WIDTH - ASSETS_OVERFLOWED_NODE_WIDTH - 14,
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
					data: { overflowedAssets: overflowedOutputAssets, displayedAccessType: 'w' },
					id: `${node.id}-assets-overflowed-out`,
					parentId: node.id,
					width: ASSETS_OVERFLOWED_NODE_WIDTH,
					position: {
						x: (node.data.offset ?? 0) + MAX_ASSET_ROW_WIDTH - ASSETS_OVERFLOWED_NODE_WIDTH - 14,
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
		const sortedNewNodes = nodes
			.map((n) => ({ position: { ...n.position }, id: n.id }))
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

		let ret: ReturnType<typeof computeAssetNodes> = {
			newAssetNodes: allAssetNodes,
			newAssetEdges: allAssetEdges,
			newNodePositions: Object.fromEntries(sortedNewNodes.map((n) => [n.id, n.position]))
		}
		computeAssetNodesCache = [clone(nodes), ret]
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
		formatShortAssetPath,
		getAccessType,
		type AssetWithAltAccessType
	} from '$lib/components/assets/lib'
	import { twMerge } from 'tailwind-merge'
	import { getContext } from 'svelte'
	import ExploreAssetButton, { assetCanBeExplored } from '../../../ExploreAssetButton.svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { clone, pluralize } from '$lib/utils'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import type { Edge, Node } from '@xyflow/svelte'

	import { getNodeColorClasses, NODE } from '../../util'
	import { globalDbManagerDrawer, userStore } from '$lib/stores'
	import { deepEqual } from 'fast-equals'
	import { slide } from 'svelte/transition'
	import AssetColumnBadges from '$lib/components/assets/AssetColumnBadges.svelte'

	interface Props {
		data: AssetN['data']
	}

	const flowGraphAssetsCtx = getContext<any | undefined>('FlowGraphAssetContext')

	let { data }: Props = $props()

	const isSelected = $derived(assetEq(flowGraphAssetsCtx?.val.selectedAsset, data.asset))
	const cachedResourceMetadata = $derived.by(() => {
		if (data.asset.kind !== 'resource') return undefined
		let truncatedPath = data.asset.path.split('?table=')[0]
		return flowGraphAssetsCtx?.val.resourceMetadataCache[truncatedPath]
	})
	const usageCount = $derived(flowGraphAssetsCtx?.val.computeAssetsCount?.(data.asset))
	const colors = $derived(getNodeColorClasses(undefined, isSelected))

	let assetColumns = $derived(
		data.asset.columns &&
			Object.fromEntries(
				Object.entries(data.asset.columns).filter(
					([_, accessType]) => accessType && accessType === data.displayedAccessType
				)
			)
	)
</script>

<NodeWrapper wrapperClass="bg-surface-secondary rounded-md">
	{#snippet children({ darkMode })}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<Tooltip customBgClass="bg-surface-tertiary">
			<div
				class={twMerge(
					'h-6 flex items-center rounded-md drop-shadow-base overflow-clip transition-colors',
					colors.outline,
					colors.text,
					colors.bg
				)}
				onmouseenter={() =>
					flowGraphAssetsCtx && (flowGraphAssetsCtx.val.selectedAsset = data.asset)}
				onmouseleave={() =>
					flowGraphAssetsCtx && (flowGraphAssetsCtx.val.selectedAsset = undefined)}
			>
				<AssetGenericIcon
					assetKind={data.asset.kind}
					class="shrink-0 ml-1 mr-1.5 {isSelected ? 'text-accent' : 'text-tertiary'}"
					size="16px"
				/>
				<span
					class="text-3xs truncate flex-1 flex items-center gap-1 [mask-image:linear-gradient(to_right,black_85%,transparent)] mr-0.5"
				>
					{formatShortAssetPath(data.asset)}
					<AssetColumnBadges
						columns={assetColumns}
						disableTooltip
						disableWrap
						badgeClasses="text-3xs transition-opacity opacity-50 {isSelected ? 'opacity-0' : ''}"
					/>
				</span>
				{#if data.asset.kind === 'resource' && cachedResourceMetadata === undefined}
					<Tooltip class={'pr-1 flex items-center justify-center'}>
						<AlertTriangle size={16} class="text-orange-500" />
						<svelte:fragment slot="text">Could not find resource</svelte:fragment>
					</Tooltip>
				{:else if isSelected && assetCanBeExplored(data.asset, cachedResourceMetadata) && !$userStore?.operator}
					<div transition:slide={{ axis: 'x', duration: 100 }}>
						<ExploreAssetButton
							btnClasses="rounded-none"
							asset={data.asset}
							noText
							buttonVariant="accent"
							s3FilePicker={flowGraphAssetsCtx?.val.s3FilePicker}
							dbManagerDrawer={globalDbManagerDrawer.val}
							_resourceMetadata={cachedResourceMetadata}
						/>
					</div>
				{/if}
			</div>
			<svelte:fragment slot="text">
				{#if usageCount !== undefined}
					Used in {pluralize(usageCount, 'step')}<br />
				{/if}
				<a
					href={undefined}
					class={twMerge(
						'text-xs',
						data.asset.kind === 'resource' ? 'text-accent cursor-pointer' : 'text-hint'
					)}
					onclick={() => {
						if (data.asset.kind === 'resource')
							flowGraphAssetsCtx?.val.resourceEditorDrawer?.initEdit(data.asset.path)
					}}
				>
					{data.asset.path}
				</a><br />
				<span class="text-hint text-xs">
					{formatAssetKind({ ...data.asset, metadata: cachedResourceMetadata })}</span
				>
				<AssetColumnBadges columns={assetColumns} disableTooltip />
			</svelte:fragment>
		</Tooltip>
	{/snippet}
</NodeWrapper>
