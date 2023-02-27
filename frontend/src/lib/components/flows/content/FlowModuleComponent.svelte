<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Editor from '$lib/components/Editor.svelte'
	import EditorBar from '$lib/components/EditorBar.svelte'
	import ModulePreview from '$lib/components/ModulePreview.svelte'
	import { createScriptFromInlineScript, fork } from '$lib/components/flows/flowStateUtils'
	import { flowStore } from '$lib/components/flows/flowStore'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { RawScript, type FlowModule, type PathFlow, type PathScript } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowModuleHeader from './FlowModuleHeader.svelte'
	import { flowStateStore } from '../flowState'
	import { schemaToObject, scriptLangToEditorLang } from '$lib/utils'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import { afterUpdate, getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { loadSchemaFromModule } from '../utils'
	import FlowModuleScript from './FlowModuleScript.svelte'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	import FlowRetries from './FlowRetries.svelte'
	import { getStepPropPicker } from '../previousResults'
	import { deepEqual } from 'fast-equals'

	import Button from '$lib/components/common/button/Button.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import FlowModuleSleep from './FlowModuleSleep.svelte'
	import FlowPathViewer from './FlowPathViewer.svelte'

	const { selectedId, previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')

	export let flowModule: FlowModule
	export let failureModule: boolean = false

	export let parentModule: FlowModule | undefined = undefined
	export let previousModule: FlowModule | undefined

	let value = flowModule.value as PathFlow | RawScript | PathScript
	$: value = flowModule.value as PathFlow | RawScript | PathScript

	let editor: Editor
	let modulePreview: ModulePreview
	let websocketAlive = { pyright: false, black: false, deno: false, go: false }
	let selected = 'inputs'
	let wrapper: HTMLDivElement
	let panes: HTMLElement
	let totalTopGap = 0
	let validCode = true
	let width = 1200

	let inputTransforms: Record<string, any> = value.input_transforms

	$: value.input_transforms = inputTransforms

	$: stepPropPicker = failureModule
		? {
				pickableProperties: {
					flow_input: schemaToObject($flowStore.schema, $previewArgs),
					priorIds: {},
					previousId: undefined,
					hasResume: false
				},
				extraLib: ''
		  }
		: getStepPropPicker(
				$flowStateStore,
				parentModule,
				previousModule,
				flowModule.id,
				$flowStore,
				$previewArgs,
				false
		  )

	function onKeyDown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key == 'Enter') {
			event.preventDefault()
			selected = 'test'
			modulePreview?.runTestWithStepArgs()
		}
	}

	async function reload(flowModule: FlowModule) {
		try {
			const { input_transforms, schema } = await loadSchemaFromModule(flowModule)
			validCode = true
			setTimeout(() => {
				if (!deepEqual(value.input_transforms, input_transforms)) {
					inputTransforms = input_transforms
				}
			})

			if (!deepEqual(schema, $flowStateStore[flowModule.id]?.schema)) {
				if (!$flowStateStore[flowModule.id]) {
					$flowStateStore[flowModule.id] = { schema }
				} else {
					$flowStateStore[flowModule.id].schema = schema
				}
			}
		} catch (e) {
			validCode = false
		}
	}

	afterUpdate(() => {
		totalTopGap = 0
		if (!(wrapper && panes)) return

		const wrapperTop = wrapper.getBoundingClientRect().top
		const panesTop = panes.getBoundingClientRect().top
		totalTopGap = panesTop - wrapperTop
	})

	let isScript = true
	$: isScript != (value.type === 'script') && (isScript = value.type === 'script')
</script>

<svelte:window on:keydown={onKeyDown} />

