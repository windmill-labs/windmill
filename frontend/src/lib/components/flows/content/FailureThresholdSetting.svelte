<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowModule, ForloopFlow, WhileloopFlow } from '$lib/gen'
	import type { FlowEditorContext } from '../types'
	import Toggle from '$lib/components/Toggle.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import { getStepPropPicker } from '../previousResults'
	import { emptySchema } from '$lib/utils'

	interface Props {
		mod: FlowModule
		parentModule: FlowModule | undefined
		previousModule: FlowModule | undefined
		tooltip: string
		documentationLink: string
	}

	let {
		mod = $bindable(),
		parentModule,
		previousModule,
		tooltip,
		documentationLink
	}: Props = $props()

	const { previewArgs, flowStateStore, flowStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	const DEFAULT_THRESHOLD = 5

	let editor: SimpleEditor | undefined = $state(undefined)
	let schema = $state(emptySchema())
	schema.properties['failure_threshold'] = { type: 'integer' }

	const loop = () => mod.value as ForloopFlow | WhileloopFlow

	// The worker also takes a bare number, which the form cannot edit.
	if (typeof loop().failure_threshold === 'number') {
		loop().failure_threshold = { type: 'static', value: loop().failure_threshold }
	}

	let stepPropPicker = $derived(
		getStepPropPicker(
			flowStateStore.val,
			parentModule,
			previousModule,
			mod.id,
			flowStore.val,
			previewArgs.val,
			false
		)
	)

	const enabled = $derived(loop().failure_threshold != undefined)
	// The form seeds its static/expression mode once, so a threshold that appears needs a fresh
	// form; one that is removed must not remount it, or the header shows twice while it slides out.
	let appearances = 0
	let wasEnabled = false
	const formKey = $derived.by(() => {
		if (enabled && !wasEnabled) appearances++
		wasEnabled = enabled
		return appearances
	})
</script>

{#snippet header()}
	<Toggle
		size="xs"
		textClass="text-xs font-normal text-primary"
		checked={enabled}
		on:change={({ detail }) => {
			loop().failure_threshold = detail ? { type: 'static', value: DEFAULT_THRESHOLD } : undefined
		}}
		options={{
			right: 'Limit failed iterations',
			rightTooltip: tooltip,
			rightDocumentationLink: documentationLink
		}}
	/>
{/snippet}

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
	{#key formKey}
		<InputTransformForm
			bind:arg={() => loop().failure_threshold, (v) => (loop().failure_threshold = v)}
			argName="failure_threshold"
			collapsed={!enabled}
			{header}
			{schema}
			argExtra={{ min: 1, step: 1 }}
			extraLib={stepPropPicker.extraLib}
			animateAppear
			previousModuleId={previousModule?.id}
			bind:editor
		/>
	{/key}
</PropPickerWrapper>
