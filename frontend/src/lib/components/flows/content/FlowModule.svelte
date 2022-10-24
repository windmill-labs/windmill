<script context="module" lang="ts">
	export type FlowModuleWidthContext = {
		width: Writable<number>
		threshold: number
	}
</script>

<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Editor from '$lib/components/Editor.svelte'
	import EditorBar from '$lib/components/EditorBar.svelte'
	import ModulePreview from '$lib/components/ModulePreview.svelte'
	import FlowInputs from './FlowInputs.svelte'
	import {
		createInlineScriptModule,
		createLoop,
		createScriptFromInlineScript,
		fork,
		getStepPropPicker,
		isEmptyFlowModule,
		pickScript
	} from '$lib/components/flows/flowStateUtils'
	import { flowStore } from '$lib/components/flows/flowStore'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { RawScript, Script, type FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowModuleHeader from './FlowModuleHeader.svelte'
	import { flowStateStore, type FlowModuleState } from '../flowState'
	import { scriptLangToEditorLang } from '$lib/utils'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import { afterUpdate, getContext, setContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { loadSchemaFromModule, selectedIdToIndexes } from '../utils'
	import { writable, type Writable } from 'svelte/store'
	import FlowModuleScript from './FlowModuleScript.svelte'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	import FlowRetries from './FlowRetries.svelte'

	const { selectedId, select, previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')

	export let flowModule: FlowModule
	export let flowModuleState: FlowModuleState
	export let failureModule: boolean

	$: [parentIndex, childIndex] = selectedIdToIndexes($selectedId)

	let editor: Editor
	let modulePreview: ModulePreview
	let websocketAlive = { pyright: false, black: false, deno: false, go: false }
	let selected = 'inputs'
	let wrapper: HTMLDivElement
	let panes: HTMLElement
	let totalTopGap = 0

	$: shouldPick = isEmptyFlowModule(flowModule)
	$: stepPropPicker = failureModule
		? { pickableProperties: { previous_result: { error: 'the error message' } }, extraLib: '' }
		: getStepPropPicker(
				[parentIndex, childIndex],
				$flowStore?.schema,
				$flowStateStore,
				$previewArgs
		  )

	function onKeyDown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key == 'Enter') {
			event.preventDefault()
			selected = 'test'
			modulePreview?.runTestWithStepArgs()
		}
	}

	async function apply<T>(fn: (arg: T) => Promise<[FlowModule, FlowModuleState]>, arg: T) {
		const [module, moduleState] = await fn(arg)

		if (
			JSON.stringify(flowModule) != JSON.stringify(module) ||
			JSON.stringify(flowModuleState) != JSON.stringify(moduleState)
		) {
			flowModule = module
			flowModuleState = moduleState
		}
	}

	async function reload(flowModule: FlowModule) {
		const { input_transforms, schema } = await loadSchemaFromModule(flowModule)

		flowModuleState.schema = schema
		if (flowModule.value.type == 'script' || flowModule.value.type == 'rawscript') {
			flowModule.input_transforms = input_transforms
		}
		$flowStore = $flowStore
	}

	async function applyCreateLoop() {
		await apply(createLoop, null)
	}

	export const FLOW_MODULE_WIDTH_THRESHOLD = 768
	const width = writable<number>(0)

	setContext<FlowModuleWidthContext>('FlowModuleWidth', {
		width,
		threshold: FLOW_MODULE_WIDTH_THRESHOLD
	})

	afterUpdate(() => {
		totalTopGap = 0
		if (!(wrapper && panes)) return

		const wrapperTop = wrapper.getBoundingClientRect().top
		const panesTop = panes.getBoundingClientRect().top
		totalTopGap = panesTop - wrapperTop
	})
</script>

<svelte:window on:keydown={onKeyDown} />

{#if flowModule.value.type === 'rawscript' || flowModule.value.type === 'script'}
	<div class="h-full" bind:this={wrapper} bind:clientWidth={$width}>
		<FlowCard bind:flowModule>
			<svelte:fragment slot="header">
				<FlowModuleHeader
					bind:module={flowModule}
					on:delete
					on:toggleSuspend={() => (selected = 'suspend')}
					on:toggleRetry={() => (selected = 'retries')}
					on:toggleStopAfterIf={() => (selected = 'early-stop')}
					on:fork={() => apply(fork, flowModule)}
					on:createScriptFromInlineScript={() => {
						apply(createScriptFromInlineScript, {
							flowModule: flowModule,
							suffix: $selectedId,
							schema: flowModuleState.schema
						})
					}}
				/>
			</svelte:fragment>
			{#if shouldPick}
				<FlowInputs
					shouldDisableTriggerScripts={parentIndex != 0}
					shouldDisableLoopCreation={childIndex !== undefined || $selectedId.includes('failure')}
					on:loop={() => {
						applyCreateLoop()
						select(['loop', $selectedId].join('-'))
					}}
					on:pick={async (e) => {
						await apply(pickScript, { path: e.detail.path, summary: e.detail.summary })
						if (e.detail.kind == Script.kind.APPROVAL) {
							flowModule.suspend = { required_events: 1, timeout: 1800 }
							flowModule = flowModule
						}
					}}
					on:new={async (e) => {
						await apply(createInlineScriptModule, {
							language: e.detail.language,
							kind: e.detail.kind,
							subkind: e.detail.subkind
						})
						if (e.detail.kind == Script.kind.APPROVAL) {
							flowModule.suspend = { required_events: 1, timeout: 1800 }
							flowModule = flowModule
						}
					}}
					{failureModule}
				/>
			{:else}
				{#if flowModule.value.type === 'rawscript'}
					<div class="border-b-2 shadow-sm p-1 mb-1">
						<EditorBar
							{editor}
							lang={flowModule.value['language'] ?? 'deno'}
							{websocketAlive}
							iconOnly={$width < FLOW_MODULE_WIDTH_THRESHOLD}
						/>
					</div>
				{/if}

				<div
					bind:this={panes}
					class="h-full"
					style="max-height: calc(100% - {totalTopGap}px) !important;"
				>
					<Splitpanes horizontal>
						<Pane size={50} minSize={20}>
							{#if flowModule.value.type === 'rawscript'}
								<div on:mouseleave={() => reload(flowModule)} class="h-full">
									<Editor
										bind:websocketAlive
										bind:this={editor}
										class="h-full px-2"
										bind:code={flowModule.value.content}
										deno={flowModule.value.language === RawScript.language.DENO}
										lang={scriptLangToEditorLang(flowModule.value.language)}
										automaticLayout={true}
										cmdEnterAction={async () => {
											selected = 'test'
											await reload(flowModule)
											modulePreview?.runTestWithStepArgs()
										}}
										formatAction={() => reload(flowModule)}
									/>
								</div>
							{:else if flowModule.value.type === 'script'}
								<FlowModuleScript path={flowModule.value.path} />
							{/if}
						</Pane>
						<Pane size={50} minSize={20}>
							<Tabs bind:selected>
								<Tab value="inputs">Inputs</Tab>
								<Tab value="test">Test</Tab>
								<Tab value="retries">Retries</Tab>
								{#if !$selectedId.includes('failure')}
									<Tab value="early-stop">Early Stop</Tab>
									<Tab value="suspend">Sleep/Suspend</Tab>
								{/if}
							</Tabs>
							<div class="h-[calc(100%-32px)]">
								{#if selected === 'inputs'}
									<div class="h-full overflow-auto">
										<PropPickerWrapper pickableProperties={stepPropPicker.pickableProperties}>
											<p class="items-baseline text-xs text-gray-700 italic hidden md:block mb-2">
												Move the focus outside of the text editor to recompute the inputs or press
												Ctrl/Cmd+S
											</p>
											<SchemaForm
												schema={flowModuleState.schema}
												inputTransform={true}
												importPath={$selectedId}
												bind:args={flowModule.value.input_transforms}
												bind:extraLib={stepPropPicker.extraLib}
											/>
										</PropPickerWrapper>
									</div>
								{:else if selected === 'test'}
									<ModulePreview
										bind:this={modulePreview}
										mod={flowModule}
										schema={flowModuleState.schema}
										indices={[parentIndex, childIndex]}
									/>
								{:else if selected === 'retries'}
									<FlowRetries bind:flowModule class="px-4 pb-4 h-full overflow-auto" />
								{:else if selected === 'early-stop'}
									<FlowModuleEarlyStop bind:flowModule class="px-4 pb-4 h-full overflow-auto" />
								{:else if selected === 'suspend'}
									<div class="px-4 pb-4 h-full overflow-auto">
										<FlowModuleSuspend bind:flowModule />
									</div>
								{/if}
							</div>
						</Pane>
					</Splitpanes>
				</div>
			{/if}
		</FlowCard>
	</div>
{:else}
	Incorrect flow module type
{/if}
