<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { ForLoopEndN } from '../../graphBuilder.svelte'
	interface Props {
		data: ForLoopEndN['data']
	}

	let { data }: Props = $props()
</script>

<NodeWrapper offset={data.offset}>
	{#snippet children({ darkMode })}
		{#if data.simplifiedTriggerView}
			<VirtualItem
				label={'Each event is processed'}
				selectable={false}
				selected={false}
				id={data.id}
				hideId
				on:select={(e) => {
					setTimeout(() => data?.eventHandlers?.select(e.detail))
				}}
			/>
		{:else}
			<VirtualItem
				label={'Collect result of each iteration'}
				selectable={true}
				selected={false}
				id={data.id}
				on:select={(e) => {
					setTimeout(() => data?.eventHandlers?.select(e.detail))
				}}
			/>
		{/if}
	{/snippet}
</NodeWrapper>
