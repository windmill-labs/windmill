<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Editor from '$lib/components/Editor.svelte'
	import EditorBar from '$lib/components/EditorBar.svelte'
	import ModulePreview from '$lib/components/ModulePreview.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { createScriptFromInlineScript, fork } from '$lib/components/flows/flowStateUtils'

	import type { FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowModuleHeader from './FlowModuleHeader.svelte'
	import { getLatestHashForScript, scriptLangToEditorLang } from '$lib/scripts'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import { afterUpdate, getContext, tick } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import FlowModuleScript from './FlowModuleScript.svelte'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	import FlowModuleCache from './FlowModuleCache.svelte'
	import FlowModuleDeleteAfterUse from './FlowModuleDeleteAfterUse.svelte'
	import FlowRetries from './FlowRetries.svelte'
	import { getStepPropPicker } from '../previousResults'
	import { deepEqual } from 'fast-equals'
	import Section from '$lib/components/Section.svelte'

	import Button from '$lib/components/common/button/Button.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import FlowModuleSleep from './FlowModuleSleep.svelte'
	import FlowPathViewer from './FlowPathViewer.svelte'
	import InputTransformSchemaForm from '$lib/components/InputTransformSchemaForm.svelte'
	import { schemaToObject } from '$lib/schema'
	import FlowModuleMock from './FlowModuleMock.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { SecondsInput } from '$lib/components/common'
	import DiffEditor from '$lib/components/DiffEditor.svelte'
	import FlowModuleTimeout from './FlowModuleTimeout.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import s3Scripts from './s3Scripts/lib'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import Label from '$lib/components/Label.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { loadSchemaFromModule } from '../flowInfers'

	const { selectedId, previewArgs, flowStateStore, flowStore, pathStore, saveDraft } =
		getContext<FlowEditorContext>('FlowEditorContext')

	export let flowModule: FlowModule
	export let failureModule: boolean = false
	export let parentModule: FlowModule | undefined = undefined
	export let previousModule: FlowModule | undefined
	export let scriptKind: 'script' | 'trigger' | 'approval' = 'script'
	export let scriptTemplate: 'pgsql' | 'mysql' | 'script' | 'docker' | 'powershell' = 'script'
	export let noEditor: boolean

	let editor: Editor
	let diffEditor: DiffEditor
	let modulePreview: ModulePreview
	let websocketAlive = {
		pyright: false,
		black: false,
		deno: false,
		go: false,
		ruff: false,
		shellcheck: false
	}
	let selected = 'inputs'
	let advancedSelected = 'retries'
	let advancedRuntimeSelected = 'concurrency'
	let s3Kind = 's3_client'
	let wrapper: HTMLDivElement
	let panes: HTMLElement
	let totalTopGap = 0
	let validCode = true
	let width = 1200

	const { modulesStore: copilotModulesStore } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}

	function setCopilotModuleEditor() {
		copilotModulesStore?.update((modules) => {
			const module = modules.find((m) => m.id === flowModule.id)
			if (module) {
				module.editor = editor
			}
			return modules
		})
	}

	$: editor !== undefined && setCopilotModuleEditor()

	$: stepPropPicker = failureModule
		? {
				pickableProperties: {
					flow_input: schemaToObject($flowStore.schema as any, $previewArgs),
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
	let inputTransformSchemaForm: InputTransformSchemaForm | undefined = undefined
	async function reload(flowModule: FlowModule) {
		try {
			const { input_transforms, schema } = await loadSchemaFromModule(flowModule)
			validCode = true
			if (inputTransformSchemaForm) {
				inputTransformSchemaForm.setArgs(input_transforms)
			} else {
				if (
					flowModule.value.type == 'rawscript' ||
					flowModule.value.type == 'script' ||
					flowModule.value.type == 'flow'
				) {
					flowModule.value.input_transforms = input_transforms
				}
			}

			if (flowModule.value.type == 'rawscript' && flowModule.value.lock != undefined) {
				flowModule.value.lock = undefined
			}
			await tick()
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

	function selectAdvanced(subtab: string) {
		selected = 'advanced'
		advancedSelected = subtab
	}

	afterUpdate(() => {
		totalTopGap = 0
		if (!(wrapper && panes)) return

		const wrapperTop = wrapper.getBoundingClientRect().top
		const panesTop = panes.getBoundingClientRect().top
		totalTopGap = panesTop - wrapperTop
	})

	let forceReload = 0

	let editorPanelSize = noEditor ? 0 : flowModule.value.type == 'script' ? 30 : 50
	let editorSettingsPanelSize = 100 - editorPanelSize
</script>

<svelte:window on:keydown={onKeyDown} />

{#if flowModule.value}
	<div class="h-full" bind:this={wrapper} bind:clientWidth={width}>
		<FlowCard
			on:reload={() => {
				forceReload++
				reload(flowModule)
			}}
			{noEditor}
			bind:flowModule
		>
			<svelte:fragment slot="header">
				<FlowModuleHeader
					bind:module={flowModule}
					on:toggleSuspend={() => selectAdvanced('suspend')}
					on:toggleSleep={() => selectAdvanced('sleep')}
					on:toggleMock={() => selectAdvanced('mock')}
					on:toggleRetry={() => selectAdvanced('retries')}
					on:toggleConcurrency={() => selectAdvanced('runtime')}
					on:toggleCache={() => selectAdvanced('cache')}
					on:toggleStopAfterIf={() => selectAdvanced('early-stop')}
					on:fork={async () => {
						const [module, state] = await fork(flowModule)
						flowModule = module
						$flowStateStore[module.id] = state
					}}
					on:reload={async () => {
						if (flowModule.value.type == 'script') {
							if (flowModule.value.hash != undefined) {
								flowModule.value.hash = await getLatestHashForScript(flowModule.value.path)
							}
							forceReload++
							await reload(flowModule)
						}
					}}
					on:createScriptFromInlineScript={async () => {
						const [module, state] = await createScriptFromInlineScript(
							flowModule,
							$selectedId,
							$flowStateStore[flowModule.id].schema,
							$flowStore,
							$pathStore
						)
						flowModule = module
						$flowStateStore[module.id] = state
					}}
				/>
			</svelte:fragment>

			{#if flowModule.value.type === 'rawscript' && !noEditor}
				<div class="border-b-2 shadow-sm px-1">
					<EditorBar
						{validCode}
						{editor}
						{diffEditor}
						lang={flowModule.value['language'] ?? 'deno'}
						{websocketAlive}
						iconOnly={width < 850}
						kind={scriptKind}
						template={scriptTemplate}
						args={Object.entries(flowModule.value.input_transforms).reduce((acc, [key, obj]) => {
							acc[key] = obj.type === 'static' ? obj.value : undefined
							return acc
						}, {})}
					/>
				</div>
			{/if}

			<div
				bind:this={panes}
				class="h-full"
				style="max-height: calc(100% - {totalTopGap}px) !important;"
				id="flow-editor-editor"
			>
				<Splitpanes horizontal>
					<Pane bind:size={editorPanelSize} minSize={20}>
						{#if flowModule.value.type === 'rawscript'}
							{#if !noEditor}
								{#key flowModule.id}
									<Editor
										folding
										path={flowModule.value.path}
										bind:websocketAlive
										bind:this={editor}
										class="h-full relative"
										bind:code={flowModule.value.content}
										lang={scriptLangToEditorLang(flowModule.value.language)}
										scriptLang={flowModule.value.language}
										automaticLayout={true}
										cmdEnterAction={async () => {
											selected = 'test'
											if ($selectedId == flowModule.id) {
												if (flowModule.value.type === 'rawscript') {
													flowModule.value.content = editor.getCode()
												}
												await reload(flowModule)
												modulePreview?.runTestWithStepArgs()
											}
										}}
										on:change={async (event) => {
											if (flowModule.value.type === 'rawscript') {
												flowModule.value.content = event.detail
											}
											await reload(flowModule)
										}}
										formatAction={() => {
											reload(flowModule)
											saveDraft()
										}}
										fixedOverflowWidgets={true}
										args={Object.entries(flowModule.value.input_transforms).reduce(
											(acc, [key, obj]) => {
												acc[key] = obj.type === 'static' ? obj.value : undefined
												return acc
											},
											{}
										)}
									/>
									<DiffEditor
										bind:this={diffEditor}
										automaticLayout
										fixedOverflowWidgets
										class="hidden h-full"
									/>
								{/key}
							{/if}
						{:else if flowModule.value.type === 'script'}
							{#if !noEditor}
								<div class="border-t">
									{#key forceReload}
										<FlowModuleScript path={flowModule.value.path} hash={flowModule.value.hash} />
									{/key}
								</div>
							{/if}
						{:else if flowModule.value.type === 'flow'}
							<FlowPathViewer path={flowModule.value.path} />
						{/if}
					</Pane>
					<Pane bind:size={editorSettingsPanelSize} minSize={20}>
						<Tabs bind:selected>
							<Tab value="inputs">Step Input</Tab>
							<Tab value="test">Test this step</Tab>
							<Tab value="advanced">Advanced</Tab>
						</Tabs>
						<div
							class={advancedSelected === 'runtime' ? 'h-[calc(100%-64px)]' : 'h-[calc(100%-32px)]'}
						>
							{#if selected === 'inputs' && (flowModule.value.type == 'rawscript' || flowModule.value.type == 'script' || flowModule.value.type == 'flow')}
								<div class="h-full overflow-auto" id="flow-editor-step-input">
									<PropPickerWrapper
										pickableProperties={stepPropPicker.pickableProperties}
										error={failureModule}
									>
										<InputTransformSchemaForm
											bind:this={inputTransformSchemaForm}
											pickableProperties={stepPropPicker.pickableProperties}
											schema={$flowStateStore[$selectedId]?.schema ?? {}}
											previousModuleId={previousModule?.id}
											bind:args={flowModule.value.input_transforms}
											extraLib={stepPropPicker.extraLib}
											enableAi
										/>
									</PropPickerWrapper>
								</div>
							{:else if selected === 'test'}
								<ModulePreview
									pickableProperties={stepPropPicker.pickableProperties}
									bind:this={modulePreview}
									mod={flowModule}
									{editor}
									{diffEditor}
									lang={flowModule.value['language'] ?? 'deno'}
									schema={$flowStateStore[$selectedId]?.schema ?? {}}
								/>
							{:else if selected === 'advanced'}
								<Tabs bind:selected={advancedSelected}>
									<Tab value="retries" active={flowModule.retry !== undefined}>Retries</Tab>
									{#if !$selectedId.includes('failure')}
										<Tab value="runtime">Runtime</Tab>
										<Tab value="cache" active={Boolean(flowModule.cache_ttl)}>Cache</Tab>
										<Tab value="early-stop" active={Boolean(flowModule.stop_after_if)}>
											Early Stop
										</Tab>
										<Tab value="suspend" active={Boolean(flowModule.suspend)}>Suspend</Tab>
										<Tab value="sleep" active={Boolean(flowModule.sleep)}>Sleep</Tab>
										<Tab value="mock" active={Boolean(flowModule.mock?.enabled)}>Mock</Tab>
										<Tab value="same_worker">Shared Directory</Tab>
										{#if flowModule.value['language'] === 'python3' || flowModule.value['language'] === 'deno'}
											<Tab value="s3">S3</Tab>
										{/if}
									{/if}
								</Tabs>
								{#if advancedSelected === 'runtime'}
									<Tabs bind:selected={advancedRuntimeSelected}>
										<Tab value="concurrency">Concurrency</Tab>
										<Tab value="timeout">Timeout</Tab>
										<Tab value="priority">Priority</Tab>
										<Tab value="lifetime">Lifetime</Tab>
									</Tabs>
								{/if}
								<div class="h-[calc(100%-32px)] overflow-auto p-4">
									{#if advancedSelected === 'retries'}
										<Section label="Retries">
											<svelte:fragment slot="header">
												<Tooltip documentationLink="https://www.windmill.dev/docs/flows/retries">
													If defined, upon error this step will be retried with a delay and a
													maximum number of attempts as defined below.
												</Tooltip>
											</svelte:fragment>
											<FlowRetries bind:flowModuleRetry={flowModule.retry} />
										</Section>
									{:else if advancedSelected === 'runtime' && advancedRuntimeSelected === 'concurrency'}
										<Section label="Concurrency Limits" class="flex flex-col gap-4" eeOnly>
											<svelte:fragment slot="header">
												<Tooltip>Allowed concurrency within a given timeframe</Tooltip>
											</svelte:fragment>
											{#if flowModule.value.type == 'rawscript'}
												<Label label="Max number of executions within the time window">
													<div class="flex flex-row gap-2 max-w-sm">
														<input
															disabled={!$enterpriseLicense}
															bind:value={flowModule.value.concurrent_limit}
															type="number"
														/>
														<Button
															size="xs"
															color="light"
															variant="border"
															on:click={() => {
																if (flowModule.value.type == 'rawscript') {
																	flowModule.value.concurrent_limit = undefined
																}
															}}
														>
															<div class="flex flex-row gap-2"> Remove Limits </div>
														</Button>
													</div>
												</Label>
												<Label label="Time window in seconds">
													<SecondsInput
														disabled={!$enterpriseLicense}
														bind:seconds={flowModule.value.concurrency_time_window_s}
													/>
												</Label>
											{:else}
												<Alert type="warning" title="Limitation" size="xs">
													The concurrency limit of a workspace script is only settable in the script
													metadata itself. For hub scripts, this feature is non available yet.
												</Alert>
											{/if}
										</Section>
									{:else if advancedSelected === 'runtime' && advancedRuntimeSelected === 'timeout'}
										<div>
											<FlowModuleTimeout bind:flowModule />
										</div>
									{:else if advancedSelected === 'runtime' && advancedRuntimeSelected === 'priority'}
										<Section label="Priority" class="flex flex-col gap-4">
											<!-- TODO: Add EE-only badge when we have it -->
											<Toggle
												disabled={!$enterpriseLicense || isCloudHosted()}
												checked={flowModule.priority !== undefined && flowModule.priority > 0}
												on:change={() => {
													if (flowModule.priority) {
														flowModule.priority = undefined
													} else {
														flowModule.priority = 100
													}
												}}
												options={{
													right: 'Enabled high priority flow step',
													rightTooltip: `Jobs scheduled from this step when the flow is executed are labeled as high priority and take precedence over the other jobs in the jobs queue. ${
														!$enterpriseLicense
															? 'This is a feature only available on enterprise edition.'
															: ''
													}`
												}}
											/>
											<Label label="Priority number">
												<svelte:fragment slot="header">
													<Tooltip>The higher the number, the higher the priority.</Tooltip>
												</svelte:fragment>
												<input
													type="number"
													class="!w-24"
													disabled={flowModule.priority === undefined}
													bind:value={flowModule.priority}
													on:focus
													on:change={() => {
														if (flowModule.priority && flowModule.priority > 100) {
															flowModule.priority = 100
														} else if (flowModule.priority && flowModule.priority < 0) {
															flowModule.priority = 0
														}
													}}
												/>
											</Label>

											<Alert type="warning" title="Limitation" size="xs">
												Setting priority is only available for enterprise edition and not available
												on the cloud.
											</Alert>
										</Section>
									{:else if advancedSelected === 'runtime' && advancedRuntimeSelected === 'lifetime'}
										<div>
											<FlowModuleDeleteAfterUse bind:flowModule disabled={!$enterpriseLicense} />
										</div>
									{:else if advancedSelected === 'cache'}
										<div>
											<FlowModuleCache bind:flowModule />
										</div>
									{:else if advancedSelected === 'early-stop'}
										<FlowModuleEarlyStop bind:flowModule />
									{:else if advancedSelected === 'suspend'}
										<div>
											<FlowModuleSuspend previousModuleId={previousModule?.id} bind:flowModule />
										</div>
									{:else if advancedSelected === 'sleep'}
										<div>
											<FlowModuleSleep previousModuleId={previousModule?.id} bind:flowModule />
										</div>
									{:else if advancedSelected === 'mock'}
										<div>
											<FlowModuleMock bind:flowModule />
										</div>
									{:else if advancedSelected === 'same_worker'}
										<div>
											<Alert type="info" title="Share a directory between steps">
												If shared directory is set, will share a folder that will be mounted on
												`./shared` for each of them to pass data between each other.
											</Alert>
											<Button
												btnClasses="mt-4"
												on:click={() => {
													$selectedId = 'settings-same-worker'
												}}
											>
												Set shared directory in the flow settings
											</Button>
										</div>
									{:else if advancedSelected === 's3'}
										<div>
											<h2 class="pb-4">
												S3 snippets
												<Tooltip>
													Read/Write object from/to S3 and leverage Polars and DuckDB to run
													efficient ETL processes.
												</Tooltip>
											</h2>
										</div>
										<div class="flex gap-2 justify-between mb-4 items-center">
											<div class="flex gap-2">
												<ToggleButtonGroup bind:selected={s3Kind} class="w-auto">
													{#if flowModule.value['language'] === 'deno'}
														<ToggleButton value="s3_client" size="sm" label="S3 lite client" />
													{:else}
														<ToggleButton value="s3_client" size="sm" label="Boto3" />
														<ToggleButton value="polars" size="sm" label="Polars" />
														<ToggleButton value="duckdb" size="sm" label="DuckDB" />
													{/if}
												</ToggleButtonGroup>
											</div>

											<Button
												size="xs"
												on:click={() =>
													editor.setCode(s3Scripts[flowModule.value['language']][s3Kind])}
											>
												Apply snippet
											</Button>
										</div>
										<HighlightCode
											language={flowModule.value['language']}
											code={s3Scripts[flowModule.value['language']][s3Kind]}
										/>
									{/if}
								</div>
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
