<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import Button from '$lib/components/common/button/Button.svelte'
	import type { Preview, ScriptLang } from '$lib/gen'
	import { createEventDispatcher, onDestroy, onMount, untrack } from 'svelte'
	import { AlertTriangle, Trash2, Bug, Terminal } from 'lucide-svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { inferArgs, inferAssets } from '$lib/infer'
	import type { Schema } from '$lib/common'
	import Editor from '$lib/components/Editor.svelte'
	import { emptySchema, getLocalSetting, sendUserToast, storeLocalSetting } from '$lib/utils'

	import { scriptLangToEditorLang } from '$lib/scripts'
	import DiffEditor from '$lib/components/DiffEditor.svelte'
	import type { InlineScript, StaticAppInput, UserAppInput, CtxAppInput } from '../apps/inputType'
	import CacheTtlPopup from '../apps/editor/inlineScriptsPanel/CacheTtlPopup.svelte'
	import { computeFields } from '../apps/editor/inlineScriptsPanel/utils'
	import EditorBar from '../EditorBar.svelte'
	import { LanguageIcon } from '../common/languageIcons'
	import { resource } from 'runed'
	import { usePreparedAssetSqlQueries } from '$lib/infer.svelte'
	import AssetsDropdownButton from '../assets/AssetsDropdownButton.svelte'
	import { workspaceStore } from '$lib/stores'
	import { SvelteSet } from 'svelte/reactivity'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { editor as meditor } from 'monaco-editor'
	import {
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
	import TextInput from '../text_input/TextInput.svelte'

	interface Props {
		inlineScript: (InlineScript & { language: ScriptLang }) | undefined
		name?: string | undefined
		id: string
		fields?: Record<string, StaticAppInput | UserAppInput | CtxAppInput>
		path: string
		onRun: () => Promise<void>
		editor?: any | undefined
		lastDeployedCode?: string | undefined
		/** Called when code is selected in the editor */
		onSelectionChange?: (
			selection: {
				content: string
				startLine: number
				endLine: number
				startColumn: number
				endColumn: number
			} | null
		) => void
	}

	let {
		inlineScript = $bindable(),
		name = $bindable(undefined),
		id,
		fields = $bindable(undefined),
		path,
		onRun,
		editor = $bindable(undefined),
		lastDeployedCode,
		onSelectionChange
	}: Props = $props()
	let diffEditor = $state() as DiffEditor | undefined
	let validCode = $state(true)

	async function inferInlineScriptSchema(
		language: Preview['language'],
		content: string,
		schema: Schema
	): Promise<Schema> {
		try {
			await inferArgs(language, content, schema)
			validCode = true
		} catch (e) {
			console.error("Couldn't infer args", e)
			validCode = false
		}

		return schema
	}

	let websocketAlive = $state({
		pyright: false,
		deno: false,
		go: false,
		ruff: false,
		shellcheck: false
	})

	let diffMode = $state(false)

	function showDiffMode() {
		const model = editor?.getModel()
		if (model == undefined) return
		diffMode = true
		diffEditor?.showWithModelAndOriginal(lastDeployedCode ?? '', model)
		editor?.hide()
	}
	function hideDiffMode() {
		diffMode = false
		diffEditor?.hide()
		editor?.show()
	}

	onMount(async () => {
		if (inlineScript && !inlineScript.schema) {
			inlineScript.schema = await inferInlineScriptSchema(
				inlineScript?.language,
				inlineScript?.content,
				emptySchema()
			)
		}
		syncFields()
	})

	async function syncFields() {
		if (inlineScript) {
			const newSchema = inlineScript.schema ?? emptySchema()
			fields = computeFields(newSchema, true, fields ?? {})
		}
	}

	const dispatch = createEventDispatcher()
	let width = $state(0)

	let inferAssetsRes = resource(
		[() => inlineScript?.language, () => inlineScript?.content],
		async () => inlineScript && inferAssets(inlineScript.language, inlineScript.content)
	)
	let preparedSqlQueries = usePreparedAssetSqlQueries(
		() => inferAssetsRes.current?.sql_queries,
		() => $workspaceStore
	)
	$effect(() => {
		if (inlineScript && inferAssetsRes.current) inlineScript.assets = inferAssetsRes.current?.assets
	})

	// Debug mode state
	const DEBUG_BETA_WARNING_KEY = 'debug_beta_warning_confirmed'
	let showDebugBetaWarning = $state(false)
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
	const dapServerUrl = $derived(
		getDebugServerUrl((inlineScript?.language || 'python3') as DebugLanguage)
	)
	const debugFilePath = $derived(
		`/tmp/script${getDebugFileExtension(inlineScript?.language ?? '')}`
	)
	const isDebuggableScript = $derived(isDebuggable(inlineScript?.language ?? ''))
	const showDebugPanel = $derived(
		debugMode && $debugState.connected && ($debugState.running || $debugState.stopped)
	)
	const hasDebugResult = $derived(debugMode && $debugState.result !== undefined)
	const debugConsoleVisible = $derived(showDebugPanel && showDebugConsole)
	const currentDebugFrameId = $derived(selectedDebugFrameId ?? $debugState.stackFrames[0]?.id)

	// Export debug state for parent component
	export function getDebugState() {
		return {
			debugMode,
			isDebuggableScript,
			showDebugPanel,
			hasDebugResult,
			dapClient,
			selectedDebugFrameId,
			debugSessionJobId,
			debugBreakpoints
		}
	}

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

	export async function startDebugging(): Promise<void> {
		if (!inlineScript) return

		try {
			showDebugConsole = true
			selectedDebugFrameId = null

			resetDAPClient()
			dapClient = getDAPClient(dapServerUrl)

			const env = await fetchContextualVariables($workspaceStore ?? '')
			const code = inlineScript.content

			let signedPayload
			try {
				signedPayload = await signDebugRequest(
					$workspaceStore ?? '',
					code ?? '',
					inlineScript.language ?? 'python3'
				)
				debugSessionJobId = signedPayload.job_id
			} catch (signError) {
				sendUserToast(getDebugErrorMessage(signError), true)
				return
			}

			// Get static args from fields
			const args = Object.entries(fields ?? {}).reduce<Record<string, unknown>>(
				(acc, [key, obj]) => {
					if (obj.type === 'static') {
						acc[key] = obj.value
					}
					return acc
				},
				{}
			)

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

	export async function stopDebugging(): Promise<void> {
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

	export async function continueExecution(): Promise<void> {
		if (!dapClient) return
		await dapClient.continue_()
	}

	export async function stepOver(): Promise<void> {
		if (!dapClient) return
		await dapClient.stepOver()
	}

	export async function stepIn(): Promise<void> {
		if (!dapClient) return
		await dapClient.stepIn()
	}

	export async function stepOut(): Promise<void> {
		if (!dapClient) return
		await dapClient.stepOut()
	}

	export function clearAllBreakpoints(): void {
		debugBreakpoints.clear()
		updateBreakpointDecorations()
	}

	export function toggleDebugMode(): void {
		if (debugMode) {
			// Exiting debug mode - clean up
			debugMode = false
			stopDebugging()
			clearAllBreakpoints()
			updateCurrentLineDecoration(undefined)
		} else {
			// Entering debug mode - check if beta warning was confirmed
			if (getLocalSetting(DEBUG_BETA_WARNING_KEY) !== 'true') {
				showDebugBetaWarning = true
			} else {
				debugMode = true
			}
		}
	}

	function confirmDebugBetaWarning(): void {
		storeLocalSetting(DEBUG_BETA_WARNING_KEY, 'true')
		showDebugBetaWarning = false
		debugMode = true
	}

	// Subscribe to debug state changes for current line highlighting
	$effect(() => {
		const currentLine = $debugState.currentLine
		if (debugMode) {
			untrack(() => updateCurrentLineDecoration(currentLine))
		}
	})

	// Watch for language changes - exit debug mode when language changes
	let lastDebugLang: ScriptLang | undefined = undefined
	$effect(() => {
		const currentLang = inlineScript?.language
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
	onDestroy(() => {
		if (debugMode) {
			stopDebugging()
			resetDAPClient()
		}
	})

	// Track last selection to avoid duplicate events
	let lastSelectionKey = $state<string | null>(null)
	// Track pending selection during mouse drag
	let pendingSelection: {
		startLineNumber: number
		startColumn: number
		endLineNumber: number
		endColumn: number
	} | null = null
	let isMouseDown = false

	function emitSelection(editorInstance: Editor): void {
		if (!onSelectionChange) return

		const selection = pendingSelection
		if (!selection) {
			// No selection - only emit null if we previously had a selection
			if (lastSelectionKey !== null) {
				lastSelectionKey = null
				onSelectionChange(null)
			}
			return
		}

		// Check if there's an actual selection (not just cursor position)
		const hasSelection =
			selection.startLineNumber !== selection.endLineNumber ||
			selection.startColumn !== selection.endColumn

		if (!hasSelection) {
			// No selection - only emit null if we previously had a selection
			if (lastSelectionKey !== null) {
				lastSelectionKey = null
				onSelectionChange(null)
			}
			return
		}

		// Get the selected content from the editor
		const model = editorInstance.getModel?.()
		if (!model || !('getValueInRange' in model)) return

		const content = (model as any).getValueInRange({
			startLineNumber: selection.startLineNumber,
			startColumn: selection.startColumn,
			endLineNumber: selection.endLineNumber,
			endColumn: selection.endColumn
		})

		// Create a key to deduplicate identical selections
		const selectionKey = `${selection.startLineNumber}:${selection.startColumn}:${selection.endLineNumber}:${selection.endColumn}`
		if (selectionKey === lastSelectionKey) return
		lastSelectionKey = selectionKey

		onSelectionChange({
			content,
			startLine: selection.startLineNumber,
			endLine: selection.endLineNumber,
			startColumn: selection.startColumn,
			endColumn: selection.endColumn
		})
	}

	// Listen for editor selection changes - wait for mouseup before emitting
	$effect(() => {
		if (!editor || !onSelectionChange) return

		const editorInstance = editor

		// Track selection changes but don't emit until mouseup
		const selectionDisposable = editorInstance.onDidChangeCursorSelection?.((e) => {
			pendingSelection = e.selection
			// If not mouse-driven (e.g., keyboard selection), emit immediately
			if (!isMouseDown) {
				untrack(() => emitSelection(editorInstance))
			}
		})

		// Track mouse state
		const handleMouseDown = () => {
			isMouseDown = true
		}
		const handleMouseUp = () => {
			if (isMouseDown) {
				isMouseDown = false
				untrack(() => emitSelection(editorInstance))
			}
		}

		// Add mouse listeners to the document to catch mouseup even outside editor
		document.addEventListener('mousedown', handleMouseDown)
		document.addEventListener('mouseup', handleMouseUp)

		return () => {
			selectionDisposable?.dispose()
			document.removeEventListener('mousedown', handleMouseDown)
			document.removeEventListener('mouseup', handleMouseUp)
		}
	})
</script>

{#if inlineScript}
	<div class="h-full flex flex-col gap-1" bind:clientWidth={width}>
		<div class="flex justify-between w-full gap-2 px-2 pt-1 flex-row items-center">
			<div class="mx-0.5">
				<LanguageIcon lang={inlineScript.language} width={20} height={20} />
			</div>
			{#if name !== undefined}
				<div class="flex flex-row gap-2 w-full items-center">
					<TextInput
						inputProps={{
							onkeydown: () => stopPropagation(bubble('keydown')),
							placeholder: 'Inline script name'
						}}
						bind:value={name}
						size="sm"
					/>
				</div>
				<Button
					title="Clear script"
					variant="subtle"
					aria-label="Clear script"
					destructive
					unifiedSize="sm"
					on:click={() => dispatch('delete')}
					endIcon={{ icon: Trash2 }}
					iconOnly
				/>
			{/if}
			<div class="flex w-full flex-row gap-2 items-center justify-end">
				{#if inlineScript}
					<CacheTtlPopup bind:cache_ttl={inlineScript.cache_ttl} />
				{/if}

				<Button
					variant="default"
					unifiedSize="sm"
					on:click={async () => {
						editor?.format()
					}}
				>
					Format
				</Button>
			</div>
		</div>

		<div class="shadow-sm px-1 border-b-1 border-gray-200 dark:border-gray-700">
			<EditorBar
				{validCode}
				{editor}
				lang={inlineScript.language}
				{websocketAlive}
				iconOnly={width < 1250}
				kind={'script'}
				template={'script'}
				on:showDiffMode={showDiffMode}
				on:hideDiffMode={hideDiffMode}
				{lastDeployedCode}
				{diffMode}
				openAiChat
				moduleId={id}
			/>
		</div>

		<div class="border-y h-full w-full relative">
			<div class="absolute top-2 right-4 z-10 flex flex-row gap-2">
				{#if inlineScript.assets?.length}
					<AssetsDropdownButton assets={inlineScript.assets} />
				{/if}
				{#if isDebuggableScript}
					<Button
						variant={debugMode ? 'accent' : 'default'}
						size="xs2"
						on:click={toggleDebugMode}
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
						size="xs2"
						on:click={() => (showDebugConsole = true)}
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
						<Editor
							path={path + '/' + id}
							bind:this={editor}
							class="flex flex-1 grow h-full"
							scriptLang={inlineScript.language}
							bind:code={inlineScript.content}
							fixedOverflowWidgets={true}
							cmdEnterAction={() => onRun()}
							bind:websocketAlive
							rawAppRunnableKey={id}
							on:change={async (e) => {
								if (inlineScript) {
									if (inlineScript.lock != undefined) {
										inlineScript.lock = undefined
									}
									const oldSchema = JSON.stringify(inlineScript.schema)
									if (inlineScript.schema == undefined) {
										inlineScript.schema = emptySchema()
									}
									await inferInlineScriptSchema(
										inlineScript?.language,
										e.detail,
										inlineScript.schema
									)
									if (JSON.stringify(inlineScript.schema) != oldSchema) {
										inlineScript = inlineScript
										syncFields()
									}
									if (debugMode && breakpointDecorations.length > 0) {
										refreshBreakpointPositions()
									}
								}
							}}
							args={Object.entries(fields ?? {}).reduce((acc, [key, obj]) => {
								acc[key] = obj.type === 'static' ? obj.value : undefined
								return acc
							}, {})}
							preparedAssetsSqlQueries={preparedSqlQueries.current}
						/>
					</Pane>
					<Pane bind:size={consolePaneSize} minSize={10}>
						<DebugConsole
							client={dapClient}
							currentFrameId={currentDebugFrameId}
							onClose={() => (showDebugConsole = false)}
							workspace={$workspaceStore}
							jobId={debugSessionJobId ?? undefined}
						/>
					</Pane>
				</Splitpanes>
			{:else}
				<Editor
					path={path + '/' + id}
					bind:this={editor}
					class="flex flex-1 grow h-full"
					scriptLang={inlineScript.language}
					bind:code={inlineScript.content}
					fixedOverflowWidgets={true}
					cmdEnterAction={() => onRun()}
					bind:websocketAlive
					rawAppRunnableKey={id}
					on:change={async (e) => {
						if (inlineScript) {
							if (inlineScript.lock != undefined) {
								inlineScript.lock = undefined
							}
							const oldSchema = JSON.stringify(inlineScript.schema)
							if (inlineScript.schema == undefined) {
								inlineScript.schema = emptySchema()
							}
							await inferInlineScriptSchema(inlineScript?.language, e.detail, inlineScript.schema)
							if (JSON.stringify(inlineScript.schema) != oldSchema) {
								inlineScript = inlineScript
								syncFields()
							}
							if (debugMode && breakpointDecorations.length > 0) {
								refreshBreakpointPositions()
							}
						}
					}}
					args={Object.entries(fields ?? {}).reduce((acc, [key, obj]) => {
						acc[key] = obj.type === 'static' ? obj.value : undefined
						return acc
					}, {})}
					preparedAssetsSqlQueries={preparedSqlQueries.current}
				/>
			{/if}

			<DiffEditor
				open={false}
				bind:this={diffEditor}
				modifiedModel={editor?.getModel()}
				className="h-full"
				automaticLayout
				fixedOverflowWidgets
				defaultLang={scriptLangToEditorLang(inlineScript?.language)}
			/>
		</div>
	</div>
{/if}

<Modal title="Debug Feature (Beta)" bind:open={showDebugBetaWarning}>
	<div class="flex items-start gap-3">
		<div class="flex-shrink-0">
			<div
				class="flex h-10 w-10 items-center justify-center rounded-full bg-yellow-100 dark:bg-yellow-800/50"
			>
				<AlertTriangle class="h-5 w-5 text-yellow-600 dark:text-yellow-400" />
			</div>
		</div>
		<div class="text-secondary text-sm">
			<p
				>The Debug feature is currently in <strong>beta</strong>. You may encounter unexpected
				behavior or limitations.</p
			>
			<p class="mt-2">By continuing, you acknowledge that this feature is experimental.</p>
		</div>
	</div>
	{#snippet actions()}
		<Button size="sm" on:click={confirmDebugBetaWarning}>Continue</Button>
	{/snippet}
</Modal>
