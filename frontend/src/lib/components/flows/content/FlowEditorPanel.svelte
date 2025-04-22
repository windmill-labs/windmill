<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import FlowSettings from './FlowSettings.svelte'
	import FlowInput from './FlowInput.svelte'
	import FlowFailureModule from './FlowFailureModule.svelte'
	import FlowConstants from './FlowConstants.svelte'
	import TriggersEditor from '../../triggers/TriggersEditor.svelte'
	import type { FlowModule, Flow } from '$lib/gen'
	import { initFlowStepWarnings } from '../utils'
	import { dfs } from '../dfs'
	import FlowPreprocessorModule from './FlowPreprocessorModule.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { insertNewPreprocessorModule } from '../flowStateUtils'
	import TriggersEditorV2 from '../../triggers/TriggersEditorV2.svelte'

	export let noEditor = false
	export let enableAi = false
	export let newFlow = false
	export let disabledFlowInputs = false
	export let savedFlow:
		| (Flow & {
				draft?: Flow | undefined
		  })
		| undefined = undefined

	const useV2 = true //Only for dev

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
	} = getContext<FlowEditorContext>('FlowEditorContext')

	const { selectedTrigger, defaultValues, captureOn, showCaptureHint } =
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

	async function initWarnings() {
		for (const module of $flowStore?.value?.modules) {
			if (!module) {
				continue
			}

			if (!$flowInputsStore) {
				$flowInputsStore = {}
			}

			$flowInputsStore[module?.id] = {
				flowStepWarnings: await initFlowStepWarnings(
					module.value,
					$flowStateStore?.[module?.id]?.schema,
					dfs($flowStore.value.modules, (fm) => fm.id)
				)
			}
		}
	}

	onMount(() => {
		initWarnings()
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
			selectedTrigger.set(ev.detail.kind)
			defaultValues.set(ev.detail.config)
			captureOn.set(true)
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
{:else if $selectedId === 'triggers' && useV2}
	<TriggersEditorV2
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
				$previewArgs = JSON.parse(JSON.stringify(payloadData))
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
		schema={$flowStore.schema}
		{noEditor}
		newItem={newFlow}
		isFlow={true}
		hasPreprocessor={!!$flowStore.value.preprocessor_module}
		canHavePreprocessor={true}
		args={$previewArgs}
		isDeployed={savedFlow && !savedFlow?.draft_only}
	/>
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
				$previewArgs = JSON.parse(JSON.stringify(payloadData))
			}
			if (redirect) {
				$selectedId = 'Input'
				$flowInputEditorState.selectedTab = 'captures'
				$flowInputEditorState.payloadData = payloadData
			}
		}}
		on:testWithArgs
		args={$previewArgs}
		currentPath={$pathStore}
		initialPath={$initialPathStore}
		{fakeInitialPath}
		schema={$flowStore.schema}
		{noEditor}
		newItem={newFlow}
		isFlow={true}
		hasPreprocessor={!!$flowStore.value.preprocessor_module}
		canHavePreprocessor={true}
	/>
{:else if $selectedId.startsWith('subflow:')}
	<div class="p-4"
		>Selected step is witin an expanded subflow and is not directly editable in the flow editor</div
	>
{:else}
	{@const dup = checkDup($flowStore.value.modules)}
	{#if dup}
		<div class="text-red-600 text-xl p-2">There are duplicate modules in the flow at id: {dup}</div>
	{:else}
		{#key $selectedId}
			{#each $flowStore.value.modules as flowModule, index (flowModule.id ?? index)}
				<FlowModuleWrapper
					{noEditor}
					bind:flowModule={$flowStore.value.modules[index]}
					previousModule={$flowStore.value.modules[index - 1]}
					{enableAi}
					savedModule={savedFlow?.value.modules[index]}
				/>
			{/each}
		{/key}
	{/if}
{/if}
