<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'

	import type { FlowModule } from '$lib/gen'

	export let flowModule: FlowModule

	$: isSuspendEnabled = Boolean(flowModule.suspend)
</script>

<Toggle
	checked={isSuspendEnabled}
	on:change={() => {
		if (isSuspendEnabled && flowModule.suspend != undefined) {
			flowModule.suspend = undefined
		} else {
			flowModule.suspend = 1
		}
	}}
	options={{
		right: 'Suspend flow execution until events received'
	}}
/>
<div>
	<span class="text-xs font-bold">Number of events to wait for</span>

	{#if flowModule.suspend}
		<input bind:value={flowModule.suspend} type="number" min="1" placeholder="1" />
	{:else}
		<input type="number" disabled />
	{/if}
</div>
