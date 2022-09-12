<script lang="ts">
	import { getContext } from 'svelte'

	import type { FlowEditorContext } from '../types'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import FlowSchedules from './FlowSchedules.svelte'
	import FlowSettings from './FlowSettings.svelte'
	import FlowInput from './FlowInput.svelte'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

{#if $selectedId === 'settings'}
	<FlowSettings />
{:else if $selectedId === 'schedules'}
	<FlowSchedules />
{:else if $selectedId === 'inputs'}
	<FlowInput />
{:else}
	<FlowModuleWrapper />
{/if}
