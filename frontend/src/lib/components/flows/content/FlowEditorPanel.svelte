<script lang="ts">
	import { getContext } from 'svelte'

	import type { FlowEditorContext } from '../types'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import FlowSettings from './FlowSettings.svelte'
	import FlowInput from './FlowInput.svelte'
	import FlowLoop from './FlowLoop.svelte'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

{#if $selectedId === 'settings'}
	<FlowSettings />
{:else if $selectedId === 'settings-schedule'}
	<FlowSettings defaultTab="schedule" />
{:else if $selectedId === 'settings-retries'}
	<FlowSettings defaultTab="retries" />
{:else if $selectedId.includes('loop')}
	<FlowLoop />
{:else if $selectedId === 'inputs'}
	<FlowInput />
{:else}
	<FlowModuleWrapper />
{/if}
