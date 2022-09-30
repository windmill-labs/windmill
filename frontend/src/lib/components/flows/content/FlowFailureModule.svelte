<script lang="ts">
	import { getContext } from 'svelte'

	import { flowStateStore } from '../flowState'
	import { flowStore } from '../flowStore'
	import type { FlowEditorContext } from '../types'

	import FlowModule from './FlowModule.svelte'

	const { previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

{#if $flowStore.value.failure_module}
	<FlowModule
		{previewArgs}
		bind:flowModule={$flowStore.value.failure_module}
		bind:flowModuleState={$flowStateStore.failureModule}
		on:delete={() => {
			$flowStore.value.failure_module = undefined
		}}
		failureModule={true}
	/>
{/if}
