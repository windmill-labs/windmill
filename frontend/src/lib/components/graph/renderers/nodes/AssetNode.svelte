<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import type { AssetN } from '../../graphBuilder.svelte'
	import { Pyramid } from 'lucide-svelte'
	import { assetEq, formatAsset } from '$lib/components/assets/lib'
	import { twMerge } from 'tailwind-merge'
	import type { FlowGraphAssetContext } from '$lib/components/flows/types'
	import { getContext } from 'svelte'

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
		<div
			class={twMerge(
				'bg-surface py-1 px-1.5 flex gap-1.5 rounded-sm text-tertiary border',
				isSelected ? 'bg-surface-hover border-surface-inverse' : 'border-transparent'
			)}
			onmouseenter={() => (flowGraphAssetsCtx.val.selectedAsset = data.asset)}
			onmouseleave={() => (flowGraphAssetsCtx.val.selectedAsset = undefined)}
		>
			<Pyramid size={16} class="shrink-0" />
			<span class="text-3xs truncate">
				{formatAsset(data.asset)}
			</span>
		</div>
	{/snippet}
</NodeWrapper>
