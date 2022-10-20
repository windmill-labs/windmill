<script lang="ts">
	import { flowStore } from '../flowStore'

	import FlowModule from './FlowModuleComponent.svelte'
</script>

{#if $flowStore.value.failure_module}
	<FlowModule
		bind:flowModule={$flowStore.value.failure_module}
		on:delete={() => {
			$flowStore.value.failure_module = undefined
		}}
		failureModule={true}
	/>
{/if}
