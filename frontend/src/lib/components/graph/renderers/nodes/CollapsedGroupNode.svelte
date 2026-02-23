<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import { Maximize2 } from 'lucide-svelte'
	import type { CollapsedGroupN } from '../../graphBuilder.svelte'
	import { getGraphContext } from '../../graphContext'

	interface Props {
		data: CollapsedGroupN['data']
		id: string
	}

	let { data, id }: Props = $props()

	const { selectionManager } = getGraphContext()
</script>

<NodeWrapper offset={data.offset}>
	{#snippet children({ darkMode })}
		<VirtualItem
			label={data.summary || 'Group'}
			preLabel={`${data.stepCount} steps`}
			selectable
			selected={selectionManager && selectionManager.isNodeSelected(id)}
		/>
		<div class="z-50 absolute -top-4 right-11 rounded-md text-primary bg-surface">
			<button
				title="Expand group"
				class="rounded-md center-center text-primary hover:bg-surface-tertiary shadow-md p-1 duration-0"
				onclick={stopPropagation(
					preventDefault(() => {
						data.eventHandlers.expandGroup(data.groupId)
					})
				)}
			>
				<Maximize2 size={12} />
			</button>
		</div>
	{/snippet}
</NodeWrapper>
