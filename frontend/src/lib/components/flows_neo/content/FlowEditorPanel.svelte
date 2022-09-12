<script lang="ts">
	import { getContext } from 'svelte'

	import type { FlowEditorContext } from '../types'
	import FlowFailuresRetries from './FlowFailuresRetries.svelte'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import FlowSchedules from './FlowSchedules.svelte'
	import FlowSettings from './FlowSettings.svelte'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

{#if $selectedId === 'settings'}
	<FlowSettings />
{:else if $selectedId === 'schedules'}
	<FlowSchedules />
{:else if $selectedId === 'failures_retries'}
	<FlowFailuresRetries />
{:else}
	<FlowModuleWrapper />
{/if}
