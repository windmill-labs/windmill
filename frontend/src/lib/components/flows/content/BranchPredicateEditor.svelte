<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'
	import { getStepPropPicker } from '../previousResults'
	import type { FlowEditorContext } from '../types'
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import PredicateGen from '$lib/components/copilot/PredicateGen.svelte'
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
		enableAi?: boolean
	}

	let { branch, parentModule, previousModule, enableAi = false }: Props = $props()

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
	sidePane
	flow_input={stepPropPicker.pickableProperties.flow_input}
	notSelectable
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
	>
		{#snippet aiGen()}
			{#if enableAi}
				<PredicateGen
					on:setExpr={(e) => {
						branch.expr = e.detail
						// Monaco owns its buffer once mounted: writing the value alone leaves
						// the visible code stale until the editor is torn down and rebuilt.
						editor?.setCode(e.detail)
					}}
					on:updateSummary={(e) => {
						// The prompt names the branch better than "Branch 2" does, but only
						// when the user hasn't already named it themselves.
						if (!branch.summary) {
							branch.summary = e.detail
						}
					}}
					pickableProperties={stepPropPicker.pickableProperties}
				/>
			{/if}
		{/snippet}
	</InputTransformForm>
</PropPickerWrapper>
