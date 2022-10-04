<script context="module" lang="ts">
	export type FlowModuleWidthContext = {
		width: Writable<number>
		threshold: number
	}
</script>

<script lang="ts">
	import { VSplitPane } from 'svelte-split-pane'

	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
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

	import { RawScript, type FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowModuleHeader from './FlowModuleHeader.svelte'
	import { flowStateStore, type FlowModuleState } from '../flowState'
	import { scriptLangToEditorLang } from '$lib/utils'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import { getContext, setContext } from 'svelte'
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
	let websocketAlive = { pyright: false, black: false, deno: false }
	let selected = 'inputs'

	$: shouldPick = isEmptyFlowModule(flowModule)
	$: stepPropPicker = failureModule
		? { pickableProperties: { previous_result: { error: 'the error message' } }, extraLib: '' }
		: getStepPropPicker([parentIndex, childIndex], $flowStore.schema, $flowStateStore, $previewArgs)

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
		flowModule.input_transforms = input_transforms
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
</script>

<svelte:window on:keydown={onKeyDown} />

<div class="flex flex-col h-full" bind:clientWidth={$width}>
	<FlowCard bind:flowModule>
		<svelte:fragment slot="header">
			<FlowModuleHeader
				bind:module={flowModule}
				on:delete
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
				shouldDisableLoopCreation={childIndex !== undefined ||
					parentIndex === 0 ||
					$selectedId.includes('failure')}
				on:loop={() => {
					applyCreateLoop()
					select(['loop', $selectedId].join('-'))
				}}
				on:pick={(e) => apply(pickScript, e.detail.path)}
				on:new={(e) =>
					apply(createInlineScriptModule, {
						language: e.detail.language,
						kind: e.detail.kind,
						subkind: e.detail.subkind
					})}
				{failureModule}
			/>
		{:else}
			{#if flowModule.value.type === 'rawscript'}
				<div class="flex-shrink-0 border-b p-1">
					<EditorBar
						{editor}
						lang={flowModule.value['language'] ?? 'deno'}
						{websocketAlive}
						iconOnly={$width < FLOW_MODULE_WIDTH_THRESHOLD}
					/>
				</div>
			{/if}

			<div class="overflow-hidden flex-grow">
				<VSplitPane
					topPanelSize={flowModule.value.type === 'rawscript' ? '50%' : '0%'}
					downPanelSize={flowModule.value.type === 'rawscript' ? '50%' : '100%'}
					minTopPaneSize="20%"
					minDownPaneSize="20%"
				>
					<top slot="top">
						{#if flowModule.value.type === 'rawscript'}
							<div on:mouseleave={() => reload(flowModule)} class="h-full overflow-auto">
								<Editor
									bind:websocketAlive
									bind:this={editor}
									class="h-full px-2"
									bind:code={flowModule.value.content}
									deno={flowModule.value.language === RawScript.language.DENO}
									lang={scriptLangToEditorLang(flowModule.value.language)}
									automaticLayout={true}
									cmdEnterAction={() => {
										selected = 'test'
										modulePreview?.runTestWithStepArgs()
									}}
									formatAction={() => reload(flowModule)}
								/>
							</div>
						{:else if flowModule.value.type === 'script'}
							<FlowModuleScript {flowModule} />
						{/if}
					</top>

					<down slot="down" class="flex flex-col flex-1 h-full">
						<Tabs bind:selected>
							<Tab value="inputs">Inputs</Tab>
							<Tab value="test">Test</Tab>
							<Tab value="retries">Retries</Tab>
							{#if !$selectedId.includes('failure')}
								<Tab value="early-stop">Early Stop</Tab>
								<Tab value="suspend">Suspend</Tab>
							{/if}

							<svelte:fragment slot="content">
								<div class="overflow-hidden bg-white" style="height:calc(100% - 32px);">
									<TabContent value="inputs" class="flex flex-col flex-1 h-full">
										<PropPickerWrapper pickableProperties={stepPropPicker.pickableProperties}>
											<!-- <pre class="text-xs">{JSON.stringify($flowStateStore, null, 4)}</pre> -->
											<SchemaForm
												schema={flowModuleState.schema}
												inputTransform={true}
												importPath={$selectedId}
												bind:args={flowModule.input_transforms}
												bind:extraLib={stepPropPicker.extraLib}
											/>
										</PropPickerWrapper>
									</TabContent>
									<TabContent value="test" class="flex flex-col flex-1 h-full" alwaysMounted={true}>
										<ModulePreview
											bind:this={modulePreview}
											mod={flowModule}
											schema={flowModuleState.schema}
											indices={[parentIndex, childIndex]}
										/>
									</TabContent>

									<TabContent value="retries" class="flex flex-col flex-1 h-full">
										<div class="p-4 overflow-y-auto">
											<FlowRetries bind:flowModule />
										</div>
									</TabContent>

									<TabContent value="early-stop" class="flex flex-col flex-1 h-full">
										<div class="p-4 overflow-y-auto">
											<FlowModuleEarlyStop bind:flowModule />
										</div>
									</TabContent>

									<TabContent value="suspend" class="flex flex-col flex-1 h-full">
										<div class="p-4 overflow-y-auto">
											<FlowModuleSuspend bind:flowModule />
										</div>
									</TabContent>
								</div>
							</svelte:fragment>
						</Tabs>
					</down>
				</VSplitPane>
			</div>
		{/if}
	</FlowCard>
</div>
