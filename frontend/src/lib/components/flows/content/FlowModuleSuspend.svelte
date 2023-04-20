<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import type { FlowModule } from '$lib/gen'
	import { emptySchema } from '$lib/utils'

	export let flowModule: FlowModule

	let schema = emptySchema()
	schema.properties['sleep'] = {
		type: 'number',
		description: 'Sleep time in seconds'
	}

	$: isSuspendEnabled = Boolean(flowModule.suspend)
</script>

<h2>
	Suspend
	<Tooltip>
		If defined, at the end of the step, the flow will be suspended until it receives external
		requests to be resumed or canceled. This is most useful to implement approval steps but can be
		used flexibly for other purpose. To get the resume urls, use `wmill.getResumeUrls()` in
		Typescript, or `wmill.get_resume_urls()` in Python.
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

	<span class="text-xs font-bold">Timeout (in seconds)</span>

	{#if flowModule.suspend}
		<input bind:value={flowModule.suspend.timeout} type="number" min="1" placeholder="1800" />
	{:else}
		<input type="number" disabled />
	{/if}
</div>
