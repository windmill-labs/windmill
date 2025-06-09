<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import type { FlowModule } from '$lib/gen'

	export let noEditor = false
	export let savedModule: FlowModule | undefined = undefined

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

{#if flowStore.val.value.failure_module}
	<FlowModuleWrapper
		{noEditor}
		bind:flowModule={flowStore.val.value.failure_module}
		{savedModule}
		previousModule={undefined}
	/>
{/if}
