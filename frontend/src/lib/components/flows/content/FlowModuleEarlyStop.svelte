<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import PropPickerWrapper from '$lib/components/flows/propPicker/PropPickerWrapper.svelte'
	import type { FlowModule } from '$lib/gen'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { NEVER_TESTED_THIS_FAR } from '../models'

	const { flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')

	export let flowModule: FlowModule

	let editor: SimpleEditor | undefined = undefined

	$: isStopAfterIfEnabled = Boolean(flowModule.stop_after_if)

	$: result = $flowStateStore[flowModule.id]?.previewResult ?? NEVER_TESTED_THIS_FAR
</script>

<div class="flex flex-col items-start space-y-2 {$$props.class}">
	<h2 class="pb-4">
		Early stop/Break
		<Tooltip documentationLink="https://www.windmill.dev/docs/flows/early_stop">
			If defined, at the end of the step, the predicate expression will be evaluated to decide if
			the flow should stop early.
		</Tooltip>
	</h2>
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
			right: 'Early stop or Break if condition met'
		}}
	/>

	<div
		class="w-full border p-2 flex flex-col {flowModule.stop_after_if ? '' : 'bg-surface-secondary'}"
	>
		{#if flowModule.stop_after_if}
			<Toggle
				bind:checked={flowModule.stop_after_if.skip_if_stopped}
				options={{
					right: 'Label flow as "skipped" if stopped'
				}}
			/>
			<span class="text-xs font-bold">Stop condition expression</span>
			<div class="border w-full">
				<PropPickerWrapper
					{result}
					pickableProperties={undefined}
					on:select={({ detail }) => {
						editor?.insertAtCursor(detail)
						editor?.focus()
					}}
				>
					<SimpleEditor
						bind:this={editor}
						lang="javascript"
						bind:code={flowModule.stop_after_if.expr}
						class="small-editor"
						extraLib={`declare const result = ${JSON.stringify(result)};`}
					/>
				</PropPickerWrapper>
			</div>
		{:else}
			<Toggle
				disabled
				options={{
					right: 'Label flow as "skipped" if stopped'
				}}
			/> <span class="text-xs font-bold">Stop condition expression</span>
			<textarea disabled rows="3" class="min-h-[80px]" />
		{/if}
	</div>
</div>
