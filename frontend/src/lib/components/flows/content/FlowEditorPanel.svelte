<script lang="ts">
	import { getContext, onMount } from 'svelte'

	import type { FlowEditorContext } from '../types'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import FlowSettings from './FlowSettings.svelte'
	import FlowInput from './FlowInput.svelte'
	import FlowFailureModule from './FlowFailureModule.svelte'
	import FlowConstants from './FlowConstants.svelte'
	import TriggersEditor from '../../triggers/TriggersEditor.svelte'
	import type { FlowModule } from '$lib/gen'
	import { initFlowStepWarnings } from '../utils'
	import { dfs } from '../dfs'
	import FlowPreprocessorModule from './FlowPreprocessorModule.svelte'

	export let noEditor = false
	export let enableAi = false
	export let newFlow = false
	export let disabledFlowInputs = false

	const { selectedId, flowStore, flowStateStore, flowInputsStore, pathStore, initialPath } =
		getContext<FlowEditorContext>('FlowEditorContext')

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
	<FlowInput {noEditor} disabled={disabledFlowInputs} />
{:else if $selectedId === 'Result'}
	<p class="p-4 text-secondary">Nothing to show about the result node. Happy flow building!</p>
{:else if $selectedId === 'constants'}
	<FlowConstants {noEditor} />
{:else if $selectedId === 'failure'}
	<FlowFailureModule {noEditor} />
{:else if $selectedId === 'preprocessor'}
	<FlowPreprocessorModule {noEditor} />
{:else if $selectedId === 'triggers'}
	<TriggersEditor
		currentPath={$pathStore}
		{initialPath}
		schema={$flowStore.schema}
		{noEditor}
		newItem={newFlow}
		isFlow={true}
	/>
{:else}
	{@const dup = checkDup($flowStore.value.modules)}
	{#if dup}
		<div class="text-red-600 text-xl p-2">There are duplicate modules in the flow at id: {dup}</div>
	{:else}
		{#key $selectedId}
			{#each $flowStore.value.modules as flowModule, index (flowModule.id ?? index)}
				<FlowModuleWrapper
					{noEditor}
					bind:flowModule
					previousModule={$flowStore.value.modules[index - 1]}
					{enableAi}
				/>
			{/each}
		{/key}
	{/if}
{/if}
