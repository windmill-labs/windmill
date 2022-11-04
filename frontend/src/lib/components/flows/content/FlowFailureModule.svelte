<script lang="ts">
	import { flowStore } from '../flowStore'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
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
