<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Editor from '$lib/components/Editor.svelte'
	import EditorBar from '$lib/components/EditorBar.svelte'
	import ModulePreview from '$lib/components/ModulePreview.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { createScriptFromInlineScript, fork } from '$lib/components/flows/flowStateUtils.svelte'

	import type { FlowModule, RawScript } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowModuleHeader from './FlowModuleHeader.svelte'
	import { getLatestHashForScript, scriptLangToEditorLang } from '$lib/scripts'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import { getContext, onDestroy, tick, untrack } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import FlowModuleScript from './FlowModuleScript.svelte'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	import FlowModuleCache from './FlowModuleCache.svelte'
	import FlowModuleDeleteAfterUse from './FlowModuleDeleteAfterUse.svelte'
	import FlowRetries from './FlowRetries.svelte'
	import { getFailureStepPropPicker, getStepPropPicker } from '../previousResults'
	import { deepEqual } from 'fast-equals'
	import Section from '$lib/components/Section.svelte'

	import Button from '$lib/components/common/button/Button.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import FlowModuleSleep from './FlowModuleSleep.svelte'
	import FlowPathViewer from './FlowPathViewer.svelte'
	import InputTransformSchemaForm from '$lib/components/InputTransformSchemaForm.svelte'
	import FlowModuleMockTransitionMessage from './FlowModuleMockTransitionMessage.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { SecondsInput } from '$lib/components/common'
	import DiffEditor from '$lib/components/DiffEditor.svelte'
	import FlowModuleTimeout from './FlowModuleTimeout.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import s3Scripts from './s3Scripts/lib'
	import Label from '$lib/components/Label.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { loadSchemaFromModule } from '../flowInfers'
	import FlowModuleSkip from './FlowModuleSkip.svelte'
	import { type Job, JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { checkIfParentLoop } from '../utils'
	import ModulePreviewResultViewer from '$lib/components/ModulePreviewResultViewer.svelte'
	import { aiChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import { refreshStateStore, usePromise } from '$lib/svelte5Utils.svelte'
	import { getStepHistoryLoaderContext } from '$lib/components/stepHistoryLoader.svelte'
	import AssetsDropdownButton from '$lib/components/assets/AssetsDropdownButton.svelte'
	import { inferAssets } from '$lib/infer'
	import { assetEq } from '$lib/components/assets/lib'

	const {
		selectedId,
		currentEditor,
		previewArgs,
		flowStateStore,
		flowStore,
		pathStore,
		saveDraft,
		customUi,
		executionCount
	} = getContext<FlowEditorContext>('FlowEditorContext')

	interface Props {
		flowModule: FlowModule
		failureModule?: boolean
		preprocessorModule?: boolean
		parentModule?: FlowModule | undefined
		previousModule: FlowModule | undefined
		scriptKind?: 'script' | 'trigger' | 'approval'
		scriptTemplate?: 'pgsql' | 'mysql' | 'script' | 'docker' | 'powershell'
		noEditor: boolean
		enableAi: boolean
		savedModule?: FlowModule | undefined
		forceTestTab?: boolean
		highlightArg?: string
	}

	let {
		flowModule = $bindable(),
		failureModule = false,
		preprocessorModule = false,
		parentModule = $bindable(),
		previousModule,
		scriptKind = 'script',
		scriptTemplate = 'script',
		noEditor,
		enableAi,
		savedModule = undefined,
		forceTestTab = false,
		highlightArg = undefined
	}: Props = $props()

	let tag: string | undefined = $state(undefined)
	let diffMode = $state(false)

	let editor: Editor | undefined = $state()
	let diffEditor: DiffEditor | undefined = $state()
	let modulePreview: ModulePreview | undefined = $state()
	let websocketAlive = $state({
		pyright: false,
		deno: false,
		go: false,
		ruff: false,
		shellcheck: false
	})
	let selected = $state(preprocessorModule ? 'test' : 'inputs')
	let advancedSelected = $state('retries')
	let advancedRuntimeSelected = $state('concurrency')
	let s3Kind = $state('s3_client')
	let validCode = $state(true)
	let width = $state(1200)
	let lastJob: Job | undefined = $state(undefined)
	let testJob: Job | undefined = $state(undefined)
	let testIsLoading = $state(false)
	let scriptProgress = $state(undefined)

	function onModulesChange(savedModule: FlowModule | undefined, flowModule: FlowModule) {
		// console.log('onModulesChange', savedModule, flowModule)
		return savedModule?.value?.type === 'rawscript' &&
			flowModule.value.type === 'rawscript' &&
			savedModule.value.content !== flowModule.value.content
			? savedModule.value.content
			: undefined
	}

	function onKeyDown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key == 'Enter') {
			event.preventDefault()
			selected = 'test'
			modulePreview?.runTestWithStepArgs()
		}
	}
	let inputTransformSchemaForm: InputTransformSchemaForm | undefined = $state(undefined)

	let reloadError: string | undefined = $state(undefined)
	async function reload(flowModule: FlowModule) {
		reloadError = undefined
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
					if (!deepEqual(flowModule.value.input_transforms, input_transforms)) {
						flowModule.value.input_transforms = input_transforms
					}
				}
			}

			if (flowModule.value.type == 'rawscript' && flowModule.value.lock != undefined) {
				if (flowModule.value.lock != undefined) {
					flowModule.value.lock = undefined
				}
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
			reloadError = e?.message
		}
	}

	function selectAdvanced(subtab: string) {
		selected = 'advanced'
		advancedSelected = subtab
	}

	let forceReload = $state(0)
	let editorPanelSize = $state(noEditor ? 0 : flowModule.value.type == 'script' ? 30 : 50)
	let editorSettingsPanelSize = $state(100 - untrack(() => editorPanelSize))
	let stepHistoryLoader = getStepHistoryLoaderContext()

	function onSelectedIdChange() {
		if (!$flowStateStore?.[$selectedId]?.schema && flowModule) {
			reload(flowModule)
		}
	}

	async function getLastJob() {
		if (
			!$flowStateStore ||
			!flowModule.id ||
			$flowStateStore[flowModule.id]?.previewResult === 'never tested this far' ||
			!$flowStateStore[flowModule.id]?.previewJobId ||
			!$flowStateStore[flowModule.id]?.previewWorkspaceId
		) {
			return
		}
		const job = await JobService.getJob({
			workspace: $flowStateStore[flowModule.id]?.previewWorkspaceId ?? '',
			id: $flowStateStore[flowModule.id]?.previewJobId ?? ''
		})
		if (job) {
			lastJob = job
		}
	}

	let leftPanelSize = $state(0)

	function showDiffMode() {
		diffMode = true
		diffEditor?.setOriginal((savedModule?.value as RawScript).content ?? '')
		diffEditor?.setModified(editor?.getCode() ?? '')
		diffEditor?.show()
		editor?.hide()
	}

	function hideDiffMode() {
		diffMode = false
		diffEditor?.hide()
		editor?.show()
	}
	let lastDeployedCode = $derived(onModulesChange(savedModule, flowModule))

	let stepPropPicker = $derived(
		$executionCount != undefined && failureModule
			? getFailureStepPropPicker($flowStateStore, flowStore.val, previewArgs.val)
			: getStepPropPicker(
					$flowStateStore,
					parentModule,
					previousModule,
					flowModule.id,
					flowStore.val,
					previewArgs.val,
					false
				)
	)

	$effect(() => {
		$selectedId && untrack(() => onSelectedIdChange())
	})
	$effect(() => {
		if (testJob && testJob.type === 'CompletedJob') {
			lastJob = $state.snapshot(testJob)
		} else if ($workspaceStore && $pathStore && flowModule?.id && $flowStateStore) {
			untrack(() => getLastJob())
		}
	})
	let parentLoop = $derived(
		flowStore.val && flowModule ? checkIfParentLoop(flowStore.val, flowModule.id) : undefined
	)
	$effect(() => {
		if (selected === 'test') {
			leftPanelSize = 50
		} else {
			leftPanelSize = 100
		}
	})

	$effect(() => {
		editor &&
			($currentEditor = {
				type: 'script',
				editor,
				stepId: flowModule.id,
				showDiffMode,
				hideDiffMode,
				diffMode,
				lastDeployedCode
			})
	})

	onDestroy(() => {
		$currentEditor = undefined
	})

	// Handle force test tab prop with animation
	$effect(() => {
		if (forceTestTab) {
			selected = 'test'
			// Add a smooth transition to the test tab
			setTimeout(() => {
				const testTab = document.querySelector('[value="test"]')
				if (testTab) {
					testTab.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
				}
			}, 100)
		}
	})

	let assets = usePromise(async () =>
		flowModule.value.type === 'rawscript'
			? await inferAssets(flowModule.value.language, flowModule.value.content)
			: undefined
	)
	$effect(() => {
		if (flowModule.value.type !== 'rawscript') return
		;[flowModule.value.content, flowModule.value.language]
		untrack(() => assets.refresh())
	})
