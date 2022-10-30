<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import PropPickerWrapper from '$lib/components/flows/propPicker/PropPickerWrapper.svelte'
	import type { FlowModule } from '$lib/gen'
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { getStepPropPicker } from '../previousResults'
	import { flowStateStore } from '../flowState'
	import { flowStore } from '../flowStore'
	import Tooltip from '$lib/components/Tooltip.svelte'

	const { previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')

	export let previousModuleId: string | undefined
	export let flowModule: FlowModule
	export let parentModule: FlowModule | undefined

	let editor: SimpleEditor | undefined = undefined

	$: isStopAfterIfEnabled = Boolean(flowModule.stop_after_if)

	let pickableProperties: Record<string, any> = {}

	$: {
		const propPicker = getStepPropPicker(
			$flowStateStore,
			parentModule,
			flowModule.id,
			$flowStore,
			previewArgs
		).pickableProperties
		propPicker['result'] = propPicker['previous_result']
		delete propPicker['previous_result']
		pickableProperties = propPicker
	}
</script>

<div class="flex flex-col items-start space-y-2 {$$props.class}">
	<h2 class="mt-2"
		>Early stop <Tooltip>
			If defined, at the end of the step, the predicate expression will be evaluated to decide if
			the flow should stop early. Skipped flows are just a label useful to not see them in the runs
			page.</Tooltip
		></h2
	>
	<Toggle
		checked={isStopAfterIfEnabled}
		on:change={() => {
			if (isStopAfterIfEnabled && flowModule.stop_after_if) {
				flowModule.stop_after_if = undefined
			} else {
				flowModule.stop_after_if = {
					expr: 'result == undefined',
					skip_if_stopped: false
				}
			}
		}}
		options={{
			right: 'Early stop if condition met'
		}}
	/>

	{#if flowModule.stop_after_if}
		<span class="text-xs font-bold">Should skip if stopped</span>
		<input type="checkbox" bind:checked={flowModule.stop_after_if.skip_if_stopped} />
		<span class="text-xs font-bold">Stop condition expression</span>
		<div class="border w-full">
			<PropPickerWrapper
				priorId={previousModuleId}
				{pickableProperties}
				on:select={({ detail }) => {
					editor?.insertAtCursor(detail)
				}}
			>
				<SimpleEditor
					bind:this={editor}
					lang="javascript"
					bind:code={flowModule.stop_after_if.expr}
					class="small-editor"
				/>
			</PropPickerWrapper>
		</div>
	{:else}
		<span class="text-xs font-bold">Should skip if stopped</span>
		<input type="checkbox" disabled />
		<span class="text-xs font-bold">Stop condition expression</span>
		<textarea disabled rows="3" class="min-h-[80px]" />
	{/if}
</div>
