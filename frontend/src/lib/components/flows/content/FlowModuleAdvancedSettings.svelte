<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'

	import Toggle from '$lib/components/Toggle.svelte'
	import PropPickerWrapper from '$lib/components/flows/propPicker/PropPickerWrapper.svelte'

	import type { FlowModule } from '$lib/gen'
	import { getStepPropPicker } from '../flowStateUtils'
	import { flowStore } from '../flowStore'
	import { flowStateStore } from '../flowState'
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'

	const { selectedId, previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')

	export let flowModule: FlowModule

	let editor: SimpleEditor | undefined = undefined

	$: isSuspendEnabled = Boolean(flowModule.suspend)
	$: isStopAfterIfEnabled = Boolean(flowModule.stop_after_if)
	let pickableProperties: Object = {}

	$: {
		let indices = $selectedId.split('-').map(Number)
		if (indices[1]) {
			indices[1] += 1
		} else {
			indices[0] += 1
		}
		const props = getStepPropPicker(
			indices,
			$flowStore.schema,
			$flowStateStore,
			$previewArgs
		).pickableProperties

		props['result'] = props['previous_result']
		delete props['previous_result']
		props.step = props.step?.slice(0, props.step.length - 1)
		pickableProperties = props
	}
</script>

<div class="flex flex-col items-start space-y-2">
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
			right: 'Suspend flow execution until events received enabled'
		}}
	/>
	<span class="text-xs font-bold">Number of events to wait for</span>

	{#if flowModule.suspend}
		<input bind:value={flowModule.suspend} type="number" min="1" placeholder="1" />
	{:else}
		<input type="number" disabled />
	{/if}

	<Toggle
		checked={isStopAfterIfEnabled}
		on:change={() => {
			if (isStopAfterIfEnabled && flowModule.stop_after_if) {
				flowModule.stop_after_if = undefined
			} else {
				flowModule.stop_after_if = {
					expr: 'result == undefined',
					skip_if_stopped: false
				}
			}
		}}
		options={{
			right: 'Early stop if condition met enabled'
		}}
	/>

	{#if flowModule.stop_after_if}
		<span class="text-xs font-bold">Should skip if stopped</span>

		<input type="checkbox" bind:checked={flowModule.stop_after_if.skip_if_stopped} />

		<span class="text-xs font-bold">Stop condition expression</span>

		<div class="border w-full">
			<PropPickerWrapper
				{pickableProperties}
				on:select={({ detail }) => {
					editor?.insertAtCursor(detail)
				}}
			>
				<SimpleEditor
					bind:this={editor}
					lang="javascript"
					bind:code={flowModule.stop_after_if.expr}
					class="small-editor"
				/>
			</PropPickerWrapper>
		</div>
	{:else}
		<span class="text-xs font-bold">Should skip if stopped</span>
		<input type="checkbox" disabled />
		<span class="text-xs font-bold">Stop condition expression</span>
		<textarea disabled rows="3" />
	{/if}
</div>
