<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'
	import { Pen } from 'lucide-svelte'
	import { getStepPropPicker } from '../previousResults'
	import type { FlowEditorContext } from '../types'
	import Button from '$lib/components/common/button/Button.svelte'
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

	let { branch, parentModule, previousModule, enableAi = false }: Props = $props()

	// Collapsed until edited: the editor mounts Monaco, and a branch-one step renders one
	// of these per branch, so mounting them all scales panel cost with branch count.
	let open = $state(false)

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

{#if open}
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
{:else}
	<div class="flex flex-col gap-2">
		<div class="text-xs font-semibold text-emphasis">Run this branch if</div>
		<div class="flex flex-row items-center gap-2">
			<pre
				class="min-w-0 grow truncate rounded-md border bg-surface px-2 py-1 text-xs {branch.expr?.trim()
					? 'font-mono'
					: 'italic text-tertiary'}">{branch.expr?.trim() || 'No expression'}</pre
			>
			{#if enableAi}
				<PredicateGen
					on:setExpr={(e) => {
						branch.expr = e.detail
					}}
					on:updateSummary
					pickableProperties={stepPropPicker.pickableProperties}
				/>
			{/if}
			<Button
				unifiedSize="sm"
				variant="default"
				iconOnly
				startIcon={{ icon: Pen }}
				title="Edit predicate"
				id="flow-editor-edit-predicate"
				on:click={() => (open = true)}
			/>
		</div>
	</div>
{/if}
