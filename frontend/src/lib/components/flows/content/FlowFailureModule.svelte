<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

{#if $flowStore.value.failure_module}
	<FlowModuleWrapper
		bind:flowModule={$flowStore.value.failure_module}
		on:delete={() => {
			$flowStore.value.failure_module = undefined
		}}
		previousModule={undefined}
	/>
{/if}
