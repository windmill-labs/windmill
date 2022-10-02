<script lang="ts">
	import { flowStateStore } from '../flowState'
	import { flowStore } from '../flowStore'

	import FlowModule from './FlowModule.svelte'
</script>

{#if $flowStore.value.failure_module}
	<FlowModule
		bind:flowModule={$flowStore.value.failure_module}
		bind:flowModuleState={$flowStateStore.failureModule}
		on:delete={() => {
			$flowStore.value.failure_module = undefined
		}}
		failureModule={true}
	/>
{/if}
