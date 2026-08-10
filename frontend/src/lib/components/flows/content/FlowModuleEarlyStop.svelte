<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { stepSettingDefaults } from '../flowStepSettings'
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import type { Flow, FlowModule, StopAfterIf } from '$lib/gen'
	import Label from '$lib/components/Label.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import type { ExtendedOpenFlow, FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { emptySchema } from '$lib/utils'
	import { NEVER_TESTED_THIS_FAR } from '../models'
	import { getStepPropPicker } from '../previousResults'
	import { dfs } from '../previousResults'
	import { slideDynamic } from '$lib/transitions'

	const { flowStateStore, flowStore, previewArgs } =
		getContext<FlowEditorContext>('FlowEditorContext')

	interface Props {
		flowModule: FlowModule
		/** A loop shows both predicates, and puts the per-iteration one (`stop_after_if`)
		 *  next to its own settings rather than in the run-settings list. */
		blocks?: 'both' | 'stop-after' | 'all-iters'
	}

	let { flowModule = $bindable(), blocks = 'both' }: Props = $props()

	let stopAfterEditor: SimpleEditor | undefined = $state(undefined)
	let stopAfterAllItersEditor: SimpleEditor | undefined = $state(undefined)

	// Both predicates are stored as a bare `{ expr }`, so the form is told their kind
	// through `argType` rather than inferring one from the value.
	let predicateSchema = $state(emptySchema())
	predicateSchema.properties['stop_after_if'] = { type: 'boolean' }
	predicateSchema.properties['stop_after_all_iters_if'] = { type: 'boolean' }

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
	// `skip_if_stopped` and `error_message` are mutually exclusive in the worker, and
	// setting neither is a third outcome — so the three are one choice, not two flags.
	type StopStatus = 'success' | 'skipped' | 'error'
	function stopStatus(stop: StopAfterIf): StopStatus {
		if (stop.skip_if_stopped) return 'skipped'
		if (stop.error_message != undefined) return 'error'
		return 'success'
	}
	function setStopStatus(stop: StopAfterIf, status: StopStatus) {
		stop.skip_if_stopped = status === 'skipped'
		stop.error_message = status === 'error' ? (stop.error_message ?? '') : undefined
		if (status !== 'error') stop.error_include_result = false
	}

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
	// One `stop_after_if` field, but what stopping early *does* depends on where the step
	// sits — so the name and the explanation are picked together rather than sharing one
	// tooltip that has to enumerate every case.
	let stopAfterCopy = $derived(
		isParallelLoop
			? {
					label: 'Break loop if',
					tooltip:
						'Unavailable on a parallel loop: iterations don\'t run in sequence, so there is nothing to break out of and the worker skips this predicate. Use "Stop flow if" to decide once every iteration has completed.'
				}
			: isLoop
				? {
						label: 'Break loop if',
						tooltip:
							'Evaluated after each iteration. When it returns true the loop stops iterating and the flow carries on with the iterations collected so far.'
					}
				: breakableParent
					? breakableParent.isParallel
						? breakableParent.type === 'loop'
							? {
									label: 'Skip rest of iteration if',
									tooltip:
										'Evaluated after this step. When it returns true the remaining steps of this iteration are skipped; the other iterations are unaffected.'
								}
							: {
									label: 'Skip rest of branch if',
									tooltip:
										'Evaluated after this step. When it returns true the remaining steps of this branch are skipped; the other branches are unaffected.'
								}
						: {
								label: 'Break parent loop if',
								tooltip: `Evaluated after this step. When it returns true the enclosing loop ${breakableParent.stepId} stops iterating and the flow carries on.`
							}
					: {
							label: 'Stop flow if',
							tooltip:
								"Evaluated after this step. When it returns true the flow stops here and returns this step's result."
						}
	)

	// The all-iterations predicate runs once the loop or branch-all is done, over what
	// every iteration returned, so it can decide on the whole rather than on one result.
	let allItersPrefix = $derived(
		`Evaluated once ${isBranchAll ? 'every branch' : 'every iteration'} has completed, over their collected results.`
	)
	let stopAfterAllItersCopy = $derived(
		breakableParent
			? breakableParent.isParallel
				? breakableParent.type === 'loop'
					? {
							label: 'Skip rest of iteration if',
							tooltip: `${allItersPrefix} When it returns true the remaining steps of the enclosing iteration are skipped.`
						}
					: {
							label: 'Skip rest of branch if',
							tooltip: `${allItersPrefix} When it returns true the remaining steps of the enclosing branch are skipped.`
						}
				: {
						label: `Break parent loop ${breakableParent.stepId} if`,
						tooltip: `${allItersPrefix} When it returns true the enclosing loop stops iterating and the flow carries on.`
					}
			: {
					label: 'Stop flow if',
					tooltip: `${allItersPrefix} When it returns true the flow stops here and returns them.`
				}
	)

	let earlyStopResult = $derived(
		isLoop
			? Array.isArray(result) && result.length > 0
				? result[result.length - 1]
				: result === NEVER_TESTED_THIS_FAR
					? result
					: undefined
			: result
	)
</script>