</script>

<svelte:window onkeydown={onKeyDown} />

{#if flowModule.value}
	<div class="h-full" bind:clientWidth={width}>
		<FlowCard
			flowModuleValue={flowModule?.value}
			on:reload={() => {
				forceReload++
				reload(flowModule)
			}}
			{noEditor}
			on:setHash={(e) => {
				if (flowModule.value.type == 'script') {
					flowModule.value.hash = e.detail
				}
			}}
			bind:summary={flowModule.summary}
		>
			{#snippet header()}
				<FlowModuleHeader
					{tag}
					module={flowModule}
					on:tagChange={(e) => {
						console.log('tagChange', e.detail)
						if (flowModule.value.type == 'script') {
							flowModule.value.tag_override = e.detail
						} else if (flowModule.value.type == 'rawscript') {
							flowModule.value.tag = e.detail
						}
					}}
					on:toggleSuspend={() => selectAdvanced('suspend')}
					on:toggleSleep={() => selectAdvanced('sleep')}
					on:toggleMock={() => selectAdvanced('mock')}
					on:toggleRetry={() => selectAdvanced('retries')}
					on:togglePin={() => (selected = 'test')}
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
						if (flowModule.value.type == 'flow') {
							forceReload++
							await reload(flowModule)
						}
					}}
					on:createScriptFromInlineScript={async () => {
						const [module, state] = await createScriptFromInlineScript(
							flowModule,
							$selectedId,
							$flowStateStore[flowModule.id].schema,
							$pathStore
						)
						if (flowModule.value.type == 'rawscript') {
							module.value.input_transforms = flowModule.value.input_transforms
						}
						flowModule = module
						$flowStateStore[module.id] = state
					}}
				/>
			{/snippet}

			<div class="h-full flex flex-col">
				{#if flowModule.value.type === 'rawscript' && !noEditor}
					<div class="shadow-sm px-1 border-b-1 border-gray-200 dark:border-gray-700">
						<EditorBar
							customUi={customUi?.editorBar}
							{validCode}
							{editor}
							{diffEditor}
							lang={flowModule.value['language'] ?? 'deno'}
							{websocketAlive}
							iconOnly={width < 950}
							kind={scriptKind}
							template={scriptTemplate}
							args={Object.entries(flowModule.value.input_transforms).reduce((acc, [key, obj]) => {
								acc[key] = obj.type === 'static' ? obj.value : undefined
								return acc
							}, {})}
							on:showDiffMode={showDiffMode}
							on:hideDiffMode={hideDiffMode}
							{lastDeployedCode}
							{diffMode}
							openAiChat
						/>
					</div>
				{/if}

				<div class="min-h-0 flex-grow" id="flow-editor-editor">
					<Splitpanes horizontal>
						<Pane bind:size={editorPanelSize} minSize={10} class="relative">
							{#if flowModule.value.type === 'rawscript'}
								{#if !noEditor}
									{#key flowModule.id}
										<div class="absolute top-2 right-4 z-10 flex flex-row gap-2">
											{#if assets.value?.length}
												<AssetsDropdownButton
													assets={assets.value}
													accessTypeOverrides={flowModule.value.asset_access_type_overrides}
													onAccessTypeChanged={async (asset, access_type) => {
														if (flowModule.value.type !== 'rawscript') return
														flowModule.value.asset_access_type_overrides =
															flowModule.value.asset_access_type_overrides?.filter(
																(a) => !assetEq(a, asset)
															)
														flowModule.value.asset_access_type_overrides ??= []
														await tick()
														flowModule.value.asset_access_type_overrides.push({
															...asset,
															access_type
														})
													}}
												/>
											{/if}
										</div>
										<Editor
											on:addSelectedLinesToAiChat={(e) => {
												const { lines, startLine, endLine } = e.detail
												aiChatManager.addSelectedLinesToContext(lines, startLine, endLine)
											}}
											on:toggleAiPanel={() => {
												aiChatManager.toggleOpen()
											}}
											loadAsync
											folding
											path={$pathStore + '/' + flowModule.id}
											bind:websocketAlive
											bind:this={editor}
											class="h-full relative"
											code={flowModule.value.content}
											lang={scriptLangToEditorLang(flowModule?.value?.language)}
											scriptLang={flowModule?.value?.language}
											automaticLayout={true}
											cmdEnterAction={async () => {
												selected = 'test'
												if ($selectedId == flowModule.id) {
													if (flowModule.value.type === 'rawscript' && editor) {
														flowModule.value.content = editor.getCode()
													}
													await reload(flowModule)
													modulePreview?.runTestWithStepArgs()
												}
											}}
											on:change={async (event) => {
												const content = event.detail
												if (flowModule.value.type === 'rawscript') {
													if (flowModule.value.content !== content) {
														flowModule.value.content = content
													}
													await reload(flowModule)
												}
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
											key={`flow-inline-${$workspaceStore}-${$pathStore}-${flowModule.id}`}
										/>
										<DiffEditor
											open={false}
											bind:this={diffEditor}
											automaticLayout
											fixedOverflowWidgets
											defaultLang={scriptLangToEditorLang(flowModule.value.language)}
											class="h-full"
										/>
									{/key}
								{/if}
							{:else if flowModule.value.type === 'script'}
								{#if !noEditor && (customUi?.hubCode != false || !flowModule?.value?.path?.startsWith('hub/'))}
									<div class="border-t">
										{#key forceReload}
											<FlowModuleScript
												bind:tag
												showAllCode={false}
												path={flowModule.value.path}
												hash={flowModule.value.hash}
											/>
										{/key}
									</div>
								{/if}
							{:else if flowModule.value.type === 'flow'}
								{#key forceReload}
									<FlowPathViewer path={flowModule.value.path} />
								{/key}
							{/if}
						</Pane>
						<Pane bind:size={editorSettingsPanelSize} minSize={20}>
							<Splitpanes>
								<Pane minSize={36} bind:size={leftPanelSize}>
									<Tabs bind:selected>
										{#if !preprocessorModule}
											<Tab value="inputs">Step Input</Tab>
										{/if}
										<Tab value="test">Test this step</Tab>
										{#if !preprocessorModule}
											<Tab value="advanced">Advanced</Tab>
										{/if}
									</Tabs>
									<div
										class={advancedSelected === 'runtime'
											? 'h-[calc(100%-68px)]'
											: 'h-[calc(100%-34px)]'}
									>
										{#if selected === 'inputs' && (flowModule.value.type == 'rawscript' || flowModule.value.type == 'script' || flowModule.value.type == 'flow')}
											<div class="h-full overflow-auto bg-surface" id="flow-editor-step-input">
												<PropPickerWrapper
													pickableProperties={stepPropPicker.pickableProperties}
													error={failureModule}
													noPadding
												>
													{#if reloadError}
														<div
															title={reloadError}
															class="absolute left-2 top-2 rounded-full w-2 h-2 bg-red-300"
														></div>
													{/if}
													<InputTransformSchemaForm
														class="px-1 xl:px-2"
														bind:this={inputTransformSchemaForm}
														pickableProperties={stepPropPicker.pickableProperties}
														schema={$flowStateStore[$selectedId]?.schema ?? {}}
														previousModuleId={previousModule?.id}
														bind:args={
															() => {
																// @ts-ignore
																return flowModule?.value?.input_transforms
															},
															(v) => {
																if (
																	typeof flowModule?.value === 'object' &&
																	flowModule?.value !== null
																) {
																	// @ts-ignore
																	flowModule.value.input_transforms = v
																}
															}
														}
														extraLib={stepPropPicker.extraLib}
														{enableAi}
													/>
												</PropPickerWrapper>
											</div>
										{:else if selected === 'test'}
											<ModulePreview
												pickableProperties={stepPropPicker.pickableProperties}
												bind:this={modulePreview}
												mod={flowModule}
												{noEditor}
												schema={$flowStateStore[$selectedId]?.schema ?? {}}
												bind:testJob
												bind:testIsLoading
												bind:scriptProgress
												focusArg={highlightArg}
											/>
										{:else if selected === 'advanced'}
											<Tabs bind:selected={advancedSelected}>
												<Tab value="retries" active={flowModule.retry !== undefined}>Retries</Tab>
												{#if !$selectedId.includes('failure')}
													<Tab value="runtime">Runtime</Tab>
													<Tab value="cache" active={Boolean(flowModule.cache_ttl)}>Cache</Tab>
													<Tab
														value="early-stop"
														active={Boolean(
															flowModule.stop_after_if || flowModule.stop_after_all_iters_if
														)}
													>
														Early Stop
													</Tab>
													<Tab value="skip" active={Boolean(flowModule.skip_if)}>Skip</Tab>
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
														{#snippet header()}
															<Tooltip
																documentationLink="https://www.windmill.dev/docs/flows/retries"
															>
																If defined, upon error this step will be retried with a delay and a
																maximum number of attempts as defined below.
															</Tooltip>
														{/snippet}
														<span class="text-2xs"
															>After all retries attempts have been exhausted:</span
														>
														<div class="flex gap-2 mb-4">
															<Toggle
																size="xs"
																bind:checked={flowModule.continue_on_error}
																options={{
																	left: 'Stop on error and propagate error up',
																	right: "Continue on error with error as step's return"
																}}
															/>
															<Tooltip>
																When enabled, the flow will continue to the next step after going
																through all the retries (if any) even if this step fails. This
																enables to process the error in a branch one for instance.
															</Tooltip>
														</div>
														<div class="my-8"></div>
														<FlowRetries bind:flowModuleRetry={flowModule.retry} />
													</Section>
												{:else if advancedSelected === 'runtime' && advancedRuntimeSelected === 'concurrency'}
													<Section label="Concurrency limits" class="flex flex-col gap-4" eeOnly>
														{#snippet header()}
															<Tooltip>Allowed concurrency within a given timeframe</Tooltip>
														{/snippet}
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
															<Label label="Custom concurrency key (optional)">
																{#snippet header()}
																	<Tooltip>
																		Concurrency keys are global, you can have them be workspace
																		specific using the variable `$workspace`. You can also use an
																		argument's value using `$args[name_of_arg]`</Tooltip
																	>
																{/snippet}
																<!-- svelte-ignore a11y_autofocus -->
																<input
																	type="text"
																	autofocus
																	disabled={!$enterpriseLicense}
																	bind:value={flowModule.value.custom_concurrency_key}
																	placeholder={`$workspace/script/${$pathStore}-$args[foo]`}
																/>
															</Label>
														{:else}
															<Alert type="warning" title="Limitation" size="xs">
																The concurrency limit of a workspace script is only settable in the
																script metadata itself. For hub scripts, this feature is non
																available yet.
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
															{#snippet header()}
																<Tooltip>The higher the number, the higher the priority.</Tooltip>
															{/snippet}
															<input
																type="number"
																class="!w-24"
																disabled={flowModule.priority === undefined}
																bind:value={flowModule.priority}
																onfocus={bubble('focus')}
																onchange={() => {
																	if (flowModule.priority && flowModule.priority > 100) {
																		flowModule.priority = 100
																	} else if (flowModule.priority && flowModule.priority < 0) {
																		flowModule.priority = 0
																	}
																}}
															/>
														</Label>

														<Alert type="warning" title="Limitation" size="xs">
															Setting priority is only available for enterprise edition and not
															available on the cloud.
														</Alert>
													</Section>
												{:else if advancedSelected === 'runtime' && advancedRuntimeSelected === 'lifetime'}
													<div>
														<FlowModuleDeleteAfterUse
															bind:flowModule
															disabled={!$enterpriseLicense}
														/>
													</div>
												{:else if advancedSelected === 'cache'}
													<div>
														<FlowModuleCache bind:flowModule />
													</div>
												{:else if advancedSelected === 'early-stop'}
													<FlowModuleEarlyStop bind:flowModule />
												{:else if advancedSelected === 'skip'}
													<FlowModuleSkip bind:flowModule {parentModule} {previousModule} />
												{:else if advancedSelected === 'suspend'}
													<div>
														<FlowModuleSuspend
															previousModuleId={previousModule?.id}
															bind:flowModule
														/>
													</div>
												{:else if advancedSelected === 'sleep'}
													<div>
														<FlowModuleSleep
															previousModuleId={previousModule?.id}
															bind:flowModule
														/>
													</div>
												{:else if advancedSelected === 'mock'}
													<div>
														<FlowModuleMockTransitionMessage />
													</div>
												{:else if advancedSelected === 'same_worker'}
													<div>
														<Alert type="info" title="Share a directory between steps">
															If shared directory is set, will share a folder that will be mounted
															on `./shared` for each of them to pass data between each other.
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
																{#snippet children({ item })}
																	{#if flowModule.value['language'] === 'deno'}
																		<ToggleButton
																			value="s3_client"
																			size="sm"
																			label="S3 lite client"
																			{item}
																		/>
																	{:else}
																		<ToggleButton
																			value="s3_client"
																			size="sm"
																			label="Boto3"
																			{item}
																		/>
																		<ToggleButton value="polars" size="sm" label="Polars" {item} />
																		<ToggleButton value="duckdb" size="sm" label="DuckDB" {item} />
																	{/if}
																{/snippet}
															</ToggleButtonGroup>
														</div>

														<Button
															size="xs"
															on:click={() =>
																editor?.setCode(s3Scripts[flowModule.value['language']][s3Kind])}
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
								{#if selected === 'test'}
									<Pane minSize={20} class="relative">
										{#if stepHistoryLoader?.stepStates[flowModule.id]?.initial && !flowModule.mock?.enabled}
											<!-- svelte-ignore a11y_no_static_element_interactions -->
											<!-- svelte-ignore a11y_click_events_have_key_events -->
											<div
												onclick={() => {
													stepHistoryLoader?.resetInitial(flowModule.id)
												}}
												class="cursor-pointer h-full hover:bg-gray-500/20 dark:hover:bg-gray-500/20 dark:bg-gray-500/80 bg-gray-500/40 absolute top-0 left-0 w-full z-50"
											>
												<div class="text-center text-primary text-sm py-2 pt-20"
													><span class="font-bold border p-2 bg-surface-secondary rounded-md"
														>Run loaded from history</span
													></div
												>
											</div>
										{/if}
										<ModulePreviewResultViewer
											lang={flowModule.value['language'] ?? 'deno'}
											{editor}
											{diffEditor}
											loopStatus={parentLoop
												? { type: 'inside', flow: parentLoop.type }
												: undefined}
											onUpdateMock={(detail) => {
												flowModule.mock = detail
												flowModule = flowModule
												refreshStateStore(flowStore)
											}}
											{lastJob}
											{scriptProgress}
											{testJob}
											mod={flowModule}
											{testIsLoading}
											disableMock={preprocessorModule || failureModule}
											disableHistory={failureModule}
											loadingJob={stepHistoryLoader?.stepStates[flowModule.id]?.loadingJobs}
										/>
									</Pane>
								{/if}
							</Splitpanes>
						</Pane>
					</Splitpanes>
				</div>
			</div>
		</FlowCard>
	</div>
{:else}
	Incorrect flow module type
{/if}
