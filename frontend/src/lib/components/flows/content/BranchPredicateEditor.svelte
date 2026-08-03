<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'
	import { getStepPropPicker } from '../previousResults'
	import type { FlowEditorContext } from '../types'
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { emptySchema } from '$lib/utils'

	interface Props {
		branch: {
			summary?: string
			expr: string
			modules: Array<FlowModule>
		}
		parentModule: FlowModule
		previousModule: FlowModule | undefined
	}

	let { branch, parentModule, previousModule }: Props = $props()

	let editor: SimpleEditor | undefined = $state(undefined)

	// The predicate is a bare string on `branch`, so the form is told its kind through
	// `argType` rather than inferring one from the value.
	let predicateSchema = $state(emptySchema())
	predicateSchema.properties['expr'] = { type: 'boolean' }

	const { previewArgs, flowStateStore, flowStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let stepPropPicker = $derived(
		getStepPropPicker(
			flowStateStore.val,
			parentModule,
			previousModule,
			parentModule.id,
			flowStore.val,
			previewArgs.val,
			false
		)
	)
</script>

<PropPickerWrapper
	popover={true}
	flow_input={stepPropPicker.pickableProperties.flow_input}
	notSelectable
	displayContext={false}
	pickableProperties={stepPropPicker.pickableProperties}
	on:select={({ detail }) => {
		editor?.insertAtCursor(detail)
		editor?.focus()
	}}
>
	<!-- `branch` itself is the arg: the form reads and writes `arg.expr` in place, which
	     is exactly where the predicate lives. Its other keys are inert here. -->
	<InputTransformForm
		bind:arg={branch}
		argName="expr"
		argType="javascript"
		label="Run this branch if"
		headerTooltip="The first branch whose expression evaluates to true is the one that runs."
		noDynamicToggle
		schema={predicateSchema}
		previousModuleId={previousModule?.id}
		pickableProperties={stepPropPicker.pickableProperties}
		extraLib={stepPropPicker.extraLib}
		bind:editor
	/>
</PropPickerWrapper>
