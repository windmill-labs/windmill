<script lang="ts">
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	import type { FlowModule } from '$lib/gen'
	import { emptySchema } from '$lib/utils'
	import { getContext } from 'svelte'
	import { flowStateStore } from '../flowState'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import type { FlowEditorContext } from '../types'
	import { selectedIdToModuleState } from '../utils'

	export let flowModule: FlowModule

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	let schema = emptySchema()
	schema.properties['sleep'] = {
		type: 'number',
		description: 'Sleep time in seconds'
	}

	let editor: SimpleEditor | undefined = undefined

	let pickableProperties = {
		result: selectedIdToModuleState($selectedId, $flowStateStore).previewResult
	}

	$: isSuspendEnabled = Boolean(flowModule.suspend)
	$: isSleepEnabled = Boolean(flowModule.sleep)
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
<div class="mb-4">
	<span class="text-xs font-bold">Number of events to wait for</span>

	{#if flowModule.suspend}
		<input bind:value={flowModule.suspend} type="number" min="1" placeholder="1" />
	{:else}
		<input type="number" disabled />
	{/if}
</div>

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
		<PropPickerWrapper
			displayContext={false}
			{pickableProperties}
			on:select={({ detail }) => {
				editor?.insertAtCursor(detail)
			}}
		>
			<InputTransformForm bind:arg={flowModule.sleep} argName="sleep" {schema} />
		</PropPickerWrapper>
	{:else}
		<input type="number" disabled />
	{/if}
</div>
