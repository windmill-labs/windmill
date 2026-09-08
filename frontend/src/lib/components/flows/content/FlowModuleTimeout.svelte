<script lang="ts">
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	import type { FlowModule } from '$lib/gen'
	import { Alert } from '../../common'
	import { emptySchema } from '$lib/utils'
	import { getContext } from 'svelte'
	import PropPickerWrapper from '$lib/components/flows/propPicker/PropPickerWrapper.svelte'
	import type { FlowEditorContext } from '../types'
	import { getStepPropPicker } from '../previousResults'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import { slideDynamic } from '$lib/transitions'

	interface Props {
		flowModule: FlowModule
		previousModuleId: string | undefined
	}

	let { flowModule = $bindable(), previousModuleId }: Props = $props()

	const { flowStore, flowStateStore, previewArgs } =
		getContext<FlowEditorContext>('FlowEditorContext')

	const customUi = getContext<FlowBuilderWhitelabelCustomUi | undefined>('customUi')

	let schema = $state(emptySchema())
	schema.properties['timeout'] = {
		type: 'number'
	}

	if (typeof flowModule.timeout === 'number') {
		flowModule.timeout = {
			type: 'static',
			value: flowModule.timeout
		}
	}

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

	let editor: SimpleEditor | undefined = $state(undefined)

	let istimeoutEnabled = $derived(Boolean(flowModule.timeout))
</script>

<div class="flex flex-col gap-2">
	<Toggle
		size="xs"
		textClass="text-xs font-normal text-primary"
		checked={istimeoutEnabled}
		on:change={() => {
			if (istimeoutEnabled && flowModule.timeout != undefined) {
				flowModule.timeout = undefined
			} else {
				flowModule.timeout = {
					type: 'static',
					value: customUi?.defaultTimeout ?? 300
				}
			}
		}}
		options={{
			right: 'Custom timeout',
			rightTooltip:
				"The custom timeout is used instead of the instance timeout for the step. The step's timeout cannot be greater than the instance timeout."
		}}
	/>
	{#if flowModule.timeout && schema.properties['timeout']}
		<div class="pl-9" transition:slideDynamic>
			<PropPickerWrapper
				sidePane
				flow_input={stepPropPicker.pickableProperties.flow_input}
				notSelectable
				pickableProperties={stepPropPicker.pickableProperties}
				on:select={({ detail }) => {
					editor?.insertAtCursor(detail)
					editor?.focus()
				}}
			>
				<InputTransformForm
					bind:arg={flowModule.timeout}
					argName="timeout"
					{schema}
					{previousModuleId}
					argExtra={{ seconds: true }}
					bind:editor
				/>
			</PropPickerWrapper>
		</div>
	{/if}

	{#if flowModule.timeout && flowModule.timeout.type !== 'static'}
		<div class="mt-4 pl-9" transition:slideDynamic>
			<Alert title="Dynamic timeout only used when testing the full flow" type="info">
				<p class="text-xs">
					A dynamic timeout expression is evaluated when running the full flow. It is ignored when
					running "Test this step" — only a static timeout value applies there.
				</p>
			</Alert>
		</div>
	{/if}
</div>
