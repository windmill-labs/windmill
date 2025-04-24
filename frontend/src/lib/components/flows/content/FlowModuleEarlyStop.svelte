<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import PropPickerWrapper from '$lib/components/flows/propPicker/PropPickerWrapper.svelte'
	import type { Flow, FlowModule } from '$lib/gen'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { NEVER_TESTED_THIS_FAR } from '../models'
	import Section from '$lib/components/Section.svelte'
	import { getStepPropPicker } from '../previousResults'
	import { dfs } from '../previousResults'

	const { flowStateStore, flowStore, previewArgs } =
		getContext<FlowEditorContext>('FlowEditorContext')

	export let flowModule: FlowModule

	let editor: SimpleEditor | undefined = undefined
	$: stepPropPicker = getStepPropPicker(
		$flowStateStore,
		undefined,
		undefined,
		flowModule.id,
		$flowStore,
		$previewArgs,
		false
	)

	function checkIfParentLoop(flowStore: typeof $flowStore): string | null {
		const flow: Flow = JSON.parse(JSON.stringify(flowStore))
		const parents = dfs(flowModule.id, flow, true)
		for (const parent of parents.slice(1)) {
			if (parent.value.type === 'forloopflow' || parent.value.type === 'whileloopflow') {
				return parent.id
			}
		}
		return null
	}

	$: isLoop = flowModule.value.type === 'forloopflow' || flowModule.value.type === 'whileloopflow'
	$: isBranchAll = flowModule.value.type === 'branchall'
	$: isStopAfterIfEnabled = Boolean(flowModule.stop_after_if)
	$: isStopAfterAllIterationsEnabled = Boolean(flowModule.stop_after_all_iters_if)
	$: result = $flowStateStore[flowModule.id]?.previewResult ?? NEVER_TESTED_THIS_FAR
	$: parentLoopId = checkIfParentLoop($flowStore)
</script>

