<script lang="ts">
	import type { Schema } from '$lib/common'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Editor from '$lib/components/Editor.svelte'
	import EditorBar from '$lib/components/EditorBar.svelte'
	import FlowPreview from '$lib/components/FlowPreview.svelte'
	import FlowInputs from '$lib/components/flows/FlowInputs.svelte'
	import { flowStateStore, type FlowModuleSchema } from '$lib/components/flows/flowState'
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
	import { VSplitPane } from 'svelte-split-pane'

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

<FlowCard title="TODO title">
	<svelte:fragment slot="header">
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
			<div class="border-b p-2">
				<EditorBar {editor} {websocketAlive} lang={flowModule.value.language ?? 'deno'} />
			</div>
		{/if}
		<VSplitPane topPanelSize="50%" downPanelSize="50%">
			<top slot="top">
				{#if flowModule.value.type === 'rawscript'}
					<div on:mouseleave={() => reload(flowModule)} class="h-full">
						<Editor
							bind:websocketAlive
							bind:this={editor}
							class="h-full"
							bind:code={flowModule.value.content}
							deno={flowModule.value.language === RawScript.language.DENO}
							automaticLayout={true}
							formatAction={() => reload(flowModule)}
						/>
					</div>
				{/if}
			</top>

			<down slot="down" class="flex flex-col h-full overflow-auto">
				<div class="h-1">
					<Tabs selected="preview">
						<Tab value="inputs">Inputs</Tab>
						<Tab value="preview">Preview</Tab>

						<svelte:fragment slot="content">
							<TabContent value="inputs">
								<SchemaForm
									{schema}
									inputTransform={true}
									importPath={indexes}
									bind:pickableProperties={stepPropPicker.pickableProperties}
									bind:args={flowModule.input_transforms}
									bind:extraLib={stepPropPicker.extraLib}
								/>
							</TabContent>
							<TabContent value="preview">
								<FlowPreview flow={$flowStore} {i} {schema} />
							</TabContent>
						</svelte:fragment>
					</Tabs>
				</div>
			</down>
		</VSplitPane>
	{/if}
</FlowCard>
