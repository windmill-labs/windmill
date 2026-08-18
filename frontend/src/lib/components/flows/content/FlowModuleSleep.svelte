<script lang="ts">
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import type { FlowModule } from '$lib/gen'
	import { stepSettingDefaults } from '../flowStepSettings'
	import { emptySchema } from '$lib/utils'
	import { getContext } from 'svelte'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import type { FlowEditorContext } from '../types'
	import { getStepPropPicker } from '../previousResults'
	import { slideDynamic } from '$lib/transitions'
	import { SAME_WORKER_INCOMPATIBLE_MSG } from '../utils.svelte'
	import { Alert } from '$lib/components/common'

	interface Props {
		flowModule: FlowModule
		previousModuleId: string | undefined
		isAgentTool?: boolean
	}

	let { flowModule = $bindable(), previousModuleId, isAgentTool = false }: Props = $props()

	const { selectionManager, flowStore, flowStateStore, previewArgs } =
		getContext<FlowEditorContext>('FlowEditorContext')
	let schema = $state(emptySchema())
	schema.properties['sleep'] = {
		type: 'number'
	}

	let editor: SimpleEditor | undefined = $state(undefined)

	let stepPropPicker = $derived(
		getStepPropPicker(
			flowStateStore.val,
			undefined,
			undefined,
			flowModule.id,
			flowStore.val,
			previewArgs.val,
			false
		)
	)

	const result = flowStateStore.val[selectionManager.getSelectedId()]?.previewResult ?? {}

	let isSleepEnabled = $derived(Boolean(flowModule.sleep))
	// Agent tools never go through the flow scheduler, so `same_worker` doesn't apply to them.
	let sameWorker = $derived(Boolean(!isAgentTool && flowStore.val.value.same_worker))
</script>

<div class="flex flex-col gap-2">
	{#if sameWorker}
		<Alert type="warning" size="xs" title="Disabled by the shared directory">
			{SAME_WORKER_INCOMPATIBLE_MSG} Disable `Same Worker` in the flow settings to use a sleep.
		</Alert>
	{/if}
	<Toggle
		size="xs"
		textClass="text-xs font-normal text-primary"
		checked={isSleepEnabled}
		disabled={sameWorker}
		on:change={() => {
			if (isSleepEnabled && flowModule.sleep != undefined) {
				flowModule.sleep = undefined
			} else {
				flowModule.sleep = stepSettingDefaults('sleep')
			}
		}}
		options={{
			right: 'Sleep after step',
			rightTooltip:
				'At the end of the step, the flow sleeps for a number of seconds before scheduling the next job (no effect if the step is the last one).',
			rightDocumentationLink: 'https://www.windmill.dev/docs/flows/sleep'
		}}
	/>
	{#if flowModule.sleep && schema.properties['sleep'] && !sameWorker}
		<div class="pl-9" transition:slideDynamic>
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
					bind:arg={flowModule.sleep}
					argName="sleep"
					{schema}
					{previousModuleId}
					argExtra={{ seconds: true, clearable: false }}
					bind:editor
				/>
			</PropPickerWrapper>
		</div>
	{/if}
</div>
