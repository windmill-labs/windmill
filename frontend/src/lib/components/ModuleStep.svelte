<script lang="ts">
	import { Job, RawScript, type Flow, type FlowModule } from '$lib/gen'
	import { faArrowDown, faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import Editor from './Editor.svelte'
	import EditorBar from './EditorBar.svelte'
	import FlowPreview from './FlowPreview.svelte'
	import FlowBox from './flows/FlowBox.svelte'
	import FlowInputs from './flows/FlowInputs.svelte'
	import FlowModuleHeader from './flows/FlowModuleHeader.svelte'
	import { flowStore } from './flows/flowStore'
	import {
		createInlineScriptModule,
		createLoop,
		fork,
		loadFlowModuleSchema,
		pickScript,
		createScriptFromInlineScript,
		isEmptyFlowModule
	} from './flows/flowStateUtils'
	import { jobsToResults } from './flows/utils'
	import SchemaForm from './SchemaForm.svelte'
	import type { Schema } from '$lib/common'
	import { flowStateStore, type FlowModuleSchema, type FlowState } from './flows/flowState'
	import { stepOpened } from './flows/stepOpenedStore'
	import { buildExtraLib, objectToTsType, schemaToObject, schemaToTsType } from '$lib/utils'
	import { rotateRight } from 'svelte-awesome/icons'

	export let indexes: number[]
	export let mod: FlowModule
	export let args: Record<string, any> = {}
	export let schema: Schema
	export let previewResults: Array<any>
	export let childFlowModules: FlowModuleSchema[] | undefined = undefined
	export let previousStepPreviewResults: Array<any>

	let editor: Editor
	let websocketAlive = { pyright: false, black: false, deno: false }
	let pickableProperties: Object | undefined = undefined
	let bigEditor = false

	const i = indexes[0]

	type PickableProperties = {
		flow_input?: Object
		previous_result?: Object
		step?: Object[]
	}

	function hasElements(arr: any) {
		return Array.isArray(arr) && arr.length > 0
	}

	function getLast<T>(arr: T[]): T {
		return arr[arr.length - 1]
	}

	function getPickableProperties(flow: Flow, flowState: FlowState): PickableProperties {
		const flowInputAsObject = schemaToObject(flow.schema, args)

		if (indexes.length > 1) {
			const [parentIndex] = indexes

			const stepBeforeLoop = flowState[parentIndex - 1]

			flowInputAsObject['iter'] = {
				value: "iteration's value",
				index: "iteration's index"
			}

			if (hasElements(stepBeforeLoop?.previewResults)) {
				const lastResults = getLast(stepBeforeLoop.previewResults)

				if (hasElements(lastResults)) {
					flowInputAsObject['iter'] = {
						value: getLast(lastResults),
						index: `iteration's index (0 to ${lastResults.length - 1})`
					}
				}
			}
		}

		const hasResults = hasElements(previousStepPreviewResults)
		const last = hasResults ? getLast(previousStepPreviewResults) : {}

		return {
			flow_input: flowInputAsObject,
			previous_result: last,
			step: previousStepPreviewResults
		}
	}

	$: shouldPick = isEmptyFlowModule(mod)
	$: pickableProperties = getPickableProperties($flowStore, $flowStateStore)
	$: extraLib = buildExtraLib(
		schemaToTsType($flowStore?.schema),
		i === 0 ? schemaToTsType($flowStore?.schema) : objectToTsType(previousStepPreviewResults)
	)

	async function apply<T>(fn: (arg: T) => Promise<FlowModuleSchema>, arg: T) {
		const flowModuleSchema = await fn(arg)

		mod = flowModuleSchema.flowModule
		schema = flowModuleSchema.schema

		if (flowModuleSchema.childFlowModules) {
			childFlowModules = flowModuleSchema.childFlowModules
		}
	}

	async function reload(flowModule: FlowModule) {
		apply(loadFlowModuleSchema, flowModule)
	}

	async function applyCreateLoop() {
		await apply(createLoop, null)
		stepOpened.update(() => `${indexes[0]}-0`)
	}

	function onPreview(jobs: Job[]): void {
		const results = jobsToResults(jobs)

		if (indexes.length > 0) {
			flowStateStore.update((flowState) => {
				const [parentIndex, childIndex] = indexes
				const last = results[results.length - 1]

				if (childIndex === 0) {
					flowState[parentIndex].previewResults = [last]
				} else {
					flowState[parentIndex].previewResults.splice(childIndex, 0, last)
				}

				return flowState
			})
		}

		previewResults = results
	}

	$: opened = $stepOpened === String(indexes.join('-'))
</script>

<FlowBox bind:opened>
	<svelte:fragment slot="header">
		<FlowModuleHeader
			bind:mod
			bind:indexes
			on:delete
			on:fork={() => apply(fork, mod)}
			on:createScriptFromInlineScript={() => {
				apply(createScriptFromInlineScript, {
					flowModule: mod,
					suffix: indexes.join('-'),
					schema
				})
			}}
		/>
	</svelte:fragment>

	<div slot="content">
		{#if opened}
			<div class="p-6 border-t border-gray-300">
				{#if shouldPick}
					<FlowInputs
						shouldDisableTriggerScripts={i != 0}
						shouldDisableLoopCreation={indexes.length > 1 || i == 0}
						on:pick={(e) => apply(pickScript, e.detail.path)}
						on:new={(e) =>
							apply(createInlineScriptModule, {
								language: e.detail.language,
								isTrigger: e.detail.isTrigger
							})}
						on:loop={() => applyCreateLoop()}
					/>
				{/if}
				{#if mod.value.type === 'rawscript'}
					<div class="mb-2 overflow-hidden">
						<EditorBar {editor} {websocketAlive} lang={mod.value.language ?? 'deno'} />
					</div>
					<div on:mouseleave={() => reload(mod)}>
						<Editor
							bind:websocketAlive
							bind:this={editor}
							class="{bigEditor ? 'h-2/3' : 'h-80'} border p-2 rounded"
							bind:code={mod.value.content}
							deno={mod.value.language === RawScript.language.DENO}
							automaticLayout={true}
							on:blur={() => reload(mod)}
							formatAction={() => reload(mod)}
						/>
						<button
							class="w-full text-center"
							on:click={() => {
								bigEditor = !bigEditor
							}}
						>
							<Icon data={bigEditor ? faChevronUp : faChevronDown} scale={1.0} />
						</button>
					</div>
					<div class="mt-2 mb-8">
						<p class="text-gray-500 italic">
							Move the focus outside of the text editor to recompute the input schema or press
							Ctrl/Cmd+S
						</p>
					</div>
				{/if}
				{#if !shouldPick}
					<p class="text-lg font-bold text-gray-900 mb-2">Step inputs</p>
					<SchemaForm
						{schema}
						{extraLib}
						inputTransform={true}
						importPath={String(indexes.join('-'))}
						bind:pickableProperties
						bind:args={mod.input_transform}
					/>
				{/if}

				{#if !shouldPick}
					<div class="border-b border-gray-200" />
					<div class="p-3">
						<FlowPreview
							bind:args
							flow={$flowStore}
							{i}
							{schema}
							on:change={(e) => onPreview(e.detail)}
						/>
					</div>
				{/if}
			</div>
		{/if}
	</div>
</FlowBox>
