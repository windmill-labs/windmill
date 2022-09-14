<script lang="ts">
	import { VSplitPane } from 'svelte-split-pane'

	import type { Schema } from '$lib/common'
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

	import { RawScript, type FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowModuleHeader from './FlowModuleHeader.svelte'
	import { flowStateStore, type FlowModuleSchema } from '../flowState'
	import { scriptLangToEditorLang } from '$lib/utils'

	export let indexes: string
	export let flowModule: FlowModule
	export let args: Record<string, any> = {}
	export let schema: Schema
	export let childFlowModules: FlowModuleSchema[] | undefined = undefined

	const [i] = indexes.split('-').map(Number)

	let editor: Editor
	let websocketAlive = { pyright: false, black: false, deno: false }

	$: shouldPick = isEmptyFlowModule(flowModule)
	$: stepPropPicker = getStepPropPicker(
		indexes.split('-').map(Number),
		$flowStore.schema,
		$flowStateStore,
		args
	)

	async function apply<T>(fn: (arg: T) => Promise<FlowModuleSchema>, arg: T) {
		const flowModuleSchema = await fn(arg)

		flowModule = flowModuleSchema.flowModule
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
							suffix: indexes,
							schema
						})
					}}
				/>
			</div>
		</svelte:fragment>
		{#if shouldPick}
			<FlowInputs
				shouldDisableTriggerScripts={i != 0}
				shouldDisableLoopCreation={indexes.length > 1 || i == 0}
				on:loop={() => applyCreateLoop()}
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

					<down slot="down">
						<Tabs selected="inputs">
							<Tab value="inputs">Inputs</Tab>
							<Tab value="preview">Preview</Tab>

							<svelte:fragment slot="content">
								<div class="h-full pb-16 overflow-y-scroll bg-white">
									<div class="p-4 overflow-hidden">
										<TabContent value="inputs">
											<div class="w-2/3">
												<SchemaForm
													{schema}
													inputTransform={true}
													importPath={indexes}
													bind:pickableProperties={stepPropPicker.pickableProperties}
													bind:args={flowModule.input_transforms}
													bind:extraLib={stepPropPicker.extraLib}
												/>
											</div>
										</TabContent>
										<TabContent value="preview">
											<FlowPreview flow={$flowStore} {i} {schema} />
										</TabContent>
										<TabContent value="settings">
											{#if ('path' in flowModule.value && flowModule.value.path) || ('language' in flowModule.value && flowModule.value.language)}
												<input
													on:click|stopPropagation={() => undefined}
													class="overflow-x-auto"
													type="text"
													bind:value={flowModule.summary}
													placeholder="Summary"
												/>
											{/if}
										</TabContent>
									</div>
								</div>
							</svelte:fragment>
						</Tabs>
					</down>
				</VSplitPane>
			</div>
		{/if}
	</FlowCard>
</div>
