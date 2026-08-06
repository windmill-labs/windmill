<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import type { FlowModule } from '$lib/gen'
	import { stepSettingDefaults } from '../flowStepSettings'
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { emptySchema } from '$lib/utils'
	import { getStepPropPicker } from '../previousResults'

	const { flowStateStore, flowStore, previewArgs } =
		getContext<FlowEditorContext>('FlowEditorContext')

	interface Props {
		flowModule: FlowModule
		parentModule: FlowModule | undefined
		previousModule: FlowModule | undefined
	}

	let { flowModule = $bindable(), parentModule, previousModule }: Props = $props()

	let editor: SimpleEditor | undefined = $state(undefined)

	// A predicate is stored as a bare `{ expr }`, so the form is told its kind through
	// `argType` rather than inferring one from the value.
	let schema = $state(emptySchema())
	schema.properties['skip_if'] = { type: 'boolean' }

	let stepPropPicker = $derived(
		getStepPropPicker(
			flowStateStore.val,
			parentModule,
			previousModule,
			flowModule.id,
			flowStore.val,
			previewArgs.val,
			false
		)
	)

	// The worker evaluates skip_if before this step runs, passing the last job result,
	// so `result` here is the previous step's output — not this step's.
	let result = $derived(
		previousModule ? flowStateStore.val[previousModule.id]?.previewResult : undefined
	)

	let isSkipEnabled = $derived(Boolean(flowModule.skip_if))
</script>

{#snippet skipToggle()}
	<Toggle
		size="xs"
		textClass="text-xs font-normal text-primary"
		checked={isSkipEnabled}
		on:change={() => {
			if (isSkipEnabled && flowModule.skip_if) {
				flowModule.skip_if = undefined
			} else {
				flowModule.skip_if = stepSettingDefaults('skip')
			}
		}}
		options={{
			right: 'Skip step if',
			rightTooltip:
				"If the condition is met, the step behaves as an identity step, passing the previous step's result through unchanged."
		}}
	/>
{/snippet}

<div class="w-full">
	<PropPickerWrapper
		sidePane
		flow_input={stepPropPicker.pickableProperties.flow_input}
		notSelectable
		{result}
		pickableProperties={stepPropPicker.pickableProperties}
		on:select={({ detail }) => {
			editor?.insertAtCursor(detail)
			editor?.focus()
		}}
	>
		<InputTransformForm
			bind:arg={
				() => flowModule.skip_if,
				(v) => {
					flowModule.skip_if = v
				}
			}
			argName="skip_if"
			argType="javascript"
			collapsed={!isSkipEnabled}
			animateAppear
			header={skipToggle}
			noDynamicToggle
			{schema}
			previousModuleId={previousModule?.id}
			pickableProperties={stepPropPicker.pickableProperties}
			extraLib={`declare const result = ${JSON.stringify(result)};\n` + stepPropPicker.extraLib}
			bind:editor
		/>
	</PropPickerWrapper>
</div>
