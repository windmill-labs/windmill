<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Editor from '$lib/components/Editor.svelte'
	import EditorBar, {
		EDITOR_BAR_WIDTH_THRESHOLD,
		EDITOR_BAR_HELPERS_INLINE_THRESHOLD
	} from '$lib/components/EditorBar.svelte'
	import ModulePreview from '$lib/components/ModulePreview.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { createScriptFromInlineScript, fork } from '$lib/components/flows/flowStateUtils.svelte'

	import type { FlowModule, FlowModuleValue, RawScript, ScriptLang } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowModuleHeader from './FlowModuleHeader.svelte'
	import { getLatestHashForScript, scriptLangToEditorLang } from '$lib/scripts'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import { getContext, onDestroy, tick, untrack } from 'svelte'
	import type { FlowEditorContext, FlowGraphAssetContext } from '../types'
	import FlowModuleScript from './FlowModuleScript.svelte'
	import FlowModuleAdvancedSettings from './FlowModuleAdvancedSettings.svelte'
	import { getFailureStepPropPicker, getStepPropPicker } from '../previousResults'
	import { deepEqual } from 'fast-equals'
	import Section from '$lib/components/Section.svelte'

	import Button from '$lib/components/common/button/Button.svelte'
	import FlowPathViewer from './FlowPathViewer.svelte'
	import InputTransformSchemaForm from '$lib/components/InputTransformSchemaForm.svelte'
	import DiffEditor from '$lib/components/DiffEditor.svelte'
	import type { ButtonProp } from '$lib/components/diffEditorTypes'
	import { loadSchemaFromModule } from '../flowInfers'
	import { type Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { checkIfParentLoop } from '../utils.svelte'
	import ModulePreviewResultViewer from '$lib/components/ModulePreviewResultViewer.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import { getStepHistoryLoaderContext } from '$lib/components/stepHistoryLoader.svelte'
	import AssetsDropdownButton from '$lib/components/assets/AssetsDropdownButton.svelte'
	import { useUiIntent } from '$lib/components/copilot/chat/flow/useUiIntent'
	import { editor as meditor } from 'monaco-editor'
	import { DynamicInput } from '$lib/utils'
	import { usePreparedAssetSqlQueries } from '$lib/infer.svelte'
	import { SvelteSet } from 'svelte/reactivity'
	import { slide } from 'svelte/transition'
	import {
		DebugToolbar,
		DebugPanel,
		DebugConsole,
		getDAPClient,
		debugState,
		resetDAPClient,
		getDebugServerUrl,
		type DebugLanguage,
		isDebuggable,
		getDebugFileExtension,
		fetchContextualVariables,
		signDebugRequest,
		getDebugErrorMessage
	} from '$lib/components/debug'
	import { Bug, Terminal } from 'lucide-svelte'
	import { sendUserToast } from '$lib/utils'

	const {
		selectionManager,
		currentEditor,
		previewArgs,
		flowStateStore,
		flowStore,
		pathStore,
		saveDraft,
		customUi,
		executionCount,
		opWorkspace
	} = getContext<FlowEditorContext>('FlowEditorContext')

	const selectedId = $derived(selectionManager.getSelectedId())
	let opWs = $derived(opWorkspace?.() ?? $workspaceStore)

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
		isAgentTool?: boolean
		siblingToolNames?: string[]
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
		highlightArg = undefined,
		isAgentTool = false,
		siblingToolNames = undefined
	}: Props = $props()

	let workspaceScriptTag: string | undefined = $state(undefined)
	let workspaceScriptLang: ScriptLang | undefined = $state(undefined)
	let diffMode = $state(false)
	let diffButtons = $state<ButtonProp[]>([
		{
			text: 'Quit diff mode',
			color: 'red',
			onClick: () => {
				hideDiffMode()
			}
		}
	])

	let editor: any | undefined = $state()
	let diffEditor: DiffEditor | undefined = $state()
	let modulePreview: ModulePreview | undefined = $state()
	let websocketAlive = $state({
		pyright: false,
		deno: false,
		go: false,
		ruff: false,
		shellcheck: false
	})

	let selected = $state(untrack(() => preprocessorModule) ? 'test' : 'inputs')
	let canShowChatTab = $derived(
		!preprocessorModule &&
			Boolean(flowStore.val.value?.chat_input_enabled) &&
			flowModule.value.type === 'aiagent'
	)
	let visibleSelected = $derived(selected === 'chat' && !canShowChatTab ? 'inputs' : selected)
	let advancedSettings: FlowModuleAdvancedSettings | undefined = $state()
	let validCode = $state(true)
	let width = $state(1200)
	let testJob: Job | undefined = $state(undefined)
	let testIsLoading = $state(false)
	let scriptProgress = $state(undefined)

	let assets = $derived((flowModule.value.type === 'rawscript' && flowModule.value.assets) || [])
	const flowGraphAssetsCtx = getContext<FlowGraphAssetContext | undefined>('FlowGraphAssetContext')

	// UI Intent handling for AI tool control
	useUiIntent(`flow-${flowModule.id}`, {
		openTab: (tab) => {
			selectAdvanced(tab)
		}
	})

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
			const { input_transforms, schema } = await loadSchemaFromModule(flowModule, opWs)
			validCode = true

			if (inputTransformSchemaForm) {
				inputTransformSchemaForm.setArgs(input_transforms)
			} else {
				if (
					flowModule.value.type == 'rawscript' ||
					flowModule.value.type == 'script' ||
					flowModule.value.type == 'flow' ||
					flowModule.value.type == 'aiagent'
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
			if (!deepEqual(schema, flowStateStore.val[flowModule.id]?.schema)) {
				if (!flowStateStore.val[flowModule.id]) {
					flowStateStore.val[flowModule.id] = { schema }
				} else {
					flowStateStore.val[flowModule.id].schema = schema
				}
			}
		} catch (e) {
			validCode = false
			reloadError = e?.message
		}
	}

	function selectAdvanced(subtab: string) {
		selected = 'advanced'
		tick().then(() => advancedSettings?.openSetting(subtab))
	}

	function setOmitOutputFromConversation(omit: boolean) {
		if (flowModule.value.type !== 'aiagent') {
			return
		}

		if (omit) {
			flowModule.value.omit_output_from_conversation = true
		} else {
			delete flowModule.value.omit_output_from_conversation
		}
	}

	let forceReload = $state(0)
	let editorPanelSize = $state(untrack(() => noEditor) ? 0 : 30)
	let editorSettingsPanelSize = $state(100 - untrack(() => editorPanelSize))
	let stepHistoryLoader = getStepHistoryLoaderContext()

	function onSelectedIdChange() {
		if (!flowStateStore?.val?.[selectedId]?.schema && flowModule) {
			reload(flowModule)
		}
	}

	let leftPanelSize = $state(0)

	function showDiffMode() {
		const model = editor?.getModel()
		if (model == undefined) return
		diffMode = true

		diffEditor?.showWithModelAndOriginal((savedModule?.value as RawScript).content ?? '', model)
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
			? getFailureStepPropPicker(flowStateStore.val, flowStore.val, previewArgs.val)
			: getStepPropPicker(
					flowStateStore.val,
					parentModule,
					previousModule,
					flowModule.id,
					flowStore.val,
					previewArgs.val,
					false
				)
	)

	$effect.pre(() => {
		selectedId && untrack(() => onSelectedIdChange())
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
				lastDeployedCode,
				setDiffOriginal: (code: string) => {
					diffEditor?.setOriginal(code ?? '')
				},
				setDiffButtons: (buttons: ButtonProp[]) => {
					diffButtons = buttons
				}
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

	let rawScriptLang = $derived(
		flowModule.value.type == 'rawscript' ? flowModule.value.language : undefined
	)

	let modulePreviewResultViewer: ModulePreviewResultViewer | undefined = $state(undefined)

	function retrieveDynCodeAndLang(value: FlowModuleValue): DynamicInput.HelperScript | undefined {
		let helperScript: DynamicInput.HelperScript | undefined
		switch (value.type) {
			case 'script':
				helperScript = {
					source: 'deployed',
					path: value.path,
					runnable_kind: 'script'
				}
				break
			case 'rawscript':
				helperScript = {
					source: 'inline',
					code: value.content,
					lang: value.language
				}
				break
			case 'flow':
				helperScript = {
					source: 'deployed',
					path: value.path,
					runnable_kind: 'flow'
				}
				break
			default:
				helperScript = undefined
		}

		return helperScript
	}

	function onJobDone() {
		modulePreviewResultViewer?.getOutputPickerInner()?.setJobPreview()
	}

	let preparedSqlQueries = usePreparedAssetSqlQueries(
		() => flowGraphAssetsCtx?.val.sqlQueries[selectedId],
		() => opWs
	)

	// Debug mode state
	let debugMode = $state(false)
	let debugBreakpoints = new SvelteSet<number>()
	let breakpointDecorations: string[] = $state([])
	let currentLineDecoration: string[] = $state([])
	let dapClient = $state<ReturnType<typeof getDAPClient> | null>(null)
	let selectedDebugFrameId: number | null = $state(null)
	let debugSessionJobId: string | null = $state(null)
	let showDebugConsole = $state(true)
	let editorPaneSize = $state(75)
	let consolePaneSize = $state(25)

	// Get the DAP server URL based on language
	const dapServerUrl = $derived(getDebugServerUrl((rawScriptLang || 'python3') as DebugLanguage))
	const debugFilePath = $derived(`/tmp/script${getDebugFileExtension(rawScriptLang ?? '')}`)
	const isDebuggableScript = $derived(isDebuggable(rawScriptLang ?? ''))
	const showDebugPanel = $derived(
		debugMode && $debugState.connected && ($debugState.running || $debugState.stopped)
	)
	const hasDebugResult = $derived(debugMode && $debugState.result !== undefined)
	const debugConsoleVisible = $derived(showDebugPanel && showDebugConsole)
	const currentDebugFrameId = $derived(selectedDebugFrameId ?? $debugState.stackFrames[0]?.id)

	// Breakpoint decoration options
	const breakpointDecorationType: meditor.IModelDecorationOptions = {
		glyphMarginClassName: 'debug-breakpoint-glyph',
		glyphMarginHoverMessage: { value: 'Breakpoint (click to remove)' },
		stickiness: 1
	}

	const currentLineDecorationType = {
		isWholeLine: true,
		className: 'debug-current-line',
		glyphMarginClassName: 'debug-current-line-glyph'
	}

	// Debug functions
	function toggleBreakpoint(line: number): void {
		if (debugBreakpoints.has(line)) {
			debugBreakpoints.delete(line)
		} else {
			debugBreakpoints.add(line)
		}
		updateBreakpointDecorations()
	}

	function updateBreakpointDecorations(): void {
		const monacoEditor = editor?.getEditor?.()
		if (!monacoEditor) return

		const decorations = Array.from(debugBreakpoints).map((line) => ({
			range: { startLineNumber: line, startColumn: 1, endLineNumber: line, endColumn: 1 },
			options: breakpointDecorationType
		}))

		const oldDecorations = untrack(() => breakpointDecorations)
		breakpointDecorations = monacoEditor.deltaDecorations(oldDecorations, decorations)
	}

	function refreshBreakpointPositions(): void {
		const monacoEditor = editor?.getEditor?.()
		if (!monacoEditor || breakpointDecorations.length === 0) return

		const model = monacoEditor.getModel()
		if (!model) return

		const newLines = new Set<number>()
		for (const decorationId of breakpointDecorations) {
			const range = model.getDecorationRange(decorationId)
			if (range) {
				newLines.add(range.startLineNumber)
			}
		}

		const oldLines = Array.from(debugBreakpoints).sort((a, b) => a - b)
		const updatedLines = Array.from(newLines).sort((a, b) => a - b)

		const positionsChanged =
			oldLines.length !== updatedLines.length ||
			oldLines.some((line, i) => line !== updatedLines[i])

		if (positionsChanged) {
			debugBreakpoints.clear()
			for (const line of newLines) {
				debugBreakpoints.add(line)
			}
			syncBreakpointsWithServer()
		}
	}

	async function syncBreakpointsWithServer(): Promise<void> {
		if (!dapClient || !dapClient.isConnected()) return
		try {
			await dapClient.setBreakpoints(debugFilePath, Array.from(debugBreakpoints))
		} catch (error) {
			console.error('Failed to sync breakpoints:', error)
		}
	}

	function updateCurrentLineDecoration(line: number | undefined): void {
		const monacoEditor = editor?.getEditor?.()
		if (!monacoEditor) return

		const oldDecorations = untrack(() => currentLineDecoration)

		if (!line) {
			currentLineDecoration = monacoEditor.deltaDecorations(oldDecorations, [])
			return
		}

		const decorations = [
			{
				range: { startLineNumber: line, startColumn: 1, endLineNumber: line, endColumn: 1 },
				options: currentLineDecorationType
			}
		]

		currentLineDecoration = monacoEditor.deltaDecorations(oldDecorations, decorations)
		monacoEditor.revealLineInCenter(line)
	}

	async function startDebugging(): Promise<void> {
		if (flowModule.value.type !== 'rawscript') return

		try {
			showDebugConsole = true
			selectedDebugFrameId = null

			resetDAPClient()
			dapClient = getDAPClient(dapServerUrl)

			const env = await fetchContextualVariables(opWs ?? '')
			const code = flowModule.value.content

			let signedPayload
			try {
				signedPayload = await signDebugRequest(opWs ?? '', code ?? '', rawScriptLang ?? 'python3')
				debugSessionJobId = signedPayload.job_id
			} catch (signError) {
				sendUserToast(getDebugErrorMessage(signError), true)
				return
			}

			// Get static args from input transforms
			const args = Object.entries(flowModule.value.input_transforms).reduce<
				Record<string, unknown>
			>((acc, [key, obj]) => {
				if (obj.type === 'static') {
					acc[key] = obj.value
				}
				return acc
			}, {})

			await dapClient.connect()
			await dapClient.initialize()
			await dapClient.setBreakpoints(debugFilePath, Array.from(debugBreakpoints))
			await dapClient.configurationDone()
			await dapClient.launch({
				code,
				cwd: '/tmp',
				args,
				callMain: true,
				env,
				token: signedPayload.token
			})
		} catch (error) {
			console.error('Failed to start debugging:', error)
			sendUserToast(getDebugErrorMessage(error), true)
		}
	}

	async function stopDebugging(): Promise<void> {
		if (!dapClient) return
		try {
			await dapClient.terminate()
			dapClient.disconnect()
		} catch (error) {
			console.error('Failed to stop debugging:', error)
		} finally {
			debugSessionJobId = null
		}
	}

	async function continueExecution(): Promise<void> {
		if (!dapClient) return
		await dapClient.continue_()
	}

	async function stepOver(): Promise<void> {
		if (!dapClient) return
		await dapClient.stepOver()
	}

	async function stepIn(): Promise<void> {
		if (!dapClient) return
		await dapClient.stepIn()
	}

	async function stepOut(): Promise<void> {
		if (!dapClient) return
		await dapClient.stepOut()
	}

	function clearAllBreakpoints(): void {
		debugBreakpoints.clear()
		updateBreakpointDecorations()
	}

	function toggleDebugMode(): void {
		if (debugMode) {
			// Exiting debug mode - clean up
			debugMode = false
			stopDebugging()
			clearAllBreakpoints()
			updateCurrentLineDecoration(undefined)
		} else {
			debugMode = true
			// Switch to test tab when entering debug mode
			selected = 'test'
		}
	}

	// Subscribe to debug state changes for current line highlighting
	$effect(() => {
		const currentLine = $debugState.currentLine
		if (debugMode) {
			untrack(() => updateCurrentLineDecoration(currentLine))
		}
	})

	// Watch for language changes - exit debug mode when language changes
	let lastDebugLang: typeof rawScriptLang | undefined = undefined
	$effect(() => {
		const currentLang = rawScriptLang
		if (lastDebugLang !== undefined && lastDebugLang !== currentLang && debugMode) {
			untrack(() => {
				if (dapClient) {
					dapClient
						.terminate()
						.catch(() => {})
						.finally(() => {
							dapClient?.disconnect()
						})
				}
				resetDAPClient()
				dapClient = null
				debugMode = false
				clearAllBreakpoints()
				updateCurrentLineDecoration(undefined)
			})
		}
		lastDebugLang = currentLang
	})

	// Set up glyph margin click handler for breakpoints when debug mode is enabled
	$effect(() => {
		const monacoEditor = editor?.getEditor?.()
		if (!monacoEditor) return

		if (debugMode && isDebuggableScript) {
			monacoEditor.updateOptions({ glyphMargin: true })

			const mouseDownDisposable = monacoEditor.onMouseDown((e) => {
				if (e.target.type === 2) {
					const line = e.target.position?.lineNumber
					if (line) {
						toggleBreakpoint(line)
					}
				}
			})

			monacoEditor.addCommand(120, () => {
				const position = monacoEditor.getPosition()
				if (position) {
					toggleBreakpoint(position.lineNumber)
				}
			})

			monacoEditor.addCommand(119, () => {
				if ($debugState.stopped) continueExecution()
			})

			monacoEditor.addCommand(117, () => {
				if ($debugState.stopped) stepOver()
			})

			monacoEditor.addCommand(118, () => {
				if ($debugState.stopped) stepIn()
			})

			monacoEditor.addCommand(1143, () => {
				if ($debugState.stopped) stepOut()
			})

			return () => {
				mouseDownDisposable.dispose()
				monacoEditor.updateOptions({ glyphMargin: false })
			}
		} else {
			monacoEditor.updateOptions({ glyphMargin: false })
		}
	})

	// Clean up debug mode on destroy
	import { onDestroy as onDestroyHook } from 'svelte'
	onDestroyHook(() => {
		if (debugMode) {
			stopDebugging()
			resetDAPClient()
		}
	})
</script>

<svelte:window onkeydown={onKeyDown} />

{#if flowModule.value}
	<div class="h-full bg-surface" bind:clientWidth={width}>
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
			{isAgentTool}
			{siblingToolNames}
		>
			{#snippet header()}
				<FlowModuleHeader
					tag={workspaceScriptTag ?? rawScriptLang ?? workspaceScriptLang}
					module={flowModule}
					on:tagChange={(e) => {
						console.log('tagChange', e.detail)
						if (flowModule.value.type == 'script') {
							flowModule.value.tag_override = e.detail
						} else if (flowModule.value.type == 'rawscript' || flowModule.value.type == 'aiagent') {
							flowModule.value.tag = e.detail
						}
					}}
					on:toggleSuspend={() => selectAdvanced('suspend')}
					on:toggleSleep={() => selectAdvanced('sleep')}
					on:toggleRetry={() => selectAdvanced('retries')}
					on:togglePin={() => (selected = 'test')}
					on:toggleConcurrency={() => selectAdvanced('runtime')}
					on:toggleCache={() => selectAdvanced('cache')}
					on:toggleStopAfterIf={() => selectAdvanced('early-stop')}
					on:fork={async () => {
						const [module, state] = await fork(flowModule, opWs)
						flowModule = module
						flowStateStore.val[module.id] = state
					}}
					on:reload={async () => {
						if (flowModule.value.type == 'script') {
							if (flowModule.value.hash != undefined) {
								flowModule.value.hash = await getLatestHashForScript(flowModule.value.path, opWs)
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
							selectedId,
							flowStateStore.val[flowModule.id]?.schema,
							$pathStore,
							opWs
						)
						if (flowModule.value.type == 'rawscript') {
							module.value.input_transforms = flowModule.value.input_transforms
						}
						flowModule = module
						flowStateStore.val[module.id] = state
					}}
				/>
			{/snippet}

			<div class="h-full flex flex-col">
				{#if flowModule.value.type === 'rawscript' && !noEditor}
					<div class="shadow-sm px-1 border-b-1 border-gray-200 dark:border-gray-700">
						<EditorBar
							customUi={customUi?.editorBar}
							workspace={opWs}
							{validCode}
							{editor}
							lang={flowModule.value['language'] ?? 'deno'}
							{websocketAlive}
							iconOnly={width < EDITOR_BAR_WIDTH_THRESHOLD}
							compactHelpers={width < EDITOR_BAR_HELPERS_INLINE_THRESHOLD}
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
							moduleId={flowModule.id}
						/>
					</div>
				{/if}

				<div class="min-h-0 flex-grow" id="flow-editor-editor">
					{#snippet topPaneContent()}
						{#if flowModule.value.type === 'rawscript'}
							{#if !noEditor}
								{#key flowModule.id}
									<div class="absolute top-2 right-4 z-10 flex flex-row gap-2">
										{#if assets?.length}
											<AssetsDropdownButton {assets} />
										{/if}
										{#if isDebuggableScript && customUi?.editorBar?.debug != false}
											<Button
												variant={debugMode ? 'accent' : 'default'}
												size="xs"
												onclick={toggleDebugMode}
												startIcon={{ icon: Bug }}
												btnClasses={debugMode
													? ''
													: 'bg-surface hover:bg-surface-hover border border-tertiary/30'}
												title="Toggle Debug Mode"
											>
												{debugMode ? 'Exit Debug' : 'Debug'}
											</Button>
										{/if}
										{#if showDebugPanel && !showDebugConsole}
											<Button
												variant="default"
												size="xs"
												onclick={() => (showDebugConsole = true)}
												startIcon={{ icon: Terminal }}
												btnClasses="bg-surface hover:bg-surface-hover border border-tertiary/30"
												title="Show Debug Console"
											>
												Console
											</Button>
										{/if}
									</div>
									{#if debugConsoleVisible}
										<Splitpanes horizontal class="h-full">
											<Pane bind:size={editorPaneSize} minSize={20}>
												<div id="flow-editor-code-section" class="h-full relative">
													<Editor
														loadAsync
														folding
														path={$pathStore + '/' + flowModule.id}
														bind:websocketAlive
														bind:this={editor}
														class="h-full relative"
														code={flowModule.value.content}
														scriptLang={flowModule?.value?.language}
														automaticLayout={true}
														cmdEnterAction={async () => {
															selected = 'test'
															if (selectedId == flowModule.id) {
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
																if (debugMode && breakpointDecorations.length > 0) {
																	refreshBreakpointPositions()
																}
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
														key={`flow-inline-${opWs}-${$pathStore}-${flowModule.id}`}
														moduleId={flowModule.id}
														preparedAssetsSqlQueries={preparedSqlQueries.current}
														customTag={flowModule.value.tag}
													/>
												</div>
											</Pane>
											<Pane bind:size={consolePaneSize} minSize={10}>
												<DebugConsole
													client={dapClient}
													currentFrameId={currentDebugFrameId}
													onClose={() => (showDebugConsole = false)}
													workspace={opWs}
													jobId={debugSessionJobId ?? undefined}
												/>
											</Pane>
										</Splitpanes>
									{:else}
										<div id="flow-editor-code-section" class="h-full relative">
											<Editor
												loadAsync
												folding
												path={$pathStore + '/' + flowModule.id}
												bind:websocketAlive
												bind:this={editor}
												class="h-full relative"
												code={flowModule.value.content}
												scriptLang={flowModule?.value?.language}
												automaticLayout={true}
												cmdEnterAction={async () => {
													selected = 'test'
													if (selectedId == flowModule.id) {
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
														if (debugMode && breakpointDecorations.length > 0) {
															refreshBreakpointPositions()
														}
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
												key={`flow-inline-${opWs}-${$pathStore}-${flowModule.id}`}
												moduleId={flowModule.id}
												preparedAssetsSqlQueries={preparedSqlQueries.current}
												customTag={flowModule.value.tag}
											/>
										</div>
									{/if}
									<DiffEditor
										open={false}
										bind:this={diffEditor}
										modifiedModel={editor?.getModel() as meditor.ITextModel}
										automaticLayout
										fixedOverflowWidgets
										defaultLang={scriptLangToEditorLang(flowModule.value.language)}
										className="h-full"
										buttons={diffMode ? diffButtons : []}
									/>
								{/key}
							{/if}
						{:else if flowModule.value.type === 'script'}
							{#if !noEditor && (customUi?.hubCode != false || !flowModule?.value?.path?.startsWith('hub/'))}
								<div class="border-t">
									{#key forceReload}
										<FlowModuleScript
											bind:tag={workspaceScriptTag}
											bind:language={workspaceScriptLang}
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
					{/snippet}

					{#snippet bottomPaneContent()}
						<Splitpanes>
							<Pane minSize={36} bind:size={leftPanelSize}>
								<div class="flex flex-col relative h-[99.99%]">
									<Tabs
										selected={visibleSelected}
										on:selected={(event) => {
											selected = event.detail
										}}
										wrapperClass="shrink-0"
									>
										{#if !preprocessorModule}
											<Tab value="inputs" label="Step Input" />
										{/if}
										<Tab value="test" label="Test this step" />
										{#if canShowChatTab && flowModule.value.type === 'aiagent'}
											<Tab
												value="chat"
												active={Boolean(flowModule.value.omit_output_from_conversation)}
												label="Chat"
											/>
										{/if}
										{#if !preprocessorModule && !isAgentTool}
											<Tab value="advanced" label="Run settings" />
										{/if}
									</Tabs>
									{#if visibleSelected === 'inputs' && (flowModule.value.type == 'rawscript' || flowModule.value.type == 'script' || flowModule.value.type == 'flow' || flowModule.value.type == 'aiagent')}
										<div class="flex-1 overflow-auto" id="flow-editor-step-input">
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
													class="px-2 xl:px-4 pb-8"
													bind:this={inputTransformSchemaForm}
													pickableProperties={stepPropPicker.pickableProperties}
													schema={flowStateStore.val[selectedId]?.schema ?? {}}
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
													{isAgentTool}
													allowedAiTransforms={isAgentTool && flowModule.value.type === 'aiagent'
														? ['user_message']
														: undefined}
													helperScript={retrieveDynCodeAndLang(flowModule.value)}
													chatInputEnabled={flowStore.val.value?.chat_input_enabled ?? false}
												/>
											</PropPickerWrapper>
										</div>
									{:else if visibleSelected === 'test'}
										{#if debugMode && isDebuggableScript}
											<div transition:slide={{ duration: 200 }}>
												<DebugToolbar
													connected={$debugState.connected}
													running={$debugState.running}
													stopped={$debugState.stopped}
													breakpointCount={debugBreakpoints.size}
													onStart={startDebugging}
													onStop={stopDebugging}
													onContinue={continueExecution}
													onStepOver={stepOver}
													onStepIn={stepIn}
													onStepOut={stepOut}
													onClearBreakpoints={clearAllBreakpoints}
													onExitDebug={toggleDebugMode}
												/>
											</div>
										{/if}
										<ModulePreview
											class="flex-1"
											pickableProperties={stepPropPicker.pickableProperties}
											bind:this={modulePreview}
											mod={flowModule}
											{noEditor}
											schema={flowStateStore.val[selectedId]?.schema ?? {}}
											bind:testJob
											bind:testIsLoading
											bind:scriptProgress
											focusArg={highlightArg}
											{onJobDone}
											hideRunButton={debugMode && isDebuggableScript}
										/>
									{:else if visibleSelected === 'chat' && canShowChatTab && flowModule.value.type === 'aiagent'}
										<div class="flex-1 overflow-auto p-4">
											<Section label="Conversation output">
												<Toggle
													size="xs"
													checked={Boolean(flowModule.value.omit_output_from_conversation)}
													on:change={(event) => {
														setOmitOutputFromConversation(event.detail)
													}}
													options={{
														right: 'Omit assistant and tool messages from the flow conversation',
														rightTooltip:
															'When enabled, this AI agent still runs normally, but its assistant response and tool-use messages are not stored in chat-mode conversation history.'
													}}
												/>
											</Section>
										</div>
									{:else if visibleSelected === 'advanced'}
										<FlowModuleAdvancedSettings
											bind:this={advancedSettings}
											bind:flowModule
											{parentModule}
											{previousModule}
											{selectedId}
										/>
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
									{#if showDebugPanel || hasDebugResult}
										<Splitpanes horizontal class="h-full">
											<Pane size={50} minSize={15}>
												<Splitpanes horizontal class="h-full">
													<Pane size={50} minSize={10}>
														<LogViewer
															small
															content={$debugState.logs}
															isLoading={$debugState.running && !$debugState.stopped}
															tag={undefined}
														/>
													</Pane>
													<Pane size={50} minSize={10}>
														{#if hasDebugResult}
															<div class="h-full p-2 overflow-auto">
																<DisplayResult
																	result={$debugState.result}
																	language={rawScriptLang}
																/>
															</div>
														{:else}
															<div
																class="h-full flex items-center justify-center text-sm text-tertiary"
															>
																{#if $debugState.running && !$debugState.stopped}
																	Running...
																{:else if $debugState.stopped}
																	Paused at breakpoint
																{:else}
																	Waiting for debug session
																{/if}
															</div>
														{/if}
													</Pane>
												</Splitpanes>
											</Pane>
											<Pane size={50} minSize={15}>
												<DebugPanel
													stackFrames={$debugState.stackFrames}
													scopes={$debugState.scopes}
													variables={$debugState.variables}
													client={dapClient}
													bind:selectedFrameId={selectedDebugFrameId}
												/>
											</Pane>
										</Splitpanes>
									{:else if debugMode && isDebuggableScript}
										<div class="h-full flex items-center justify-center text-sm text-tertiary">
											Click "Debug" in the toolbar to start debugging
										</div>
									{:else}
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
											{testJob}
											{scriptProgress}
											mod={flowModule}
											{testIsLoading}
											disableMock={preprocessorModule || failureModule}
											disableHistory={failureModule}
											loadingJob={stepHistoryLoader?.stepStates[flowModule.id]?.loadingJobs}
											tagLabel={customUi?.tagLabel}
											bind:this={modulePreviewResultViewer}
										/>
									{/if}
								</Pane>
							{/if}
						</Splitpanes>
					{/snippet}

					{#if flowModule.value.type === 'aiagent' || (noEditor && flowModule.value.type !== 'flow')}
						<!-- Top pane has no content to show (aiagent has no editor; rawscript/script
						gate their content on !noEditor). Skip the Splitpanes wrapper entirely so
						there's no orphan splitter. type === 'flow' still renders FlowPathViewer
						even with noEditor, so it falls into the Splitpanes branch below. -->
						<div class="h-full">
							{@render bottomPaneContent()}
						</div>
					{:else}
						<Splitpanes horizontal>
							<Pane bind:size={editorPanelSize} minSize={10} class="relative">
								{@render topPaneContent()}
							</Pane>
							<Pane bind:size={editorSettingsPanelSize} minSize={20}>
								{@render bottomPaneContent()}
							</Pane>
						</Splitpanes>
					{/if}
				</div>
			</div>
		</FlowCard>
	</div>
{:else}
	Incorrect flow module type
{/if}