{previousModule?.id}
{#if value}
	<div class="h-full" bind:this={wrapper} bind:clientWidth={width}>
		<FlowCard bind:flowModule>
			<svelte:fragment slot="header">
				<FlowModuleHeader
					bind:module={flowModule}
					on:toggleSuspend={() => (selected = 'advanced-suspend')}
					on:toggleSleep={() => (selected = 'advanced-sleep')}
					on:toggleRetry={() => (selected = 'advanced-retries')}
					on:toggleStopAfterIf={() => (selected = 'advanced-early-stop')}
					on:fork={async () => {
						const [module, state] = await fork(flowModule)
						flowModule = module
						$flowStateStore[module.id] = state
					}}
					on:createScriptFromInlineScript={async () => {
						const [module, state] = await createScriptFromInlineScript(
							flowModule,
							$selectedId,
							$flowStateStore[flowModule.id].schema
						)
						flowModule = module
						$flowStateStore[module.id] = state
					}}
				/>
			</svelte:fragment>

			{#if value.type === 'rawscript'}
				<div class="border-b-2 shadow-sm px-1">
					<EditorBar
						{validCode}
						{editor}
						lang={value['language'] ?? 'deno'}
						{websocketAlive}
						iconOnly={width < 768}
					/>
				</div>
			{/if}

			<div
				bind:this={panes}
				class="h-full"
				style="max-height: calc(100% - {totalTopGap}px) !important;"
			>
				<Splitpanes horizontal>
					<Pane size={isScript ? 30 : 50} minSize={20}>
						{#if value.type === 'rawscript'}
							<div class="h-full">
								<Editor
									bind:websocketAlive
									bind:this={editor}
									class="h-full px-2"
									bind:code={value.content}
									deno={value.language === RawScript.language.DENO}
									lang={scriptLangToEditorLang(value.language)}
									automaticLayout={true}
									cmdEnterAction={async () => {
										selected = 'test'
										if (value.type === 'rawscript') {
											value.content = editor.getCode()
										}
										await reload(flowModule)
										modulePreview?.runTestWithStepArgs()
									}}
									on:change={async (event) => {
										if (flowModule.value.type === 'rawscript') {
											flowModule.value.content = event.detail
										}
										await reload(flowModule)
									}}
									formatAction={() => reload(flowModule)}
								/>
							</div>
						{:else if value.type === 'script'}
							<FlowModuleScript path={value.path} hash={value.hash} />
						{:else if value.type === 'flow'}
							<FlowPathViewer path={value.path} />
						{/if}
					</Pane>
					<Pane size={isScript ? 70 : 50} minSize={20}>
						<Tabs bind:selected>
							<Tab value="inputs"><span class="font-semibold">Step Input</span></Tab>
							<Tab value="test"><span class="font-semibold text-md">Test this step</span></Tab>
							<Tab value="advanced-retries">Advanced</Tab>
						</Tabs>
						<div class="h-[calc(100%-32px)]">
							{#if selected === 'inputs'}
								<div class="h-full overflow-auto">
									<PropPickerWrapper
										pickableProperties={stepPropPicker.pickableProperties}
										error={failureModule}
									>
										<SchemaForm
											schema={$flowStateStore[$selectedId]?.schema ?? {}}
											inputTransform={true}
											previousModuleId={previousModule?.id}
											bind:args={value.input_transforms}
											bind:extraLib={stepPropPicker.extraLib}
										/>
									</PropPickerWrapper>
								</div>
							{:else if selected === 'test'}
								<ModulePreview
									bind:this={modulePreview}
									mod={flowModule}
									schema={$flowStateStore[$selectedId]?.schema ?? {}}
								/>
							{:else if selected.startsWith('advanced')}
								<Tabs bind:selected>
									<Tab value="advanced-retries">Retries</Tab>
									{#if !$selectedId.includes('failure')}
										<Tab value="advanced-early-stop">Early Stop/Break</Tab>
										<Tab value="advanced-suspend">Suspend</Tab>
										<Tab value="advanced-sleep">Sleep</Tab>
										<Tab value="advanced-same_worker">Shared Directory</Tab>
									{/if}
								</Tabs>
								{#if selected === 'advanced-retries'}
									<FlowRetries bind:flowModule class="px-4 pb-4 h-full overflow-auto" />
								{:else if selected === 'advanced-early-stop'}
									<FlowModuleEarlyStop bind:flowModule class="px-4 pb-4 h-full overflow-auto" />
								{:else if selected === 'advanced-suspend'}
									<div class="px-4 pb-4 h-full overflow-auto">
										<FlowModuleSuspend bind:flowModule />
									</div>
								{:else if selected === 'advanced-sleep'}
									<div class="px-4 pb-4 h-full overflow-auto">
										<FlowModuleSleep previousModuleId={previousModule?.id} bind:flowModule />
									</div>
								{:else if selected === 'advanced-same_worker'}
									<div class="p-4  h-full overflow-auto">
										<Alert type="info" title="Share a directory between steps">
											If shared directory is set, will share a folder that will be mounted on
											`./shared` for each of them to pass data between each other.
										</Alert>
										<Button
											btnClasses="mt-4"
											on:click={() => {
												$selectedId = 'settings-same-worker'
											}}>Set shared directory in the flow settings</Button
										>
									</div>
								{/if}
							{/if}
						</div>
					</Pane>
				</Splitpanes>
			</div>
		</FlowCard>
	</div>
{:else}
	Incorrect flow module type
{/if}
