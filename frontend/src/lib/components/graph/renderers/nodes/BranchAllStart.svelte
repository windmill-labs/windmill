<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import { X } from 'lucide-svelte'
	import type { BranchAllStartN } from '../../graphBuilder.svelte'
	interface Props {
		data: BranchAllStartN['data']
		selected: boolean
	}

	let { data, selected }: Props = $props()
</script>

<NodeWrapper offset={data.offset}>
	{#snippet children({ darkMode })}
		<VirtualItem
			label={data.label}
			selectable
			{selected}
			on:select={() => {
				setTimeout(() => data.eventHandlers.select(data.id))
			}}
			on:insert={(e) => {
				setTimeout(() => data.eventHandlers.insert(e.detail))
			}}
		/>
		{#if data.insertable}
			<button
				title="Delete branch"
				class="z-50 absolute -translate-y-[100%] top-1 -right-1 rounded-md p-1 center-center text-primary
	bg-surface duration-0 hover:bg-red-400 hover:text-white shadow-md"
				onclick={stopPropagation(
					preventDefault(() => {
						data.eventHandlers.deleteBranch(
							{
								id: data.id,
								index: data.branchIndex
							},
							data.label
						)
					})
				)}
			>
				<X size={12} />
			</button>
		{/if}
	{/snippet}
</NodeWrapper>
