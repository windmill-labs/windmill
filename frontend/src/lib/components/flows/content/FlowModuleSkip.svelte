<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import FlowExpressionEditor from './FlowExpressionEditor.svelte'
	import type { FlowModule } from '$lib/gen'
	import { stepSettingDefaults } from '../flowStepSettings'
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { getStepPropPicker } from '../previousResults'

	const { flowStateStore, flowStore, previewArgs } =
		getContext<FlowEditorContext>('FlowEditorContext')

	interface Props {
		flowModule: FlowModule
		parentModule: FlowModule | undefined
		previousModule: FlowModule | undefined
	}

	let { flowModule = $bindable(), parentModule, previousModule }: Props = $props()

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

<div class="flex w-full flex-col items-start gap-2">
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
			right: 'Skip step if condition is met',
			rightTooltip:
				"If the condition is met, the step behaves as an identity step, passing the previous step's result through unchanged."
		}}
	/>

	<div class="w-full">
		<FlowExpressionEditor
			disabled={!isSkipEnabled}
			label="Skip condition expression"
			bind:code={
				() => flowModule.skip_if?.expr ?? '',
				(v) => {
					if (flowModule.skip_if) flowModule.skip_if.expr = v
				}
			}
			pickableProperties={stepPropPicker.pickableProperties}
			{result}
			extraLib={`declare const result = ${JSON.stringify(result)};\n` + stepPropPicker.extraLib}
		/>
	</div>
</div>
