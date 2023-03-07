<script lang="ts">
	import { getContext } from 'svelte'

	import type { FlowEditorContext } from '../types'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import FlowSettings from './FlowSettings.svelte'
	import FlowInput from './FlowInput.svelte'
	import FlowFailureModule from './FlowFailureModule.svelte'
	import FlowConstants from './FlowConstants.svelte'

	export let initialPath: string

	const { selectedId, flowStore } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

{#if $selectedId?.startsWith('settings')}
	<FlowSettings {initialPath} />
{:else if $selectedId === 'Input'}
	<FlowInput />
{:else if $selectedId === 'Result'}
	<p class="p-4 text-gray-600">Nothing to show about the result node. Happy flow building!</p>
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
