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

{#if $flowStateStore.modules[parentIndex] && $flowStateStore.modules[parentIndex].childFlowModules !== undefined}
	{#each $flowStateStore.modules[parentIndex].childFlowModules ?? [] as fa, index}
		{#if index === childIndex}
			<FlowModule
				indexes={$selectedId}
				args={{}}
				bind:flowModule={fa.flowModule}
				bind:schema={fa.schema}
				bind:childFlowModules={fa.childFlowModules}
				on:delete={() => {
					$flowStateStore.modules[parentIndex].childFlowModules?.splice(index, 1)
					$flowStateStore = $flowStateStore
				}}
			/>
		{/if}
	{/each}
{:else}
	{#each $flowStateStore.modules ?? [] as fa, index}
		{#if index === parentIndex}
			<FlowModule
				indexes={$selectedId}
				args={{}}
				bind:flowModule={fa.flowModule}
				bind:schema={fa.schema}
				bind:childFlowModules={fa.childFlowModules}
				on:delete={() => {
					$flowStateStore.modules.splice(index, 1)
					$flowStateStore = $flowStateStore
				}}
			/>
		{/if}
	{/each}
{/if}
