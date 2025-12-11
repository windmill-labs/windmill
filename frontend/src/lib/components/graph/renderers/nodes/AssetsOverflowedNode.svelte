<!-- Displays as +n node instead of AssetNode when there are too many of themOverflowedAssetsNode -->

<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { type AssetsOverflowedN } from '../../graphBuilder.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import AssetNode from './AssetNode.svelte'
	import type { FlowGraphAssetContext } from '$lib/components/flows/types'
	import { getContext } from 'svelte'
	import { assetEq } from '$lib/components/assets/lib'
	import { getNodeColorClasses } from '../../util'

	interface Props {
		data: AssetsOverflowedN['data']
	}
	let { data }: Props = $props()
	const flowGraphAssetsCtx = getContext<FlowGraphAssetContext | undefined>('FlowGraphAssetContext')

	let isOpen = $state(false)

	let includesSelected = $derived(
		data.overflowedAssets.some((asset) => assetEq(flowGraphAssetsCtx?.val.selectedAsset, asset))
	)

	let wasOpenedBecauseOfExternalSelected = false
	$effect(() => {
		if (includesSelected && !isOpen) {
			isOpen = true
			wasOpenedBecauseOfExternalSelected = true
		}
		if (wasOpenedBecauseOfExternalSelected && !includesSelected) {
			isOpen = false
			wasOpenedBecauseOfExternalSelected = false
		}
	})
	const colors = $derived(getNodeColorClasses(undefined, includesSelected))
</script>

<NodeWrapper>
	{#snippet children({ darkMode })}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<Popover
			portal={null}
			usePointerDownOutside
			bind:isOpen
			class={twMerge(
				'!w-full text-2xs font-normal h-6 pr-0.5 flex justify-center items-center rounded-md text-primary drop-shadow-base',
				'hover:bg-surface-hover active:bg-surface active:opacity-80',
				colors.bg,
				colors.outline,
				colors.text
			)}
			placement="top"
		>
			<svelte:fragment slot="trigger">
				+{data.overflowedAssets.length}
			</svelte:fragment>
			<svelte:fragment slot="content">
				<ul>
					{#each data.overflowedAssets as asset}
						<li class="w-48">
							<AssetNode data={{ asset }} />
						</li>
					{/each}
				</ul>
			</svelte:fragment>
		</Popover>
	{/snippet}
</NodeWrapper>
