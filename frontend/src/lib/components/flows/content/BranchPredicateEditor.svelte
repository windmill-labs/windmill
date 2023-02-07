<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'
	import { flowStore } from '../flowStore'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'

	import { flowStateStore } from '../flowState'
	import { getStepPropPicker } from '../previousResults'
	import type { FlowEditorContext } from '../types'
	import JavascriptEditor from '$lib/components/JavascriptEditor.svelte'

	export let branch: {
		summary?: string
		expr: string
		modules: Array<FlowModule>
	}
	export let parentModule: FlowModule
	export let previousModule: FlowModule | undefined

	const { previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')
	let editor: JavascriptEditor | undefined = undefined

	$: stepPropPicker = getStepPropPicker(
		$flowStateStore,
		parentModule,
		previousModule,
		parentModule.id,
		$flowStore,
		$previewArgs,
		false,
		true
	)
</script>

<PropPickerWrapper
	notSelectable
	pickableProperties={stepPropPicker.pickableProperties}
	on:select={({ detail }) => {
		editor?.insertAtCursor(detail)
	}}
>
	<div class="border border-gray-400">
		<JavascriptEditor
			bind:this={editor}
			lang="javascript"
			bind:code={branch.expr}
			class="small-editor border "
			shouldBindKey={false}
			extraLib={stepPropPicker.extraLib}
		/>
	</div>
</PropPickerWrapper>
