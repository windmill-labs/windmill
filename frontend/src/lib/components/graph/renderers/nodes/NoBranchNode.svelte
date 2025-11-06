<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { NoBranchN } from '../../graphBuilder.svelte'
	import { getGraphContext } from '../../graphContext'
	interface Props {
		data: NoBranchN['data']
	}

	let { data }: Props = $props()

	const { selectionManager } = getGraphContext()
</script>

<NodeWrapper offset={data.offset} enableSourceHandle enableTargetHandle>
	{#snippet children({ darkMode })}
		<VirtualItem
			label={data.label ?? 'No branches'}
			id={data.id}
			hideId={true}
			selectable={true}
			selected={selectionManager?.isNodeSelected(data.id)}
			on:select={(e) => {
				setTimeout(() => data?.eventHandlers?.select(e.detail))
			}}
		/>
	{/snippet}
</NodeWrapper>
