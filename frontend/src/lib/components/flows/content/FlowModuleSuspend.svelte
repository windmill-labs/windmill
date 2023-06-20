<script lang="ts">
	import SchemaEditor from '$lib/components/SchemaEditor.svelte'
	import Slider from '$lib/components/Slider.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import type { FlowModule } from '$lib/gen'
	import { emptySchema } from '$lib/utils'
	import { SecondsInput } from '../../common'

	export let flowModule: FlowModule

	$: isSuspendEnabled = Boolean(flowModule.suspend)
</script>

<h2 class="pb-4">
	Suspend/Approval
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
	{#if flowModule.suspend}
		<div class="mt-4" />
		<div class="flex gap-4">
			<Toggle
				checked={Boolean(flowModule.suspend.resume_form)}
				options={{
					right: 'Add a form to the approval page'
				}}
				on:change={(e) => {
					if (flowModule.suspend) {
						if (e.detail) {
							flowModule.suspend.resume_form = {
								schema: emptySchema()
							}
						} else {
							flowModule.suspend.resume_form = undefined
						}
					}
				}}
			/>
			<div>
				<Slider size="xs" text="How to add dynamic default args">
					As one of the return key of this step, return an object `default_args` that contains the
					default arguments of the form arguments. e.g:
					<pre
						><code
							>{`return {
	endpoints,
	default_args: {
		foo: "foo",
		bar: true,
	},
}`}</code
						></pre
					>
				</Slider></div
			></div
		>

		<div class="mt-2" />
	{/if}
	{#if flowModule.suspend?.resume_form}
		<SchemaEditor bind:schema={flowModule.suspend.resume_form.schema} />
	{/if}
</div>
