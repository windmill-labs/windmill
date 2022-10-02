<script lang="ts">
	import { getContext } from 'svelte'

	import type { FlowEditorContext } from '../types'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import FlowSettings from './FlowSettings.svelte'
	import FlowInput from './FlowInput.svelte'
	import FlowLoop from './FlowLoop.svelte'
	import FlowFailureModule from './FlowFailureModule.svelte'

	export let previewArgs: Record<string, any> = {}

	export let initialPath: string

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

{#key $selectedId}
	{#if $selectedId === 'settings'}
		<FlowSettings {initialPath} />
	{:else if $selectedId === 'settings-schedule'}
		<FlowSettings {initialPath} defaultTab="schedule" />
	{:else if $selectedId === 'settings-retries'}
		<FlowSettings {initialPath} defaultTab="retries" />
	{:else if $selectedId.includes('loop')}
		<FlowLoop {previewArgs} />
	{:else if $selectedId === 'inputs'}
		<FlowInput />
	{:else if $selectedId === 'failure'}
		<FlowFailureModule />
	{:else}
		<FlowModuleWrapper />
	{/if}
{/key}
