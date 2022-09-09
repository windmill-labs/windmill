<script lang="ts">
	import { flowStateStore } from '$lib/components/flows/flowState'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import FlowModule from './FlowModule.svelte'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	function selectedIdToIndexes(selectedId: string): number[] {
		return selectedId.split('-').map(Number)
	}

	$: [parentIndex, childIndex] = selectedIdToIndexes($selectedId)
</script>

{#if childIndex}
	<span>should handle child</span>
{:else if $flowStateStore[parentIndex]}
	<FlowModule
		indexes={$selectedId}
		args={{}}
		bind:flowModule={$flowStateStore[parentIndex].flowModule}
		bind:schema={$flowStateStore[parentIndex].schema}
		bind:childFlowModules={$flowStateStore[parentIndex].childFlowModules}
		on:delete={() => {}}
	/>
{/if}
