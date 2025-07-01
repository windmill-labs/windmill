<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import type { AssetN } from '../../graphBuilder.svelte'
	import { Pyramid } from 'lucide-svelte'
	import { assetEq, formatAsset } from '$lib/components/assets/lib'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '$lib/components/flows/types'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		data: AssetN['data']
	}

	const { selectedAssetStore } = getContext<FlowEditorContext>('FlowEditorContext') ?? {}
	let { data }: Props = $props()
	const isSelected = $derived(assetEq(selectedAssetStore.val, data.asset))
</script>

<NodeWrapper>
	{#snippet children({ darkMode })}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class={twMerge(
				'bg-surface py-1 px-1.5 flex gap-1.5 rounded-sm text-tertiary border',
				isSelected ? 'bg-surface-hover border-surface-inverse' : 'border-transparent'
			)}
			onmouseenter={() => (selectedAssetStore.val = data.asset)}
			onmouseleave={() => (selectedAssetStore.val = undefined)}
		>
			<Pyramid size={16} class="shrink-0" />
			<span class="text-3xs truncate">
				{formatAsset(data.asset)}
			</span>
		</div>
	{/snippet}
</NodeWrapper>
