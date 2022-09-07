<script lang="ts">
	import { flowStateStore } from '$lib/components/flows/flowState'
	import FlowModule from './FlowModule.svelte'

	export let selectedId: string

	function selectedIdToIndexes(selectedId: string): number[] {
		return selectedId.split('-').map(Number)
	}

	const [parentIndex, childIndex] = selectedIdToIndexes(selectedId)
</script>

{#if childIndex}
	<span>should handle child</span>
{:else if $flowStateStore[parentIndex]}
	<FlowModule
		indexes={selectedId}
		args={{}}
		bind:flowModule={$flowStateStore[parentIndex].flowModule}
		bind:schema={$flowStateStore[parentIndex].schema}
		bind:childFlowModules={$flowStateStore[parentIndex].childFlowModules}
		on:delete={() => {}}
	/>
{/if}
