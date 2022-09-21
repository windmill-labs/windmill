<script lang="ts">
	import { VSplitPane } from 'svelte-split-pane'

	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Editor from '$lib/components/Editor.svelte'
	import EditorBar from '$lib/components/EditorBar.svelte'
	import FlowPreview from '$lib/components/FlowPreview.svelte'
	import FlowInputs from './FlowInputs.svelte'
	import {
		createInlineScriptModule,
		createLoop,
		createScriptFromInlineScript,
		fork,
		getStepPropPicker,
		isEmptyFlowModule,
		loadFlowModuleSchema,
		pickScript
	} from '$lib/components/flows/flowStateUtils'
	import { flowStore } from '$lib/components/flows/flowStore'
	import SchemaForm from '$lib/components/SchemaForm.svelte'

	import { RawScript, type Flow, type FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowModuleHeader from './FlowModuleHeader.svelte'
	import { flowStateStore, type FlowModuleState } from '../flowState'
	import { scriptLangToEditorLang } from '$lib/utils'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import FlowModuleAdvancedSettings from './FlowModuleAdvancedSettings.svelte'

	const { selectedId, select } = getContext<FlowEditorContext>('FlowEditorContext')

	export let flowModule: FlowModule
	export let args: Record<string, any> = {}
	export let flowModuleState: FlowModuleState

	$: [parentIndex, childIndex] = $selectedId.split('-').map(Number)

	let editor: Editor
	let websocketAlive = { pyright: false, black: false, deno: false }

	$: shouldPick = isEmptyFlowModule(flowModule)
	$: stepPropPicker = getStepPropPicker(
		$selectedId.split('-').map(Number),
		$flowStore.schema,
		$flowStateStore,
		args
	)

	async function apply<T>(fn: (arg: T) => Promise<[FlowModule, FlowModuleState]>, arg: T) {
		const [module, moduleState] = await fn(arg)

		flowModule = module
		flowModuleState = moduleState
	}

	async function applyState<T>(fn: (arg: T) => Promise<FlowModuleState>, arg: T) {
		flowModuleState = await fn(arg)
	}

	async function reload(flowModule: FlowModule) {
		applyState(loadFlowModuleSchema, flowModule)
	}

	async function applyCreateLoop() {
		await apply(createLoop, null)
	}
</script>

<div class="flex flex-col h-full ">
	<FlowCard {flowModule}>
		<svelte:fragment slot="header">
			<div class="flex-shrink-0">
				<FlowModuleHeader
					bind:module={flowModule}
					on:delete
					on:fork={() => apply(fork, flowModule)}
					on:createScriptFromInlineScript={() => {
						apply(createScriptFromInlineScript, {
							flowModule: flowModule,
							suffix: $selectedId,
							schema: flowModuleState.schema
						})
					}}
				/>
			</div>
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
			/>
		{:else}
			{#if flowModule.value.type === 'rawscript'}
				<div class="flex-shrink-0 border-b p-1">
					<EditorBar {editor} lang={flowModule.value['language'] ?? 'deno'} {websocketAlive} />
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
									formatAction={() => reload(flowModule)}
								/>
							</div>
						{/if}
					</top>

					<down slot="down" class="flex flex-col flex-1 h-full">
						<Tabs selected="inputs">
							<Tab value="inputs">Inputs</Tab>
							<Tab value="test">Test</Tab>
							{#if !$selectedId.includes('failure')}
								<Tab value="advanced">Advanced</Tab>
							{/if}

							<svelte:fragment slot="content">
								<div class="overflow-hidden bg-white" style="height:calc(100% - 32px);">
									<TabContent value="inputs" class="flex flex-col flex-1 h-full">
										<PropPickerWrapper bind:pickableProperties={stepPropPicker.pickableProperties}>
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
									<TabContent value="test" class="flex flex-col flex-1 h-full">
										<div class="p-4 overflow-y-auto">
											<FlowPreview indexes={$selectedId} schema={flowModuleState.schema} />
										</div>
									</TabContent>

									<TabContent value="advanced" class="flex flex-col flex-1 h-full">
										<div class="p-4 overflow-y-auto">
											<FlowModuleAdvancedSettings bind:flowModule />
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
