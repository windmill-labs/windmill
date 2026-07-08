<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import FlowExpressionEditor from './FlowExpressionEditor.svelte'
	import type { Flow, FlowModule } from '$lib/gen'
	import type { ExtendedOpenFlow, FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { NEVER_TESTED_THIS_FAR } from '../models'
	import { getStepPropPicker } from '../previousResults'
	import { dfs } from '../previousResults'

	const { flowStateStore, flowStore, previewArgs } =
		getContext<FlowEditorContext>('FlowEditorContext')

	interface Props {
		flowModule: FlowModule
	}

	let { flowModule = $bindable() }: Props = $props()

	let stepPropPicker = $derived(
		getStepPropPicker(
			flowStateStore.val,
			undefined,
			undefined,
			flowModule.id,
			flowStore.val,
			previewArgs.val,
			false
		)
	)

	function checkIfBreakableParent(
		flowStoreValue: ExtendedOpenFlow
	):
		| { stepId: string; isParallel: boolean; type: 'loop' }
		| { stepId: string; isParallel: true; type: 'branchall' }
		| null {
		const flow: Flow = JSON.parse(JSON.stringify(flowStoreValue))
		const parents = dfs(flowModule.id, flow, true)
		for (const parent of parents.slice(1)) {
			if (parent.value.type === 'forloopflow' || parent.value.type === 'whileloopflow') {
				return { stepId: parent.id, isParallel: parent.value.parallel ?? false, type: 'loop' }
			} else if (parent.value.type === 'branchall' && parent.value.parallel) {
				return { stepId: parent.id, isParallel: true, type: 'branchall' }
			}
		}
		return null
	}
	let raise_error_message_stop_after_all_if = $state(
		flowModule.stop_after_all_iters_if?.error_message != undefined
	)
	let raise_error_message_stop_after_if = $state(
		flowModule.stop_after_if?.error_message != undefined
	)
	let { isLoop, isParallelLoop } = $derived(
		flowModule.value.type === 'forloopflow' || flowModule.value.type === 'whileloopflow'
			? { isLoop: true, isParallelLoop: flowModule.value.parallel ?? false }
			: { isLoop: false, isParallelLoop: false }
	)
	let isBranchAll = $derived(flowModule.value.type === 'branchall')
	let isStopAfterIfEnabled = $derived(Boolean(flowModule.stop_after_if))
	let isStopAfterAllIterationsEnabled = $derived(Boolean(flowModule.stop_after_all_iters_if))
	let result = $derived(flowStateStore.val[flowModule.id]?.previewResult ?? NEVER_TESTED_THIS_FAR)
	let breakableParent = $derived(checkIfBreakableParent(flowStore.val))
</script>

<div class="flex flex-col items-start gap-3">
	{#if !isBranchAll && !isParallelLoop}
		<div class="w-full">
			<Toggle
				size="xs"
				textClass="text-xs font-normal text-primary"
				checked={isStopAfterIfEnabled}
				on:change={() => {
					if (isStopAfterIfEnabled && flowModule.stop_after_if) {
						flowModule.stop_after_if = undefined
					} else {
						flowModule.stop_after_if = {
							expr: 'result == undefined',
							skip_if_stopped: false,
							error_message: undefined,
							error_include_result: false
						}
					}
				}}
				options={{
					right: isLoop
						? 'Break loop'
						: breakableParent
							? breakableParent.isParallel
								? breakableParent.type === 'loop'
									? 'Skip rest of steps in iteration'
									: 'Skip rest of steps in branch'
								: 'Break parent loop module'
							: 'Stop flow if condition met',
					rightTooltip:
						'At the end of the step the predicate is evaluated to decide whether to stop the flow early, skip the rest of the steps in the iteration/branch (parallel for-loop or branch-all), or break out of a for/while loop or branch-all.',
					rightDocumentationLink: 'https://www.windmill.dev/docs/flows/early_stop'
				}}
			/>

			{#if flowModule.stop_after_if}
				{@const earlyStopResult = isLoop
					? Array.isArray(result) && result.length > 0
						? result[result.length - 1]
						: result === NEVER_TESTED_THIS_FAR
							? result
							: undefined
					: result}
				<div class="w-full mt-2 border rounded-md p-2 flex flex-col gap-2">
					{#if !breakableParent && !isLoop}
						<div class="flex flex-col gap-2">
							<Toggle
								size="xs"
								bind:checked={flowModule.stop_after_if.skip_if_stopped}
								on:change={(event) => {
									if (flowModule.stop_after_if && event.detail) {
										flowModule.stop_after_if.error_message = undefined
										flowModule.stop_after_if.error_include_result = false
										raise_error_message_stop_after_if = false
									}
								}}
								options={{
									right: 'Label flow as "skipped" if stopped'
								}}
							/>
							<Toggle
								size="xs"
								bind:checked={raise_error_message_stop_after_if}
								on:change={(event) => {
									if (flowModule.stop_after_if) {
										if (event.detail) {
											flowModule.stop_after_if.error_message = ''
											flowModule.stop_after_if.skip_if_stopped = false
										} else {
											flowModule.stop_after_if.error_message = undefined
											flowModule.stop_after_if.error_include_result = false
										}
									}
								}}
								options={{
									right: 'Raise an error message if stopped',
									rightTooltip:
										'If enabled and the stop condition is met, an error message will be raised. A custom message can be provided; otherwise, a default message will be used. Mutually exclusive with "Label flow as skipped".'
								}}
							/>
						</div>
					{/if}
					{#if raise_error_message_stop_after_if}
						<input
							type="text"
							bind:value={flowModule.stop_after_if.error_message}
							placeholder="Enter custom error message (optional)"
						/>
						<Toggle
							size="xs"
							bind:checked={flowModule.stop_after_if.error_include_result}
							options={{
								right: "Include the stopping step's result in the error",
								rightTooltip:
									"When enabled, this step's output is embedded inside the raised error object (as error.result) instead of being discarded. The flow result stays { error }."
							}}
						/>
					{/if}
					<FlowExpressionEditor
						bind:code={
							() => flowModule.stop_after_if?.expr ?? '',
							(v) => {
								if (flowModule.stop_after_if) flowModule.stop_after_if.expr = v
							}
						}
						label="Stop condition expression"
						pickableProperties={stepPropPicker.pickableProperties}
						result={earlyStopResult}
						extraResults={isLoop ? { all_iters: result } : undefined}
						extraLib={`declare const result = ${JSON.stringify(earlyStopResult)};\n` +
							stepPropPicker.extraLib +
							(isLoop ? `\ndeclare const all_iters = ${JSON.stringify(result)};` : '')}
					/>
				</div>
			{/if}
		</div>
	{/if}

	{#if isLoop || isBranchAll}
		<div class="w-full">
			<Toggle
				size="xs"
				textClass="text-xs font-normal text-primary"
				checked={isStopAfterAllIterationsEnabled}
				on:change={() => {
					if (isStopAfterAllIterationsEnabled && flowModule.stop_after_all_iters_if) {
						flowModule.stop_after_all_iters_if = undefined
					} else {
						flowModule.stop_after_all_iters_if = {
							expr: 'result == undefined',
							skip_if_stopped: false,
							error_message: undefined,
							error_include_result: false
						}
					}
				}}
				options={{
					right:
						(breakableParent
							? breakableParent.isParallel
								? breakableParent.type === 'loop'
									? 'Skip rest of steps in iteration'
									: 'Skip rest of steps in branch'
								: 'Break parent loop module ' + breakableParent.stepId
							: 'Stop flow') + ' if condition met',
					rightTooltip:
						'At the end of the step the predicate is evaluated to decide whether to stop the flow early, skip the rest of the steps in the iteration/branch (parallel for-loop or branch-all), or break out of a for/while loop or branch-all.',
					rightDocumentationLink: 'https://www.windmill.dev/docs/flows/early_stop'
				}}
			/>

			{#if flowModule.stop_after_all_iters_if}
				<div class="w-full border rounded-md mt-2 p-2 flex flex-col gap-2">
					{#if !breakableParent}
						<div class="flex flex-col gap-2">
							<Toggle
								size="xs"
								bind:checked={flowModule.stop_after_all_iters_if.skip_if_stopped}
								on:change={(event) => {
									if (flowModule.stop_after_all_iters_if && event.detail) {
										flowModule.stop_after_all_iters_if.error_message = undefined
										flowModule.stop_after_all_iters_if.error_include_result = false
										raise_error_message_stop_after_all_if = false
									}
								}}
								options={{
									right: 'Label flow as "skipped" if stopped'
								}}
							/>
							<Toggle
								size="xs"
								bind:checked={raise_error_message_stop_after_all_if}
								on:change={(event) => {
									if (flowModule.stop_after_all_iters_if) {
										if (event.detail) {
											flowModule.stop_after_all_iters_if.error_message = ''
											flowModule.stop_after_all_iters_if.skip_if_stopped = false
										} else {
											flowModule.stop_after_all_iters_if.error_message = undefined
											flowModule.stop_after_all_iters_if.error_include_result = false
										}
									}
								}}
								options={{
									right: 'Raise an error message if stopped',
									rightTooltip:
										'If enabled and the stop condition is met, an error message will be raised. A custom message can be provided; otherwise, a default message will be used. Mutually exclusive with "Label flow as skipped".'
								}}
							/>
						</div>
					{/if}
					{#if raise_error_message_stop_after_all_if}
						<input
							type="text"
							bind:value={flowModule.stop_after_all_iters_if.error_message}
							placeholder="Enter custom error message (optional)"
						/>
						<Toggle
							size="xs"
							bind:checked={flowModule.stop_after_all_iters_if.error_include_result}
							options={{
								right: "Include the stopping step's result in the error",
								rightTooltip:
									"When enabled, this step's output is embedded inside the raised error object (as error.result) instead of being discarded. The flow result stays { error }."
							}}
						/>
					{/if}
					<FlowExpressionEditor
						bind:code={
							() => flowModule.stop_after_all_iters_if?.expr ?? '',
							(v) => {
								if (flowModule.stop_after_all_iters_if) flowModule.stop_after_all_iters_if.expr = v
							}
						}
						label="Stop condition expression"
						pickableProperties={stepPropPicker.pickableProperties}
						{result}
						extraLib={`declare const result = ${JSON.stringify(result)};\n` +
							stepPropPicker.extraLib}
					/>
				</div>
			{/if}
		</div>
	{/if}
</div>
