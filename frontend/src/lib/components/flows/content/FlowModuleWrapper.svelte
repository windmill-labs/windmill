<script lang="ts">
	import { getContext } from 'svelte'
	import { flowStateStore } from '../flowState'
	import { flowStore } from '../flowStore'
	import type { FlowEditorContext } from '../types'
	import { selectedIdToIndexes } from '../utils'
	import FlowModule from './FlowModule.svelte'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	$: [parentIndex, childIndex] = selectedIdToIndexes($selectedId)
</script>

{#if childIndex != undefined}
	{#each [$flowStore.value.modules[parentIndex].value] as mod, index (index)}
		{#each [$flowStateStore.modules[parentIndex].childFlowModules] as state}
			{#if mod.type == 'forloopflow' && state != undefined}
				<FlowModule
					failureModule={false}
					bind:flowModule={mod.modules[childIndex]}
					bind:flowModuleState={state[childIndex]}
					on:delete={() => {
						$flowStateStore.modules[parentIndex].childFlowModules?.splice(childIndex, 1)
						let mod = $flowStore.value.modules[parentIndex].value
						if (mod.type === 'forloopflow') {
							mod.modules.splice(childIndex, 1)
						} else {
							throw new Error('Expected forloop')
						}
					}}
				/>
			{:else}
				<span>Incorrect state</span>
			{/if}
		{/each}
	{/each}
{:else if $flowStore.value.modules[parentIndex]}
	<FlowModule
		failureModule={false}
		bind:flowModule={$flowStore.value.modules[parentIndex]}
		bind:flowModuleState={$flowStateStore.modules[parentIndex]}
		on:delete={() => {
			$flowStateStore.modules.splice(parentIndex, 1)
			$flowStateStore = $flowStateStore
			$flowStore.value.modules.splice(parentIndex, 1)
			$flowStore = $flowStore
		}}
	/>
{/if}
