<script lang="ts">
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import type SimpleEditor from "$lib/components/SimpleEditor.svelte";
	import Section from '$lib/components/Section.svelte'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import type { FlowModule } from '$lib/gen'
	import { Alert, SecondsInput } from '../../common'
	import {getContext} from "svelte";
	import {emptySchema} from "$lib/utils";
	import type {FlowEditorContext} from "$lib/components/flows/types";

	interface Props {
		flowModule: FlowModule
		previousModuleId: string | undefined
	}

	let { flowModule = $bindable(), previousModuleId }: Props = $props()

	const { selectedId, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')
	let schema = $state(emptySchema())
	schema.properties['timeout'] = {
		type: 'number'
	}

	let editor: SimpleEditor | undefined = $state(undefined)

	const result = $flowStateStore[$selectedId]?.previewResult ?? {}

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
	<div class="mb-4">
		<span class="text-xs font-bold">Timeout duration</span>

		{#if flowModule.timeout && schema.properties['timeout']}
			<div class="border">
				<PropPickerWrapper
						noFlowPlugConnect={true}
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
			<div class="text-secondary">OR use a dynamic expression</div>
		{/if}

		<div class="mt-4"></div>

		<Alert title="Only used when testing the full flow" type="info">
			<p class="text-sm"> The timeout will be ignored when running "Test this step" </p>
		</Alert>
	</div>
</Section>
