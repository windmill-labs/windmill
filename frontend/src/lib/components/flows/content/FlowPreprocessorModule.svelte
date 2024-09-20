<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'

	export let noEditor = false

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

{#if $flowStore.value.preprocessor_module}
	<FlowModuleWrapper
		{noEditor}
		bind:flowModule={$flowStore.value.preprocessor_module}
		on:delete={() => {
			$flowStore.value.preprocessor_module = undefined
		}}
		previousModule={undefined}
	/>
{/if}
