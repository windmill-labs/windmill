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

	function checkDup(modules: FlowModule[]): string | undefined {
		let seenModules: string[] = []
		for (const m of modules) {
			if (seenModules.includes(m.id)) {
				console.error(`Duplicate module id: ${m.id}`)
				return m.id
			}
			seenModules.push(m.id)
		}
	}
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
	{@const dup = checkDup($flowStore.value.modules)}
	{#if dup}
		<div class="text-red-600 text-xl p-2">There are duplicate modules in the flow at id: {dup}</div>
	{:else}
		{#key $selectedId}
			{#each $flowStore.value.modules as flowModule, index (flowModule.id ?? index)}
				<FlowModuleWrapper bind:flowModule previousModule={$flowStore.value.modules[index - 1]} />
			{/each}
		{/key}
	{/if}
{/if}
