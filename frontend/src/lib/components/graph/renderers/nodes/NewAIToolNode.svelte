<!-- Displays as +n node instead of AssetNode when there are too many of themOverflowedAssetsNode -->

<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { type NewAiToolN } from '../../graphBuilder.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import InsertModuleInner from '$lib/components/flows/map/InsertModuleInner.svelte'

	let funcDesc = $state('')
	interface Props {
		data: NewAiToolN['data']
	}
	let { data }: Props = $props()
</script>

<NodeWrapper>
	{#snippet children({ darkMode })}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<Popover
			portal={null}
			usePointerDownOutside
			class={twMerge(
				'!w-full text-2xs font-normal bg-surface h-6 pr-0.5 flex justify-center items-center rounded-sm text-tertiary border',
				'hover:bg-surface-hover'
			)}
			placement="top"
		>
			<svelte:fragment slot="trigger">+tool</svelte:fragment>
			<svelte:fragment slot="content" let:close>
				<InsertModuleInner
					bind:funcDesc
					scriptOnly
					on:close={() => {
						close()
					}}
					on:new={(e) => {
						data.eventHandlers.insert({
							index: -1, // ignored when agentId is set
							agentId: data.agentModuleId,
							...e.detail
						})
						close()
					}}
					on:insert={(e) => {
						data.eventHandlers.insert({
							index: -1, // ignored when agentId is set
							agentId: data.agentModuleId,
							...e.detail
						})
						close()
					}}
					on:pickScript={(e) => {
						data.eventHandlers.insert({
							index: -1, // ignored when agentId is set
							agentId: data.agentModuleId,
							kind: e.detail.kind,
							script: {
								...e.detail,
								summary: e.detail.summary
									? e.detail.summary.replace(/\s/, '_').replace(/[^a-zA-Z0-9_]/g, '')
									: e.detail.path.split('/').pop()
							}
						})
						close()
					}}
				/>
			</svelte:fragment>
		</Popover>
	{/snippet}
</NodeWrapper>
