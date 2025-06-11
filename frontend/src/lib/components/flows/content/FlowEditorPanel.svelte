<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import FlowSettings from './FlowSettings.svelte'
	import FlowInput from './FlowInput.svelte'
	import FlowFailureModule from './FlowFailureModule.svelte'
	import FlowConstants from './FlowConstants.svelte'
	import type { FlowModule, Flow } from '$lib/gen'
	import FlowPreprocessorModule from './FlowPreprocessorModule.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { insertNewPreprocessorModule } from '../flowStateUtils.svelte'
	import TriggersEditor from '../../triggers/TriggersEditor.svelte'
	import { handleSelectTriggerFromKind, type Trigger } from '$lib/components/triggers/utils'
	import { computeMissingInputWarnings } from '../missingInputWarnings'

	interface Props {
		noEditor?: boolean
		enableAi?: boolean
		newFlow?: boolean
		disabledFlowInputs?: boolean
		savedFlow?:
			| (Flow & {
					draft?: Flow | undefined
			  })
			| undefined
		onDeployTrigger?: (trigger: Trigger) => void
	}

	let {
		noEditor = false,
		enableAi = false,
		newFlow = false,
		disabledFlowInputs = false,
		savedFlow = undefined,
		onDeployTrigger = () => {}
	}: Props = $props()

	const {
		selectedId,
		flowStore,
		flowStateStore,
		flowInputsStore,
		pathStore,
		initialPathStore,
		fakeInitialPath,
		previewArgs,
		flowInputEditorState
	} = $state(getContext<FlowEditorContext>('FlowEditorContext'))

	const { showCaptureHint, triggersState, triggersCount } =
		getContext<TriggerContext>('TriggerContext')
	function checkDup(modules: FlowModule[]): string | undefined {
		let seenModules: string[] = []
		for (const m of modules) {
			if (seenModules.includes(m.id)) {
				console.error(`Duplicate module id: ${m.id}`)
				return m.id
			}
			seenModules.push(m.id)
		}
	}

	$effect(() => {
		computeMissingInputWarnings(flowStore, $flowStateStore, flowInputsStore)
	})
</script>

{#if $selectedId?.startsWith('settings')}
	<FlowSettings {noEditor} />
{:else if $selectedId === 'Input'}
	<FlowInput
		{noEditor}
		disabled={disabledFlowInputs}
		on:openTriggers={(ev) => {
			$selectedId = 'triggers'
			handleSelectTriggerFromKind(triggersState, triggersCount, savedFlow?.path, ev.detail.kind)
			showCaptureHint.set(true)
		}}
		on:applyArgs
	/>
{:else if $selectedId === 'Result'}
	<p class="p-4 text-secondary">The result of the flow will be the result of the last node.</p>
{:else if $selectedId === 'constants'}
	<FlowConstants {noEditor} />
{:else if $selectedId === 'failure'}
	<FlowFailureModule {noEditor} savedModule={savedFlow?.value.failure_module} />
{:else if $selectedId === 'preprocessor'}
	<FlowPreprocessorModule {noEditor} savedModule={savedFlow?.value.preprocessor_module} />
{:else if $selectedId === 'triggers'}
	<TriggersEditor
		on:applyArgs
		on:addPreprocessor={async () => {
			await insertNewPreprocessorModule(flowStore, flowStateStore, {
				language: 'bun'
			})
			$selectedId = 'preprocessor'
		}}
		on:updateSchema={(e) => {
			const { payloadData, redirect } = e.detail
			if (payloadData) {
				previewArgs.val = JSON.parse(JSON.stringify(payloadData))
			}
			if (redirect) {
				$selectedId = 'Input'
				$flowInputEditorState.selectedTab = 'captures'
				$flowInputEditorState.payloadData = payloadData
			}
		}}
		on:testWithArgs
		currentPath={$pathStore}
		initialPath={$initialPathStore}
		{fakeInitialPath}
		{noEditor}
		newItem={newFlow}
		isFlow={true}
		hasPreprocessor={!!flowStore.val.value.preprocessor_module}
		canHavePreprocessor={true}
		args={previewArgs.val}
		isDeployed={savedFlow && !savedFlow?.draft_only}
		schema={flowStore.val.schema}
		{onDeployTrigger}
	/>
{:else if $selectedId.startsWith('subflow:')}
	<div class="p-4"
		>Selected step is witin an expanded subflow and is not directly editable in the flow editor</div
	>
{:else}
	{@const dup = checkDup(flowStore.val.value.modules)}
	{#if dup}
		<div class="text-red-600 text-xl p-2">There are duplicate modules in the flow at id: {dup}</div>
	{:else}
		{#key $selectedId}
			{#each flowStore.val.value.modules as flowModule, index (flowModule.id ?? index)}
				<FlowModuleWrapper
					{noEditor}
					bind:flowModule={flowStore.val.value.modules[index]}
					previousModule={flowStore.val.value.modules[index - 1]}
					{enableAi}
					savedModule={savedFlow?.value.modules[index]}
				/>
			{/each}
		{/key}
	{/if}
{/if}
