<script lang="ts">
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Label from '$lib/components/Label.svelte'

	import type { FlowModule } from '$lib/gen'
	import { Alert, SecondsInput } from '../../common'
	import { emptySchema } from '$lib/utils'
	import { getContext } from 'svelte'
	import PropPickerWrapper from '$lib/components/flows/propPicker/PropPickerWrapper.svelte'
	import type { FlowEditorContext } from '../types'
	import { getStepPropPicker } from '../previousResults'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'

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
			right: 'Add a custom timeout for this step',
			rightTooltip:
				"The custom timeout is used instead of the instance timeout for the step. The step's timeout cannot be greater than the instance timeout."
		}}
	/>
	{#if flowModule.timeout}
		<Label label="Timeout duration" class="mt-2">
			{#if schema.properties['timeout']}
				<div class="border">
					<PropPickerWrapper
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
			{:else}
				<SecondsInput disabled />
				<div class="text-secondary text-sm">OR use a dynamic expression</div>
			{/if}
		</Label>

		{#if flowModule.timeout.type !== 'static'}
			<div class="mt-4">
				<Alert title="Dynamic timeout only used when testing the full flow" type="info">
					<p class="text-xs">
						A dynamic timeout expression is evaluated when running the full flow. It is ignored when
						running "Test this step" — only a static timeout value applies there.
					</p>
				</Alert>
			</div>
		{/if}
	{/if}
</div>
