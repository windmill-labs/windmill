<script lang="ts">
	import { getContext } from 'svelte'

	import type { FlowEditorContext } from '../types'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import FlowSettings from './FlowSettings.svelte'
	import FlowInput from './FlowInput.svelte'
	import FlowFailureModule from './FlowFailureModule.svelte'
	import FlowConstants from './FlowConstants.svelte'
	import type { FlowModule } from '$lib/gen'

	const { selectedId, flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	// function checkDup(modules: FlowModule[]): FlowModule[] {
	// 	let seenModules: FlowModule[] = []
	// 	for (const m of modules) {
	// 		if (seenModules.find((sm) => sm.id === m.id)) {
	// 			console.error(`Duplicate module id: ${m.id}`)
	// 			continue
	// 		}
	// 		seenModules.push(m)
	// 	}
	// 	return seenModules
	// }
</script>

{#if $selectedId?.startsWith('settings')}
	<FlowSettings />
{:else if $selectedId === 'Input'}
	<FlowInput />
{:else if $selectedId === 'Result'}
	<p class="p-4 text-secondary">Nothing to show about the result node. Happy flow building!</p>
{:else if $selectedId === 'constants'}
	<FlowConstants />
{:else if $selectedId === 'failure'}
	<FlowFailureModule />
{:else}
	{#key $selectedId}
		{#each $flowStore.value.modules as flowModule, index (flowModule.id ?? index)}
			<FlowModuleWrapper bind:flowModule previousModule={$flowStore.value.modules[index - 1]} />
		{/each}
	{/key}
{/if}
