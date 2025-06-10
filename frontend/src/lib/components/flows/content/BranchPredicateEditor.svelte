<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'

	import { getStepPropPicker } from '../previousResults'
	import type { FlowEditorContext } from '../types'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Pen } from 'lucide-svelte'
	import PredicateGen from '$lib/components/copilot/PredicateGen.svelte'

	export let branch: {
		summary?: string
		expr: string
		modules: Array<FlowModule>
	}
	export let parentModule: FlowModule
	export let previousModule: FlowModule | undefined
	export let enableAi = false

	const { previewArgs, flowStateStore, flowStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let editor: SimpleEditor | undefined = undefined
	let open = false
	$: stepPropPicker = getStepPropPicker(
		$flowStateStore,
		parentModule,
		previousModule,
		parentModule.id,
		flowStore.val,
		previewArgs.val,
		false
	)
</script>

{#if open}
	<PropPickerWrapper
		alwaysOn
		notSelectable
		pickableProperties={stepPropPicker.pickableProperties}
		on:select={({ detail }) => {
			editor?.insertAtCursor(detail)
			editor?.focus()
		}}
		paneClass="max-h-[320px] overflow-auto"
	>
		<div class="border border-gray-400">
			<SimpleEditor
				bind:this={editor}
				lang="javascript"
				bind:code={branch.expr}
				class="small-editor border "
				shouldBindKey={false}
				extraLib={stepPropPicker.extraLib}
			/>
		</div>
	</PropPickerWrapper>
{:else}
	<div class="flex justify-between gap-4 p-2">
		<div class="truncate"><pre class="text-sm truncate">{branch.expr}</pre></div>
		<div class="flex flex-row gap-2 items-center">
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
				size="xs"
				startIcon={{ icon: Pen }}
				variant="border"
				on:click={() => (open = !open)}
				id="flow-editor-edit-predicate"
			>
				Edit predicate
			</Button>
		</div>
	</div>
{/if}
