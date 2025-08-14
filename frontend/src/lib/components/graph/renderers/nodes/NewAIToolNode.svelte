<!-- Displays as +n node instead of AssetNode when there are too many of themOverflowedAssetsNode -->

<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { type AssetsOverflowedN, type NewAiToolN } from '../../graphBuilder.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import AssetNode from './AssetNode.svelte'
	import type { FlowGraphAssetContext } from '$lib/components/flows/types'
	import { getContext } from 'svelte'
	import { assetEq } from '$lib/components/assets/lib'

	interface Props {
		data: NewAiToolN['data']
	}
	let { data }: Props = $props()
	const flowGraphAssetsCtx = getContext<FlowGraphAssetContext | undefined>('FlowGraphAssetContext')
</script>

<NodeWrapper>
	{#snippet children({ darkMode })}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<Popover
			portal={null}
			usePointerDownOutside
			class={twMerge(
				'!w-full text-2xs font-normal bg-surface h-6 pr-0.5 flex justify-center items-center rounded-sm text-tertiary border',
				'hover:bg-surface-secondary hover:border-surface-inverse active:opacity-55'
			)}
			placement="top"
		>
			<svelte:fragment slot="trigger">+tool</svelte:fragment>
			<svelte:fragment slot="content">
				<ul>
					<li
						class="w-48"
						onclick={() =>
							data.eventHandlers.insert({
								index: 0,
								kind: 'rawscript',
								inlineScript: {
									language: 'bun',
									kind: 'script'
								},
								agentId: data.agentModuleId
							})}
					>
						Bun
					</li>

					<li class="w-48"> Python </li>
				</ul>
			</svelte:fragment>
		</Popover>
	{/snippet}
</NodeWrapper>
