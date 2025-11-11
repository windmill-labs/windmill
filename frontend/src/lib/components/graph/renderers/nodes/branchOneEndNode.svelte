<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { BranchOneEndN } from '../../graphBuilder.svelte'
	interface Props {
		data: BranchOneEndN['data']
	}

	let { data }: Props = $props()
</script>

<NodeWrapper offset={data.offset}>
	{#snippet children({ darkMode })}
		<VirtualItem
			label={'Collect result from chosen branch'}
			id={data.id}
			selectable={true}
			selected={false}
			on:select={(e) => {
				setTimeout(() => data?.eventHandlers?.select(e.detail))
			}}
		/>
	{/snippet}
</NodeWrapper>
