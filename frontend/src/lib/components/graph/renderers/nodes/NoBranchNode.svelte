<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { NoBranchN } from '../../graphBuilder.svelte'
	interface Props {
		data: NoBranchN['data']
		id: string
	}

	let { data, id }: Props = $props()
</script>

<NodeWrapper enableSourceHandle enableTargetHandle nodeId={id}>
	{#snippet children({ darkMode })}
		<VirtualItem
			label={data.label ?? 'No branches'}
			id={data.id}
			hideId={true}
			selectable={true}
			selected={false}
			on:select={(e) => {
				setTimeout(() => data?.eventHandlers?.select(e.detail))
			}}
		/>
	{/snippet}
</NodeWrapper>
