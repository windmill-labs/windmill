<script lang="ts">
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Section from '$lib/components/Section.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Label from '$lib/components/Label.svelte'

	import type { FlowModule } from '$lib/gen'
	import { Alert, SecondsInput } from '../../common'
	import { emptySchema } from '$lib/utils'
	import { getContext } from 'svelte'
	import PropPickerWrapper from '$lib/components/flows/propPicker/PropPickerWrapper.svelte'
	import type { FlowEditorContext } from '../types'
	import { getStepPropPicker } from '../previousResults'

	interface Props {
		flowModule: FlowModule
		previousModuleId: string | undefined
	}

	let { flowModule = $bindable(), previousModuleId }: Props = $props()

	const { flowStore, flowStateStore, previewArgs } =
		getContext<FlowEditorContext>('FlowEditorContext')

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

<Section label="Timeout">
	{#snippet header()}
		<Tooltip>
			If defined, the custom timeout will be used instead of the instance timeout for the step. The
			step's timeout cannot be greater than the instance timeout.
		</Tooltip>
	{/snippet}

	<Toggle
		checked={istimeoutEnabled}
		on:change={() => {
			if (istimeoutEnabled && flowModule.timeout != undefined) {
				flowModule.timeout = undefined
			} else {
				flowModule.timeout = {
					type: 'static',
					value: 300
				}
			}
		}}
		options={{
			right: 'Add a custom timeout for this step'
		}}
	/>
	<Label label="Timeout duration" class="mt-2">
		{#if flowModule.timeout && schema.properties['timeout']}
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

	<div class="mt-4">
		<Alert title="Only used when testing the full flow" type="info">
			<p class="text-xs"> The timeout will be ignored when running "Test this step" </p>
		</Alert>
	</div>
</Section>
