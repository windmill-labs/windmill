<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import type { AssetN } from '../../graphBuilder.svelte'
	import { AlertTriangle } from 'lucide-svelte'
	import { assetEq } from '$lib/components/assets/lib'
	import { twMerge } from 'tailwind-merge'
	import type { FlowGraphAssetContext } from '$lib/components/flows/types'
	import { getContext } from 'svelte'
	import ExploreAssetButton, {
		assetCanBeExplored
	} from '../../../../../routes/(root)/(logged)/assets/ExploreAssetButton.svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { pluralize } from '$lib/utils'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'

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
