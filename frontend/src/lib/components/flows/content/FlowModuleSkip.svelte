<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import FlowExpressionEditor from './FlowExpressionEditor.svelte'
	import type { FlowModule } from '$lib/gen'
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { slide } from 'svelte/transition'
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

	let result = $derived(flowStateStore.val[flowModule.id]?.previewResult)

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
				flowModule.skip_if = {
					expr: 'false'
				}
			}
		}}
		options={{
			right: 'Skip step if condition is met',
			rightTooltip:
				"If the condition is met, the step behaves as an identity step, passing the previous step's result through unchanged."
		}}
	/>

	{#if flowModule.skip_if}
		<div class="w-full" transition:slide={{ duration: 120 }}>
			<FlowExpressionEditor
				label="Skip condition expression"
				bind:code={
					() => flowModule.skip_if?.expr ?? '',
					(v) => {
						if (flowModule.skip_if) flowModule.skip_if.expr = v
					}
				}
				pickableProperties={stepPropPicker.pickableProperties}
				{result}
				extraLib={stepPropPicker.extraLib}
			/>
		</div>
	{/if}
</div>