<div class="flex flex-col items-start space-y-2 {$$props.class}">
	{#if !isBranchAll}
		<Section
			label={(isLoop
				? 'Break loop'
				: parentLoopId
					? 'Break parent loop module ' + parentLoopId
					: 'Stop flow early') + (isLoop ? ' (evaluated after each iteration)' : '')}
			class="w-full"
		>
			<svelte:fragment slot="header">
				<Tooltip documentationLink="https://www.windmill.dev/docs/flows/early_stop">
					If defined, at the end of the step, the predicate expression will be evaluated to decide
					if the flow should stop early or break if inside a for/while loop.
				</Tooltip>
			</svelte:fragment>

			<Toggle
				checked={isStopAfterIfEnabled}
				on:change={() => {
					if (isStopAfterIfEnabled && flowModule.stop_after_if) {
						flowModule.stop_after_if = undefined
					} else {
						flowModule.stop_after_if = {
							expr: 'result == undefined',
							skip_if_stopped: false,
							message: '',
							raise_error_message: false
						}
					}
				}}
				options={{
					right: isLoop
						? 'Break loop'
						: parentLoopId
							? 'Break parent loop module'
							: 'Stop flow if condition met'
				}}
			/>

			<div
				class="w-full mt-2 border p-2 flex flex-col gap-2 {flowModule.stop_after_if
					? ''
					: 'bg-surface-secondary'}"
			>
				{#if flowModule.stop_after_if}
					{@const earlyStopResult = isLoop
						? Array.isArray(result) && result.length > 0
							? result[result.length - 1]
							: result === NEVER_TESTED_THIS_FAR
								? result
								: undefined
						: result}
					{#if !parentLoopId && !isLoop}
						<div class="flex flex-col gap-2">
							<Toggle
								size="xs"
								bind:checked={flowModule.stop_after_if.skip_if_stopped}
								options={{
									right: 'Label flow as "skipped" if stopped'
								}}
							/>
							<Toggle
								size="xs"
								bind:checked={flowModule.stop_after_if.raise_error_message}
								options={{
									right: 'Raise an error message if stopped',
									rightTooltip:
										'If enabled and the stop condition is met, an error message will be raised. A custom message can be provided; otherwise, a default message will be used.'
								}}
							/>
						</div>
					{/if}
					{#if flowModule.stop_after_if.raise_error_message}
						<input
							type="text"
							bind:value={flowModule.stop_after_if.message}
							placeholder="Enter custom error message (optional)"
						/>
					{/if}
					<span class="mt-2 text-xs font-bold">Stop condition expression</span>
					<div class="border w-full">
						<PropPickerWrapper
							notSelectable
							flow_input={stepPropPicker.pickableProperties.flow_input}
							pickableProperties={undefined}
							result={earlyStopResult}
							extraResults={isLoop ? { all_iters: result } : undefined}
							on:select={({ detail }) => {
								editor?.insertAtCursor(detail)
								editor?.focus()
							}}
						>
							<SimpleEditor
								bind:this={editor}
								lang="javascript"
								bind:code={flowModule.stop_after_if.expr}
								class="few-lines-editor"
								extraLib={`declare const result = ${JSON.stringify(earlyStopResult)};` +
									`\n declare const flow_input = ${JSON.stringify(stepPropPicker.pickableProperties.flow_input)};` +
									(isLoop ? `\ndeclare const all_iters = ${JSON.stringify(result)};` : '')}
							/>
						</PropPickerWrapper>
					</div>
				{:else}
					{#if !parentLoopId && !isLoop}
						<div class="flex flex-col gap-2">
							<Toggle
								size="xs"
								options={{
									right: 'Label flow as "skipped" if stopped'
								}}
							/>
							<Toggle
								size="xs"
								options={{
									right: 'Raise an error message if stopped'
								}}
							/>
						</div>
					{/if}
					<span class="mt-2 text-xs font-bold">Stop condition expression</span>
					<textarea disabled rows="3" class="min-h-[80px]"></textarea>
				{/if}
			</div>
		</Section>
	{/if}

	{#if isLoop || isBranchAll}
		<Section
			label={(parentLoopId ? 'Break parent loop module ' + parentLoopId : 'Stop flow early') +
				(isBranchAll
					? ' (evaluated after all branches have been run)'
					: ' (evaluated after all iterations)')}
			class="w-full"
		>
			<svelte:fragment slot="header">
				<Tooltip documentationLink="https://www.windmill.dev/docs/flows/early_stop">
					If defined, at the end of the step, the predicate expression will be evaluated to decide
					if the flow should stop early or break if inside a for/while loop.
				</Tooltip>
			</svelte:fragment>

			<Toggle
				checked={isStopAfterAllIterationsEnabled}
				on:change={() => {
					if (isStopAfterAllIterationsEnabled && flowModule.stop_after_all_iters_if) {
						flowModule.stop_after_all_iters_if = undefined
					} else {
						flowModule.stop_after_all_iters_if = {
							expr: 'result == undefined',
							skip_if_stopped: false,
							raise_error_message: false,
							message: ''
						}
					}
				}}
				options={{
					right: (parentLoopId ? 'Break parent loop module' : 'Stop flow') + ' if condition met'
				}}
			/>

			<div
				class="w-full border mt-2 p-2 flex flex-col gap-2 {flowModule.stop_after_all_iters_if
					? ''
					: 'bg-surface-secondary'}"
			>
				{#if flowModule.stop_after_all_iters_if}
					{#if !parentLoopId}
						<div class="flex flex-col gap-2">
							<Toggle
								size="xs"
								bind:checked={flowModule.stop_after_all_iters_if.skip_if_stopped}
								options={{
									right: 'Label flow as "skipped" if stopped'
								}}
							/>
							<Toggle
								size="xs"
								bind:checked={flowModule.stop_after_all_iters_if.raise_error_message}
								options={{
									right: 'Raise an error message if stopped',
									rightTooltip:
										'If enabled and the stop condition is met, an error message will be raised. A custom message can be provided; otherwise, a default message will be used.'
								}}
							/>
						</div>
					{/if}
					{#if flowModule.stop_after_all_iters_if.raise_error_message}
						<input
							type="text"
							bind:value={flowModule.stop_after_all_iters_if.message}
							placeholder="Enter custom error message (optional)"
						/>
					{/if}
					<span class="mt-2 text-xs font-bold">Stop condition expression</span>
					<div class="border w-full">
						<PropPickerWrapper
							notSelectable
							flow_input={stepPropPicker.pickableProperties.flow_input}
							pickableProperties={undefined}
							{result}
							on:select={({ detail }) => {
								editor?.insertAtCursor(detail)
								editor?.focus()
							}}
						>
							<SimpleEditor
								bind:this={editor}
								lang="javascript"
								bind:code={flowModule.stop_after_all_iters_if.expr}
								class="few-lines-editor"
								extraLib={`declare const result = ${JSON.stringify(result)};` +
									`\ndeclare const flow_input = ${JSON.stringify(stepPropPicker.pickableProperties.flow_input)};`}
							/>
						</PropPickerWrapper>
					</div>
				{:else}
					{#if !parentLoopId}
						<div class="flex flex-col gap-2">
							<Toggle
								disabled
								size="xs"
								options={{
									right: 'Label flow as "skipped" if stopped'
								}}
							/>
							<Toggle
								disabled
								size="xs"
								options={{
									right: 'Raise an error message if stopped'
								}}
							/>
						</div>
					{/if}
					<span class="mt-2 text-xs font-bold">Stop condition expression</span>
					<textarea disabled rows="3" class="min-h-[80px]"></textarea>
				{/if}
			</div>
		</Section>
	{/if}
</div>
