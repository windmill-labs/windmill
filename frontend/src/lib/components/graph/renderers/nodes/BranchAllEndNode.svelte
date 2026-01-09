<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { BranchAllEndN } from '../../graphBuilder.svelte'
	import { getGraphContext } from '../../graphContext'
	interface Props {
		data: BranchAllEndN['data']
		id: string
	}

	let { data, id }: Props = $props()

	const { selectionManager } = getGraphContext()
</script>

<NodeWrapper offset={data.offset} enableSourceHandle enableTargetHandle>
	{#snippet children({ darkMode })}
		<VirtualItem
			label={'Collect result from all branches'}
			id={data.id}
			selectable={true}
			selected={selectionManager && selectionManager.isNodeSelected(id)}
			on:select={(e) => {
				data?.eventHandlers?.select(e.detail)
			}}
		/>
	{/snippet}
</NodeWrapper>
