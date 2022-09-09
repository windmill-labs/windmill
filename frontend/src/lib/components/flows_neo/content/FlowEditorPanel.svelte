<script lang="ts">
	import { getContext } from 'svelte'

	import type { FlowEditorContext } from '../types'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import FlowSettings from './FlowSettings.svelte'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

{#if $selectedId === 'settings'}
	<FlowSettings />
{:else}
	<FlowModuleWrapper />
{/if}
