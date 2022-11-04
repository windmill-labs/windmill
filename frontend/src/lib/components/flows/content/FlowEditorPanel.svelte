<script lang="ts">
	import { getContext } from 'svelte'

	import type { FlowEditorContext } from '../types'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import FlowSettings from './FlowSettings.svelte'
	import FlowInput from './FlowInput.svelte'
	import FlowFailureModule from './FlowFailureModule.svelte'
	import { flowStore } from '../flowStore'

	export let initialPath: string

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

{#key $selectedId}
	{#if $selectedId === 'settings'}
		<FlowSettings {initialPath} />
	{:else if $selectedId === 'inputs'}
		<FlowInput />
	{:else if $selectedId === 'settings-schedule'}
		<FlowSettings {initialPath} defaultTab="schedule" />
	{:else if $selectedId === 'settings-same-worker'}
		<FlowSettings {initialPath} defaultTab="same-worker" />
	{:else if $selectedId === 'failure'}
		<FlowFailureModule />
	{:else}
		{#each $flowStore.value.modules as flowModule, index (flowModule.id ?? index)}
			<FlowModuleWrapper bind:flowModule previousModule={$flowStore.value.modules[index - 1]} />
		{/each}
	{/if}
{/key}
