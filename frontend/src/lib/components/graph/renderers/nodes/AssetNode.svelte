<script module lang="ts">
	export const NODE_WITH_READ_ASSET_Y_OFFSET = 45
	export const NODE_WITH_WRITE_ASSET_Y_OFFSET = 45
	export const READ_ASSET_Y_OFFSET = -45
	export const WRITE_ASSET_Y_OFFSET = 64
	export const assetDisplaysAsInputInFlowGraph = (a: { access_type?: AssetUsageAccessType }) =>
		!a.access_type || a.access_type === 'r' || a.access_type === 'rw'
	export const assetDisplaysAsOutputInFlowGraph = (a: { access_type?: AssetUsageAccessType }) =>
		a.access_type === 'w' || a.access_type === 'rw'

	let computeAssetNodesCache:
		| [Node[], Record<string, AssetWithAccessType[]>, ReturnType<typeof computeAssetNodes>]
		| undefined

	export function computeAssetNodes(
		nodes: Node[],
		edges: Edge[],
		assetsMap: Record<string, AssetWithAccessType[]>,
		extraData: any
	): [Node[], Edge[]] {
		if (nodes === computeAssetNodesCache?.[0] && deepEqual(assetsMap, computeAssetNodesCache?.[1]))
			return computeAssetNodesCache[2]

		const MAX_ASSET_ROW_WIDTH = 300

		const allAssetNodes: (Node & AssetN)[] = []
		const allAssetEdges: Edge[] = []

		const yPosMap: Record<number, { r?: true; w?: true }> = {}

		for (const node of nodes) {
			const assets = assetsMap?.[node.id] ?? []

			// Each asset can be displayed at the top and bottom
			// i.e once (R or W) or twice (RW)
			const inputAssets = assets.filter(assetDisplaysAsInputInFlowGraph)
			const outputAssets = assets.filter(assetDisplaysAsOutputInFlowGraph)

			// This allows calculating which nodes to offset on the y axis to
			// make space for the asset nodes
			if (inputAssets.length || outputAssets.length)
				yPosMap[node.position.y] = yPosMap[node.position.y] ?? {}
			if (inputAssets.length) yPosMap[node.position.y].r = true
			if (outputAssets.length) yPosMap[node.position.y].w = true

			const inputAssetNodes: (Node & AssetN)[] = inputAssets.map((asset, i) => {
				let inputAssetXGap = 20
				let inputAssetWidth = 180
				let totalInputRowWidth = () =>
					inputAssetWidth * inputAssets.length + inputAssetXGap * (inputAssets.length - 1)
				if (totalInputRowWidth() > MAX_ASSET_ROW_WIDTH) {
					const mult = MAX_ASSET_ROW_WIDTH / totalInputRowWidth()
					inputAssetWidth = inputAssetWidth * mult
					inputAssetXGap = inputAssetXGap * mult
				}
				return {
					type: 'asset' as const,
					parentId: node.id,
					data: { asset, displayedAs: 'input' as const },
					id: `${node.id}-asset-in-${asset.kind}-${asset.path}`,
					width: inputAssetWidth,
					position: {
						x:
							inputAssets.length === 1
								? (NODE.width - inputAssetWidth) / 2 - 10 // Ensure we see the edge
								: (inputAssetWidth + inputAssetXGap) * (i - inputAssets.length / 2) +
									(NODE.width + inputAssetXGap) / 2,
						y: READ_ASSET_Y_OFFSET
					}
				}
			})

			const outputAssetNodes: (Node & AssetN)[] = outputAssets.map((asset, i) => {
				let outputAssetXGap = 20
				let outputAssetWidth = 180
				let totalOutputRowWidth = () =>
					outputAssetWidth * outputAssets.length + outputAssetXGap * (outputAssets.length - 1)
				if (totalOutputRowWidth() > MAX_ASSET_ROW_WIDTH) {
					const mult = MAX_ASSET_ROW_WIDTH / totalOutputRowWidth()
					outputAssetWidth = outputAssetWidth * mult
					outputAssetXGap = outputAssetXGap * mult
				}
				return {
					type: 'asset' as const,
					parentId: node.id,
					data: { asset, displayedAs: 'output' as const },
					id: `${node.id}-asset-out-${asset.kind}-${asset.path}`,
					width: outputAssetWidth,
					position: {
						x:
							outputAssets.length === 1
								? (NODE.width - outputAssetWidth) / 2 - 10 // Ensure we see the edge
								: (outputAssetWidth + outputAssetXGap) * (i - outputAssets.length / 2) +
									(NODE.width + outputAssetXGap) / 2,
						y: WRITE_ASSET_Y_OFFSET
					}
				}
			})

			const assetNodes = [...inputAssetNodes, ...outputAssetNodes]

			const assetEdges = assetNodes?.map((n) => {
				const source = (n.data.displayedAs === 'output' ? n.parentId : n.id) ?? ''
				const target = (n.data.displayedAs === 'output' ? n.id : n.parentId) ?? ''
				return {
					id: `${n.id}-edge`,
					source,
					target,
					type: 'empty',
					data: { class: '!opacity-35' }
				} satisfies Edge
			})

			allAssetEdges.push(...(assetEdges ?? []))
			allAssetNodes.push(...(assetNodes ?? []))
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
		computeAssetNodesCache = [nodes, clone(assetsMap), ret]
		return ret
	}
</script>

<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import type { AssetN } from '../../graphBuilder.svelte'
	import { AlertTriangle } from 'lucide-svelte'
	import { assetEq, type AssetWithAccessType } from '$lib/components/assets/lib'
	import { twMerge } from 'tailwind-merge'
	import type { FlowGraphAssetContext } from '$lib/components/flows/types'
	import { getContext } from 'svelte'
	import ExploreAssetButton, {
		assetCanBeExplored
	} from '../../../../../routes/(root)/(logged)/assets/ExploreAssetButton.svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { clone, pluralize } from '$lib/utils'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import type { Edge, Node } from '@xyflow/svelte'
	import { deepEqual } from 'fast-equals'

	import { NODE } from '../../util'
	import type { AssetUsageAccessType } from '$lib/gen'

	interface Props {
		data: AssetN['data']
	}

	const flowGraphAssetsCtx = getContext<FlowGraphAssetContext>('FlowGraphAssetContext')

	const usageCount = $derived(
		Object.values(flowGraphAssetsCtx.val.assetsMap ?? {})
			.flat()
			.filter((asset) => assetEq(asset, data.asset)).length
	)

	let { data }: Props = $props()
	const isSelected = $derived(assetEq(flowGraphAssetsCtx.val.selectedAsset, data.asset))
</script>

<NodeWrapper>
	{#snippet children({ darkMode })}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<Tooltip>
			<div
				class={twMerge(
					'bg-surface h-6 flex items-center gap-1.5 rounded-sm text-tertiary border overflow-clip',
					isSelected ? 'bg-surface-secondary border-surface-inverse' : 'border-transparent'
				)}
				onmouseenter={() => (flowGraphAssetsCtx.val.selectedAsset = data.asset)}
				onmouseleave={() => (flowGraphAssetsCtx.val.selectedAsset = undefined)}
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
				{#if data.asset.kind === 'resource' && flowGraphAssetsCtx.val.resourceMetadataCache[data.asset.path] === undefined}
					<Tooltip class={'pr-1 flex items-center justify-center'}>
						<AlertTriangle size={16} class="text-orange-500" />
						<svelte:fragment slot="text">Could not fetch resource</svelte:fragment>
					</Tooltip>
				{:else if isSelected && assetCanBeExplored(data.asset, flowGraphAssetsCtx.val.resourceMetadataCache[data.asset.path])}
					<ExploreAssetButton
						btnClasses="rounded-none"
						asset={data.asset}
						noText
						buttonVariant="contained"
						s3FilePicker={flowGraphAssetsCtx.val.s3FilePicker}
						dbManagerDrawer={flowGraphAssetsCtx.val.dbManagerDrawer}
						_resourceMetadata={flowGraphAssetsCtx.val.resourceMetadataCache[data.asset.path]}
					/>
				{/if}
			</div>
			<svelte:fragment slot="text">
				Used in {pluralize(usageCount, 'step')}<br />
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
							flowGraphAssetsCtx.val.resourceEditorDrawer?.initEdit(data.asset.path)
					}}
				>
					{data.asset.path}
				</a><br />
				<span class="dark:text-tertiary text-tertiary-inverse text-xs">{data.asset.kind}</span>
			</svelte:fragment>
		</Tooltip>
	{/snippet}
</NodeWrapper>
