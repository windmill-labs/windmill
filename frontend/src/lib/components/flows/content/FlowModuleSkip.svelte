<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import PropPickerWrapper from '$lib/components/flows/propPicker/PropPickerWrapper.svelte'
	import type { FlowModule } from '$lib/gen'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import { getStepPropPicker } from '../previousResults'

	const { flowStateStore, flowStore, previewArgs } =
		getContext<FlowEditorContext>('FlowEditorContext')

	export let flowModule: FlowModule
	export let parentModule: FlowModule | undefined
	export let previousModule: FlowModule | undefined

	let editor: SimpleEditor | undefined = undefined
	$: stepPropPicker = getStepPropPicker(
		$flowStateStore,
		parentModule,
		previousModule,
		flowModule.id,
		flowStore,
		$previewArgs,
		false
	)

	$: isSkipEnabled = Boolean(flowModule.skip_if)
</script>

<div class="flex flex-col items-start space-y-2 {$$props.class}">
	<Section label="Skip" class="w-full">
		<svelte:fragment slot="header">
			<Tooltip>
				If the condition is met, the step will behave as an identity step, passing the previous
				step's result through unchanged.
			</Tooltip>
		</svelte:fragment>

		<Toggle
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
				right: 'Skip step if condition is met'
			}}
		/>

		<div
			class="w-full border p-2 mt-2 flex flex-col {flowModule.skip_if
				? ''
				: 'bg-surface-secondary'}"
		>
			{#if flowModule.skip_if}
				<span class="mt-2 text-xs font-bold">Skip condition expression</span>
				<div class="border w-full">
					<PropPickerWrapper
						notSelectable
						pickableProperties={stepPropPicker.pickableProperties}
						on:select={({ detail }) => {
							editor?.insertAtCursor(detail)
							editor?.focus()
						}}
					>
						<SimpleEditor
							bind:this={editor}
							lang="javascript"
							bind:code={flowModule.skip_if.expr}
							class="few-lines-editor"
							extraLib={stepPropPicker.extraLib}
						/>
					</PropPickerWrapper>
				</div>
			{:else}
				<span class="mt-2 text-xs font-bold">Skip condition expression</span>
				<textarea disabled rows="3" class="min-h-[80px]"></textarea>
			{/if}
		</div>
	</Section>
</div>
