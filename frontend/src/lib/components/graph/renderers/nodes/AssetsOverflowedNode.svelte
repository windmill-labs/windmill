<!-- Displays as +n node instead of AssetNode when there are too many of themOverflowedAssetsNode -->

<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { type AssetsOverflowedN } from '../../graphBuilder.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import AssetNode from './AssetNode.svelte'
	import type { FlowGraphAssetContext } from '$lib/components/flows/types'
	import { getContext, untrack } from 'svelte'
	import { assetEq } from '$lib/components/assets/lib'
	import { getNodeColorClasses } from '../../util'

	interface Props {
		data: AssetsOverflowedN['data']
		id?: string
	}
	let { data, id }: Props = $props()
	const flowGraphAssetsCtx = getContext<FlowGraphAssetContext | undefined>('FlowGraphAssetContext')

	let isOpen = $state(false)

	let includesSelected = $derived(
		data.overflowedAssets.some((asset) => assetEq(flowGraphAssetsCtx?.val.selectedAsset, asset))
	)

	// Open while a sibling asset node is hovered and one of the hidden assets is the same asset.
	let openedByHover = $state(false)
	$effect(() => {
		if (includesSelected) {
			if (!untrack(() => isOpen)) {
				isOpen = true
				openedByHover = true
			}
		} else if (untrack(() => openedByHover)) {
			isOpen = false
			openedByHover = false
		}
	})
	const colors = $derived(getNodeColorClasses(undefined, includesSelected))
</script>

<NodeWrapper nodeId={id}>
	{#snippet children({ darkMode })}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- A hover-opened popover must stay passive. It is rendered in place, so it can
		     land on top of the hovered asset node: taking the pointer there ends that hover,
		     which closes the popover and re-opens it as soon as the cursor is back on the
		     node. Moving focus into it would likewise steal focus from wherever the user is. -->
		<Popover
			portal={null}
			usePointerDownOutside
			disableFocusTrap
			openFocus={null}
			closeFocus={null}
			contentClasses={openedByHover ? 'pointer-events-none' : ''}
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
			{#snippet trigger()}
				+{data.overflowedAssets.length}
			{/snippet}
			{#snippet content()}
				<ul>
					{#each data.overflowedAssets as asset}
						<li class="w-48">
							<AssetNode data={{ asset, displayedAccessType: data.displayedAccessType }} />
						</li>
					{/each}
				</ul>
			{/snippet}
		</Popover>
	{/snippet}
</NodeWrapper>
