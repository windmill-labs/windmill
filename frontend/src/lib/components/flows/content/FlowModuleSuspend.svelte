<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import type { FlowModule } from '$lib/gen'
	import { SecondsInput } from '../../common'

	export let flowModule: FlowModule

	$: isSuspendEnabled = Boolean(flowModule.suspend)
</script>

<h2>
	Suspend
	<Tooltip documentationLink="https://docs.windmill.dev/docs/flows/flow_approval">
		If defined, at the end of the step, the flow will be suspended until it receives external
		requests to be resumed or canceled. This is most useful to implement approval steps but can be
		used flexibly for other purpose.
	</Tooltip>
</h2>
<Toggle
	checked={isSuspendEnabled}
	on:change={() => {
		if (isSuspendEnabled && flowModule.suspend != undefined) {
			flowModule.suspend = undefined
		} else {
			flowModule.suspend = {
				required_events: 1,
				timeout: 1800
			}
		}
	}}
	options={{
		right: 'Suspend flow execution until events/approvals received'
	}}
/>
<div class="mb-4">
	<span class="text-xs font-bold">Number of approvals/events required for resuming flow</span>

	{#if flowModule.suspend}
		<input bind:value={flowModule.suspend.required_events} type="number" min="1" placeholder="1" />
	{:else}
		<input type="number" disabled />
	{/if}

	<span class="text-xs font-bold">Timeout</span>

	{#if flowModule.suspend}
		<SecondsInput bind:seconds={flowModule.suspend.timeout} />
	{:else}
		<SecondsInput disabled />
	{/if}
</div>
