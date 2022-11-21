<script lang="ts">
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import type { FlowModule } from '$lib/gen'
	import { emptySchema } from '$lib/utils'
	import { getContext } from 'svelte'
	import { flowStateStore } from '../flowState'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import type { FlowEditorContext } from '../types'

	export let flowModule: FlowModule
	export let previousModuleId: string | undefined

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	let schema = emptySchema()
	schema.properties['sleep'] = {
		type: 'number',
		description: 'Sleep time in seconds'
	}

	let editor: SimpleEditor | undefined = undefined

	const result = $flowStateStore[$selectedId]?.previewResult ?? {}

	$: isSuspendEnabled = Boolean(flowModule.suspend)
	$: isSleepEnabled = Boolean(flowModule.sleep)
</script>

<h2 class="mt-2"
	>Suspend<Tooltip>
		If defined, at the end of the step, the flow will be suspended until it receives external
		requests to be resumed or canceled. This is most useful to implement approval steps but can be
		used flexibly for other purpose. To get the resume urls, use `wmill.getResumeEndpoints`.</Tooltip
	></h2
>
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
		right: 'Suspend flow execution until approvals received'
	}}
/>
<div class="mb-4">
	<span class="text-xs font-bold">Number of approvals required for resuming flow</span>

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

<h2 class="mt-4"
	>Sleep<Tooltip>
		If defined, at the end of the step, the flow will sleep for a number of seconds before being
		resumed. Sleeping is passive and does not consume any resources.</Tooltip
	></h2
>
<Toggle
	checked={isSleepEnabled}
	on:change={() => {
		if (isSleepEnabled && flowModule.sleep != undefined) {
			flowModule.sleep = undefined
		} else {
			flowModule.sleep = {
				type: 'static',
				value: 0
			}
		}
	}}
	options={{
		right: 'Sleep after module successful execution'
	}}
/>
<div>
	<span class="text-xs font-bold">Sleep for duration (seconds)</span>

	{#if flowModule.sleep && schema.properties['sleep']}
		<div class="border">
			<PropPickerWrapper
				notSelectable
				{result}
				displayContext={false}
				pickableProperties={undefined}
				on:select={({ detail }) => {
					editor?.insertAtCursor(detail)
				}}
			>
				<InputTransformForm
					bind:arg={flowModule.sleep}
					argName="sleep"
					{schema}
					{previousModuleId}
				/>
			</PropPickerWrapper>
		</div>
	{:else}
		<input type="number" disabled />
	{/if}
</div>
