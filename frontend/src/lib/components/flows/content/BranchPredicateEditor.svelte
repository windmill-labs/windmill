<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'
	import { getStepPropPicker } from '../previousResults'
	import type { FlowEditorContext } from '../types'
	import PredicateGen from '$lib/components/copilot/PredicateGen.svelte'
	import FlowExpressionEditor from './FlowExpressionEditor.svelte'

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

	let { branch = $bindable(), parentModule, previousModule, enableAi = false }: Props = $props()

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

<FlowExpressionEditor
	forceCollapsePicker
	label="Run this branch if"
	bind:code={branch.expr}
	pickableProperties={stepPropPicker.pickableProperties}
	extraLib={stepPropPicker.extraLib}
	id="flow-editor-edit-predicate"
>
	{#snippet tooltip()}
		The first branch whose expression evaluates to true is the one that runs.
	{/snippet}
	{#snippet headerExtra()}
		{#if enableAi}
			<PredicateGen
				on:setExpr={(e) => {
					branch.expr = e.detail
				}}
				on:updateSummary
				pickableProperties={stepPropPicker.pickableProperties}
			/>
		{/if}
	{/snippet}
</FlowExpressionEditor>
