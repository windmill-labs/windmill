<script lang="ts">
	import { getContext } from 'svelte'

	import type { FlowEditorContext } from '../types'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import FlowSettings from './FlowSettings.svelte'
	import FlowInput from './FlowInput.svelte'
	import FlowFailureModule from './FlowFailureModule.svelte'
	import FlowConstants from './FlowConstants.svelte'
	import type { FlowModule } from '$lib/gen'

	export let noEditor = false
	export let enableAi = false

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
	<FlowSettings {noEditor} />
{:else if $selectedId === 'Input'}
	<FlowInput {noEditor} />
{:else if $selectedId === 'Result'}
	<p class="p-4 text-secondary">Nothing to show about the result node. Happy flow building!</p>
{:else if $selectedId === 'constants'}
	<FlowConstants {noEditor} />
{:else if $selectedId === 'failure'}
	<FlowFailureModule {noEditor} />
{:else}
	{@const dup = checkDup($flowStore.value.modules)}
	{#if dup}
		<div class="text-red-600 text-xl p-2">There are duplicate modules in the flow at id: {dup}</div>
	{:else}
		{#key $selectedId}
			{#each $flowStore.value.modules as flowModule, index (flowModule.id ?? index)}
				<FlowModuleWrapper
					{noEditor}
					bind:flowModule
					previousModule={$flowStore.value.modules[index - 1]}
					{enableAi}
				/>
			{/each}
		{/key}
	{/if}
{/if}
