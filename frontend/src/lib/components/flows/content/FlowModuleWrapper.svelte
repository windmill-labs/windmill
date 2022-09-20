<script lang="ts">
	import { getContext } from 'svelte'
	import { flowStateStore } from '../flowState'
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
				args={{}}
				indexes={$selectedId}
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
				args={{}}
				indexes={$selectedId}
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
