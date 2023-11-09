<script lang="ts">
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { FlowModule } from '$lib/gen'
	import { emptySchema } from '$lib/utils'
	import { getContext } from 'svelte'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import type { FlowEditorContext } from '../types'
	import { SecondsInput } from '../../common'
	import Section from '$lib/components/Section.svelte'
	import Label from '$lib/components/Label.svelte'

	export let flowModule: FlowModule
	export let previousModuleId: string | undefined

	const { selectedId, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let schema = emptySchema()
	schema.properties['sleep'] = {
		type: 'number'
	}

	let editor: SimpleEditor | undefined = undefined

	const result = $flowStateStore[$selectedId]?.previewResult ?? {}

	$: isSleepEnabled = Boolean(flowModule.sleep)
</script>

<Section label="Sleep" class="w-full">
	<svelte:fragment slot="header">
		<Tooltip documentationLink="https://www.windmill.dev/docs/flows/sleep">
			If defined, at the end of the step, the flow will sleep for a number of seconds before
			scheduling the next job (if any, no effect if the step is the last one).
		</Tooltip>
	</svelte:fragment>

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
	<Label label="Sleep for duration">
		{#if flowModule.sleep && schema.properties['sleep']}
			<div class="border">
				<PropPickerWrapper
					notSelectable
					{result}
					displayContext={false}
					pickableProperties={undefined}
					on:select={({ detail }) => {
						editor?.insertAtCursor(detail)
						editor?.focus()
					}}
				>
					<InputTransformForm
						bind:arg={flowModule.sleep}
						argName="sleep"
						{schema}
						{previousModuleId}
						argExtra={{ seconds: true }}
					/>
				</PropPickerWrapper>
			</div>
		{:else}
			<SecondsInput disabled />
		{/if}
	</Label>
</Section>