{#snippet stopStatusPicker(stop: StopAfterIf)}
	<div class="flex flex-col gap-2 pl-9" transition:slideDynamic>
		<Label
			label="Flow status"
			tooltip="How the flow is reported once this condition stops it. Success returns this step's result, Skipped marks the flow as skipped, and Error fails it."
		>
			<ToggleButtonGroup
				noWFull
				selected={stopStatus(stop)}
				onSelected={(v) => setStopStatus(stop, v)}
			>
				{#snippet children({ item })}
					<ToggleButton value="success" label="Success" {item} small />
					<ToggleButton value="skipped" label="Skipped" {item} small />
					<ToggleButton value="error" label="Error" {item} small />
				{/snippet}
			</ToggleButtonGroup>
		</Label>
		{#if stop.error_message != undefined}
			<div class="flex flex-col gap-2" transition:slideDynamic>
				<TextInput
					size="sm"
					bind:value={() => stop.error_message ?? '', (v) => (stop.error_message = String(v))}
					inputProps={{ placeholder: 'Enter custom error message (optional)' }}
				/>
				<Toggle
					size="xs"
					bind:checked={
						() => stop.error_include_result ?? false, (v) => (stop.error_include_result = v)
					}
					options={{
						right: 'Include result in error',
						rightTooltip:
							"When enabled, this step's output is embedded inside the raised error object (as error.result) instead of being discarded. The flow result stays { error }."
					}}
				/>
			</div>
		{/if}
	</div>
{/snippet}

{#snippet stopAfterToggle()}
	<Toggle
		size="xs"
		textClass="text-xs font-normal text-primary"
		disabled={isParallelLoop}
		checked={isStopAfterIfEnabled}
		on:change={() => {
			if (isStopAfterIfEnabled && flowModule.stop_after_if) {
				flowModule.stop_after_if = undefined
			} else {
				flowModule.stop_after_if = stepSettingDefaults('early-stop')
			}
		}}
		options={{
			title: isParallelLoop ? stopAfterCopy.tooltip : undefined,
			right: stopAfterCopy.label,
			rightTooltip: stopAfterCopy.tooltip,
			rightDocumentationLink: 'https://www.windmill.dev/docs/flows/early_stop'
		}}
	/>
{/snippet}

{#snippet stopAfterAllItersToggle()}
	<Toggle
		size="xs"
		textClass="text-xs font-normal text-primary"
		checked={isStopAfterAllIterationsEnabled}
		on:change={() => {
			if (isStopAfterAllIterationsEnabled && flowModule.stop_after_all_iters_if) {
				flowModule.stop_after_all_iters_if = undefined
			} else {
				flowModule.stop_after_all_iters_if = stepSettingDefaults('early-stop')
			}
		}}
		options={{
			right: stopAfterAllItersCopy.label,
			rightTooltip: stopAfterAllItersCopy.tooltip,
			rightDocumentationLink: 'https://www.windmill.dev/docs/flows/early_stop'
		}}
	/>
{/snippet}

<div class="flex flex-col items-start gap-6">
	{#if blocks !== 'all-iters' && !isBranchAll}
		<div class="w-full flex flex-col gap-2">
			<PropPickerWrapper
				sidePane
				flow_input={stepPropPicker.pickableProperties.flow_input}
				notSelectable
				result={earlyStopResult}
				extraResults={isLoop ? { all_iters: result } : undefined}
				pickableProperties={stepPropPicker.pickableProperties}
				on:select={({ detail }) => {
					stopAfterEditor?.insertAtCursor(detail)
					stopAfterEditor?.focus()
				}}
			>
				<InputTransformForm
					bind:arg={
						() => flowModule.stop_after_if,
						(v) => {
							flowModule.stop_after_if = v
						}
					}
					argName="stop_after_if"
					argType="javascript"
					collapsed={!isStopAfterIfEnabled || isParallelLoop}
					animateAppear
					header={stopAfterToggle}
					noDynamicToggle
					schema={predicateSchema}
					previousModuleId={undefined}
					pickableProperties={stepPropPicker.pickableProperties}
					extraLib={`declare const result = ${JSON.stringify(earlyStopResult)};\n` +
						stepPropPicker.extraLib +
						(isLoop ? `\ndeclare const all_iters = ${JSON.stringify(result)};` : '')}
					bind:editor={stopAfterEditor}
				/>
			</PropPickerWrapper>
			{#if isStopAfterIfEnabled && !breakableParent && !isLoop && flowModule.stop_after_if}
				{@render stopStatusPicker(flowModule.stop_after_if)}
			{/if}
		</div>
	{/if}

	{#if blocks !== 'stop-after' && (isLoop || isBranchAll)}
		<div class="w-full flex flex-col gap-2">
			<PropPickerWrapper
				sidePane
				flow_input={stepPropPicker.pickableProperties.flow_input}
				notSelectable
				{result}
				pickableProperties={stepPropPicker.pickableProperties}
				on:select={({ detail }) => {
					stopAfterAllItersEditor?.insertAtCursor(detail)
					stopAfterAllItersEditor?.focus()
				}}
			>
				<InputTransformForm
					bind:arg={
						() => flowModule.stop_after_all_iters_if,
						(v) => {
							flowModule.stop_after_all_iters_if = v
						}
					}
					argName="stop_after_all_iters_if"
					argType="javascript"
					collapsed={!isStopAfterAllIterationsEnabled}
					animateAppear
					header={stopAfterAllItersToggle}
					noDynamicToggle
					schema={predicateSchema}
					previousModuleId={undefined}
					pickableProperties={stepPropPicker.pickableProperties}
					extraLib={`declare const result = ${JSON.stringify(result)};\n` + stepPropPicker.extraLib}
					bind:editor={stopAfterAllItersEditor}
				/>
			</PropPickerWrapper>
			{#if isStopAfterAllIterationsEnabled && !breakableParent && flowModule.stop_after_all_iters_if}
				{@render stopStatusPicker(flowModule.stop_after_all_iters_if)}
			{/if}
		</div>
	{/if}
</div>
