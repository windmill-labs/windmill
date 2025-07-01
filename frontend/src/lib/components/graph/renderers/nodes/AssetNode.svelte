<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import type { AssetN } from '../../graphBuilder.svelte'
	import { AlertTriangle, Pyramid } from 'lucide-svelte'
	import { assetEq, formatAsset } from '$lib/components/assets/lib'
	import { twMerge } from 'tailwind-merge'
	import type { FlowGraphAssetContext } from '$lib/components/flows/types'
	import { getContext } from 'svelte'
	import ExploreAssetButton, {
		assetCanBeExplored
	} from '../../../../../routes/(root)/(logged)/assets/ExploreAssetButton.svelte'
	import { Tooltip } from '$lib/components/meltComponents'

	interface Props {
		data: AssetN['data']
	}

	const flowGraphAssetsCtx = getContext<FlowGraphAssetContext>('FlowGraphAssetContext')

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
				<Pyramid size={16} class="shrink-0 ml-1" />
				<span class="text-3xs truncate">
					{formatAsset(data.asset)}
				</span>
				{#if isSelected}
					{#if data.asset.kind === 'resource' && flowGraphAssetsCtx.val.resourceMetadataCache[data.asset.path] === undefined}
						<Tooltip class="mr-2.5">
							<AlertTriangle size={16} class="text-orange-500" />
							<svelte:fragment slot="text">Could not fetch resource</svelte:fragment>
						</Tooltip>
					{/if}
					{#if assetCanBeExplored(data.asset, flowGraphAssetsCtx.val.resourceMetadataCache[data.asset.path])}
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
				{/if}
			</div>
			<svelte:fragment slot="text">{formatAsset(data.asset)}</svelte:fragment>
		</Tooltip>
	{/snippet}
</NodeWrapper>
