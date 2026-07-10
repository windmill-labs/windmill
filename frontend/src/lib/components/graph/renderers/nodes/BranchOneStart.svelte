<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import { X } from 'lucide-svelte'
	import type { BranchOneStartN } from '../../graphBuilder.svelte'
	import { getGraphContext } from '../../graphContext'
	import { computeBorderStatus } from '../utils'
	interface Props {
		data: BranchOneStartN['data']
		id: string
	}
	const { selectionManager } = getGraphContext()

	let { data, id }: Props = $props()

	// branchIndex is -1 for the default branch and 0-based for explicit branches;
	// branchChosen is 0 for default and 1-based, hence the +1.
	let borderStatus = $derived(
		computeBorderStatus(data.branchIndex + 1, 'branchone', data.flowModuleState)
	)
</script>

<NodeWrapper nodeId={id}>
	{#snippet children({ darkMode })}
		<VirtualItem
			label={data.label}
			preLabel={data.preLabel}
			selectable
			selected={selectionManager && selectionManager.isNodeSelected(id)}
			borderState={borderStatus}
			on:select={() => {
				setTimeout(() => data?.eventHandlers?.select(data.id))
			}}
		/>
		{#if data.insertable && data.branchIndex >= 0}
			<button
				title="Delete branch"
				class="z-50 absolute -translate-y-[100%] top-1 -right-1 rounded-md p-1 center-center text-primary
	bg-surface duration-0 hover:bg-red-400 hover:text-white shadow-md"
				onclick={stopPropagation(
					preventDefault(() => {
						data.eventHandlers.deleteBranch(
							{
								id: data.id,
								index: data.branchIndex + 1
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
