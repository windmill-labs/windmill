<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'

	import { getStepPropPicker } from '../previousResults'
	import type { FlowEditorContext } from '../types'
	import Button from '$lib/components/common/button/Button.svelte'
	import { faPen } from '@fortawesome/free-solid-svg-icons'

	export let branch: {
		summary?: string
		expr: string
		modules: Array<FlowModule>
	}
	export let parentModule: FlowModule
	export let previousModule: FlowModule | undefined

	const { previewArgs, flowStateStore, flowStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let editor: SimpleEditor | undefined = undefined
	let open = false
	$: stepPropPicker = getStepPropPicker(
		$flowStateStore,
		parentModule,
		previousModule,
		parentModule.id,
		$flowStore,
		$previewArgs,
		false
	)
</script>

{#if open}
	<PropPickerWrapper
		notSelectable
		pickableProperties={stepPropPicker.pickableProperties}
		on:select={({ detail }) => {
			editor?.insertAtCursor(detail)
			editor?.focus()
		}}
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
		<div><pre class="text-sm">{branch.expr}</pre></div><div
			><Button startIcon={{ icon: faPen }} variant="border" on:click={() => (open = !open)}
				>Edit Predicate</Button
			></div
		>
	</div>
{/if}
