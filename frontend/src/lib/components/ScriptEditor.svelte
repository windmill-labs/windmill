<script lang="ts">
	import { BROWSER } from 'esm-env'

	import type { Schema, SupportedLanguage } from '$lib/common'
	import { type CompletedJob, type Job, JobService, type Preview, type ScriptLang } from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import {
		copyToClipboard,
		emptySchema,
		getLocalSetting,
		sendUserToast,
		storeLocalSetting
	} from '$lib/utils'
	import Editor from './Editor.svelte'
	import { inferArgs, inferAssets, inferAnsibleExecutionMode } from '$lib/infer'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import SchemaForm from './SchemaForm.svelte'
	import LogPanel from './scriptEditor/LogPanel.svelte'
	import EditorBar, { EDITOR_BAR_WIDTH_THRESHOLD } from './EditorBar.svelte'
	import JobLoader from './JobLoader.svelte'
	import JobProgressBar from '$lib/components/jobs/JobProgressBar.svelte'
	import { createEventDispatcher, onDestroy, onMount, untrack } from 'svelte'
	import { Button } from './common'
	import SplitPanesWrapper from './splitPanes/SplitPanesWrapper.svelte'
	import WindmillIcon from './icons/WindmillIcon.svelte'
	import * as Y from 'yjs'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import { WebsocketProvider } from 'y-websocket'
	import Modal from './common/modal/Modal.svelte'
	import DiffEditor from './DiffEditor.svelte'
	import {
		AlertTriangle,
		Bug,
		Copy,
		CornerDownLeft,
		ExternalLink,
		Github,
		GitBranch,
		Play,
		PlayIcon,
		Terminal,
		WandSparkles
	} from 'lucide-svelte'
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
	import { SvelteSet } from 'svelte/reactivity'
	import { setLicense } from '$lib/enterpriseUtils'
	import type { ScriptEditorWhitelabelCustomUi } from './custom_ui'
	import Tabs from './common/tabs/Tabs.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import { slide } from 'svelte/transition'
	import CaptureTable from '$lib/components/triggers/CaptureTable.svelte'
	import CaptureButton from './triggers/CaptureButton.svelte'
	import { setContext } from 'svelte'
	import HideButton from './apps/editor/settingsPanel/HideButton.svelte'
	import { base } from '$lib/base'
	import { SUPPORTED_CHAT_SCRIPT_LANGUAGES } from './copilot/chat/script/core'
	import { getStringError } from './copilot/chat/utils'
	import type { ScriptOptions } from './copilot/chat/ContextManager.svelte'
	import { aiChatManager, AIMode } from './copilot/chat/AIChatManager.svelte'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import AssetsDropdownButton from './assets/AssetsDropdownButton.svelte'
	import { canHavePreprocessor } from '$lib/script_helpers'
	import { assetEq, type AssetWithAltAccessType } from './assets/lib'
	import { editor as meditor } from 'monaco-editor'
	import type { ReviewChangesOpts } from './copilot/chat/monaco-adapter'
	import GitRepoViewer from './GitRepoViewer.svelte'
	import GitRepoResourcePicker from './GitRepoResourcePicker.svelte'
	import { updateDelegateToGitRepoConfig, insertAdditionalInventories } from '$lib/ansibleUtils'
	import { copilotInfo } from '$lib/aiStore'
	import JsonInputs from '$lib/components/JsonInputs.svelte'
	import Toggle from './Toggle.svelte'
	import { deepEqual } from 'fast-equals'
	import { usePreparedAssetSqlQueries } from '$lib/infer.svelte'
	import { resource, watch } from 'runed'

	interface Props {
		// Exported
		schema?: Schema | any
		code: string
		path: string | undefined
		lang: Preview['language']
		kind?: string | undefined
		template?: 'pgsql' | 'mysql' | 'script' | 'docker' | 'powershell' | 'bunnative'
		tag: string | undefined
		initialArgs?: Record<string, any>
		fixedOverflowWidgets?: boolean
		noSyncFromGithub?: boolean
		editor?: Editor | undefined
		diffEditor?: DiffEditor | undefined
		collabMode?: boolean
		edit?: boolean
		noHistory?: boolean
		saveToWorkspace?: boolean
		watchChanges?: boolean
		customUi?: ScriptEditorWhitelabelCustomUi | undefined
		args: Record<string, any>
		selectedTab?: 'main' | 'preprocessor'
		hasPreprocessor?: boolean
		captureTable?: CaptureTable | undefined
		showCaptures?: boolean
		stablePathForCaptures?: string
		lastSavedCode?: string | undefined
		lastDeployedCode?: string | undefined
		disableAi?: boolean
		assets?: AssetWithAltAccessType[]
		editor_bar_right?: import('svelte').Snippet
		enablePreprocessorSnippet?: boolean
	}

	let {
		schema = $bindable(),
		code = $bindable(),
		path,
		lang,
		kind = undefined,
		template = 'script',
		tag,
		fixedOverflowWidgets = true,
		noSyncFromGithub = false,
		editor = $bindable(undefined),
		diffEditor = $bindable(undefined),
		collabMode = false,
		edit = true,
		noHistory = false,
		saveToWorkspace = false,
		watchChanges = false,
		customUi = undefined,
		args = $bindable(),
		selectedTab = $bindable('main'),
		hasPreprocessor = $bindable(false),
		captureTable = $bindable(undefined),
		showCaptures = true,
		stablePathForCaptures = '',
		lastSavedCode = undefined,
		lastDeployedCode = undefined,
		disableAi = false,
		assets = $bindable(),
		editor_bar_right,
		enablePreprocessorSnippet = false
	}: Props = $props()

	let initialArgs = structuredClone($state.snapshot(args))
	let jsonView = $state(false)
	let schemaHeight = $state(0)

	$effect.pre(() => {
		if (schema == undefined) {
			schema = emptySchema()
		}
	})
	let showHistoryDrawer = $state(false)

	let jobProgressBar: JobProgressBar | undefined = $state(undefined)
	let diffMode = $state(false)

	let websocketAlive = $state({
		pyright: false,
		deno: false,
		go: false,
		ruff: false,
		shellcheck: false
	})

	let inferAssetsRes = resource([() => lang, () => code, () => code], () => inferAssets(lang, code))
	let preparedSqlQueries = usePreparedAssetSqlQueries(
		() => inferAssetsRes.current?.sql_queries,
		() => $workspaceStore
	)

	const dispatch = createEventDispatcher()

	$effect(() => {
		watchChanges &&
			(code != undefined || schema != undefined) &&
			dispatch('change', { code, schema })
	})

	watch(
		() => inferAssetsRes.current,
		() => {
			if (!inferAssetsRes.current || inferAssetsRes.current?.status === 'error') return
			let newAssets = inferAssetsRes.current.assets as AssetWithAltAccessType[]
			for (const asset of newAssets) {
				const old = assets?.find((a) => assetEq(a, asset))
				if (old?.alt_access_type) asset.alt_access_type = old.alt_access_type
			}
			if (!deepEqual(assets, newAssets)) assets = newAssets
		}
	)

	watch([() => code, () => lang], () => {
		if (lang !== 'ansible') return
		inferAnsibleExecutionMode(code).then((v) => {
			if (
				v !== undefined &&
				(v.delegate_to_git_repo_details === null ||
					v.delegate_to_git_repo_details.resource !== ansibleAlternativeExecutionMode?.resource ||
					v.delegate_to_git_repo_details.playbook !== ansibleAlternativeExecutionMode?.playbook ||
					v.delegate_to_git_repo_details.inventories_location !==
						ansibleAlternativeExecutionMode?.inventories_location ||
					v.delegate_to_git_repo_details.commit !== ansibleAlternativeExecutionMode?.commit ||
					v.git_ssh_identity !== ansibleGitSshIdentity)
			) {
				ansibleAlternativeExecutionMode = v.delegate_to_git_repo_details
				ansibleGitSshIdentity = v.git_ssh_identity
			}
		})
	})

	let width = $state(1200)

	let jobLoader: JobLoader | undefined = $state(undefined)

	let isValid: boolean = $state(true)
	let scriptProgress = $state(undefined)

	let logPanel: LogPanel | undefined = $state(undefined)
	// Test
	let testIsLoading = $state(false)
	let testJob: Job | undefined = $state()
	let pastPreviews: CompletedJob[] = $state([])
	let validCode = $state(true)

	let wsProvider: WebsocketProvider | undefined = $state(undefined)
	let yContent: Y.Text | undefined = $state(undefined)
	let peers: { name: string }[] = $state([])
	let showCollabPopup = $state(false)

	let ansibleAlternativeExecutionMode = $state<
		| { resource?: string; commit?: string; inventories_location?: string; playbook?: string }
		| null
		| undefined
	>()
	let ansibleGitSshIdentity = $state<string[]>([])

	// Debug mode state
	const DEBUG_BETA_WARNING_KEY = 'debug_beta_warning_confirmed'
	let showDebugBetaWarning = $state(false)
	let debugMode = $state(false)
	let debugBreakpoints = new SvelteSet<number>()
	let breakpointDecorations: string[] = $state([])
	let currentLineDecoration: string[] = $state([])
	// Get the DAP server URL based on language
	const dapServerUrl = $derived(getDebugServerUrl((lang || 'python3') as DebugLanguage))
	const debugFilePath = $derived(`/tmp/script${getDebugFileExtension(lang || '')}`)
	let dapClient = $state<ReturnType<typeof getDAPClient> | null>(null)
	const isDebuggableScript = $derived(isDebuggable(lang || ''))
	// Derived: show debug panel when connected and (running or stopped, but not terminated)
	const showDebugPanel = $derived(
		debugMode && $debugState.connected && ($debugState.running || $debugState.stopped)
	)
	// Derived: debug has a result (script completed)
	const hasDebugResult = $derived(debugMode && $debugState.result !== undefined)
	// Show debug console at bottom of editor when debugging is active
	let showDebugConsole = $state(true)
	const debugConsoleVisible = $derived(showDebugPanel && showDebugConsole)
	// Selected stack frame ID - shared between DebugPanel and DebugConsole
	let selectedDebugFrameId: number | null = $state(null)
	// Use selected frame or first frame for console context
	const currentDebugFrameId = $derived(selectedDebugFrameId ?? $debugState.stackFrames[0]?.id)
	// Job ID of the current debug session (for expression signing/audit logging)
	let debugSessionJobId: string | null = $state(null)
	// Pane sizes for editor/console split (percentage)
	let editorPaneSize = $state(75)
	let consolePaneSize = $state(25)

	// Breakpoint decoration options
	// stickiness: 1 = NeverGrowsWhenTypingAtEdges - decorations track their position when code changes
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

	const url = new URL(window.location.toString())
	let initialCollab = /true|1/i.test(url.searchParams.get('collab') ?? '0')

	if (initialCollab) {
		setCollaborationMode()
		url.searchParams.delete('collab')
		url.searchParams.delete('path')
		history.replaceState(null, '', url)
	}

	function onKeyDown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key == 'Enter') {
			event.preventDefault()
			runTest()
		} else if ((event.ctrlKey || event.metaKey) && event.key == 'u') {
			event.preventDefault()
			toggleTestPanel()
		}
	}

	export function setArgs(nargs: Record<string, any>) {
		args = nargs
	}

	export async function runTest() {
		// Not defined if JobProgressBar not loaded
		jobProgressBar?.reset()
		//@ts-ignore
		let job = await jobLoader.runPreview(
			path,
			code,
			lang,
			selectedTab === 'preprocessor' || kind === 'preprocessor'
				? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...(args ?? {}) }
				: (args ?? {}),
			tag,
			undefined,
			undefined,
			{
				done(_x) {
					loadPastTests()
				},
				doneError({ error }) {
					console.error(error)
					// sendUserToast('Error running test', true)
				}
			}
		)
		logPanel?.setFocusToLogs()
		return job
	}

	async function loadPastTests(): Promise<void> {
		pastPreviews = await JobService.listCompletedJobs({
			workspace: $workspaceStore!,
			jobKinds: 'preview',
			createdBy: $userStore?.username,
			scriptPathExact: path
		})
	}

	export async function inferSchema(
		code: string,
		{
			nlang,
			resetArgs = false,
			applyInitialArgs = false
		}: {
			nlang?: SupportedLanguage
			resetArgs?: boolean
			applyInitialArgs?: boolean
		} = {}
	) {
		let nschema = schema ?? emptySchema()

		try {
			const result = await inferArgs(
				nlang ?? lang,
				code,
				nschema,
				selectedTab === 'preprocessor' || kind === 'preprocessor' ? 'preprocessor' : undefined
			)

			if (kind === 'preprocessor') {
				hasPreprocessor = false
				selectedTab = 'main'
			} else {
				hasPreprocessor =
					(selectedTab === 'preprocessor' ? !result?.no_main_func : result?.has_preprocessor) ??
					false

				if (!hasPreprocessor && selectedTab === 'preprocessor') {
					selectedTab = 'main'
				}
			}

			validCode = true
			if (resetArgs) {
				args = {}
			}
			if (applyInitialArgs) {
				// we reapply initial args as the schema form might have cleared them between mount and the schema inference
				args = initialArgs
			}
			schema = nschema
		} catch (e) {
			validCode = false
		}
	}

	let gitRepoResourcePickerOpen = $state(false)
	let commitHashForGitRepo = $derived(ansibleAlternativeExecutionMode?.commit)

	// Check if delegate_to_git_repo exists in the code
	let hasDelegateToGitRepo = $derived(code && code.includes('delegate_to_git_repo:'))

	function handleDelegateConfigUpdate(event: {
		detail: { resourcePath: string; playbook?: string; inventoriesLocation?: string }
	}) {
		if (!editor) return

		const currentCode = editor.getCode()
		const newCode = updateDelegateToGitRepoConfig(currentCode, {
			resource: event.detail.resourcePath,
			playbook: event.detail.playbook,
			inventories_location: event.detail.inventoriesLocation
		})
		editor.setCode(newCode)

		// Trigger schema inference to update assets
		inferSchema(newCode)
	}

	function handleAddInventories(event: { detail: { inventoryPaths: string[] } }) {
		if (!editor) return

		const currentCode = editor.getCode()
		const newCode = insertAdditionalInventories(currentCode, event.detail.inventoryPaths)
		editor.setCode(newCode)

		// Trigger schema inference to update assets
		inferSchema(newCode)
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

		// Use untrack to prevent reactive loop when reading the old decorations
		const oldDecorations = untrack(() => breakpointDecorations)
		breakpointDecorations = monacoEditor.deltaDecorations(oldDecorations, decorations)
	}

	// Refresh breakpoint line numbers from decoration positions after code edits
	function refreshBreakpointPositions(): void {
		const monacoEditor = editor?.getEditor?.()
		if (!monacoEditor || breakpointDecorations.length === 0) return

		const model = monacoEditor.getModel()
		if (!model) return

		// Get current line numbers from decorations (Monaco tracks positions when code changes)
		const newLines = new Set<number>()
		for (const decorationId of breakpointDecorations) {
			const range = model.getDecorationRange(decorationId)
			if (range) {
				newLines.add(range.startLineNumber)
			}
		}

		// Check if positions changed
		const oldLines = Array.from(debugBreakpoints).sort((a, b) => a - b)
		const updatedLines = Array.from(newLines).sort((a, b) => a - b)

		const positionsChanged =
			oldLines.length !== updatedLines.length ||
			oldLines.some((line, i) => line !== updatedLines[i])

		if (positionsChanged) {
			// Update breakpoints set with new positions
			debugBreakpoints.clear()
			for (const line of newLines) {
				debugBreakpoints.add(line)
			}
			// Sync updated positions with server if connected
			syncBreakpointsWithServer()
		}
	}

	// Sync breakpoints with DAP server when connected
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

		// Use untrack to prevent reactive loop when reading the old decorations
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
		try {
			// Show console when starting a debug session
			showDebugConsole = true
			// Reset selected frame when starting new session
			selectedDebugFrameId = null

			// Always reset and create a fresh DAP client with the correct URL for the current language
			// This ensures we connect to the correct endpoint even if language changed
			resetDAPClient()
			dapClient = getDAPClient(dapServerUrl)

			// Fetch contextual variables (WM_WORKSPACE, WM_TOKEN, etc.) from backend
			const env = await fetchContextualVariables($workspaceStore ?? '')

			// Sign the debug request (creates audit log entry)
			let signedPayload
			try {
				signedPayload = await signDebugRequest($workspaceStore ?? '', code ?? '', lang ?? 'python3')
				debugSessionJobId = signedPayload.job_id
			} catch (signError) {
				sendUserToast(getDebugErrorMessage(signError), true)
				return
			}

			await dapClient.connect()
			await dapClient.initialize()
			await dapClient.setBreakpoints(debugFilePath, Array.from(debugBreakpoints))
			await dapClient.configurationDone()
			// Pass the signed token along with other launch parameters
			await dapClient.launch({
				code,
				cwd: '/tmp',
				args: args ?? {},
				callMain: true,
				env,
				// JWT token for verification by the debugger
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
			// Clear the job ID when debug session ends
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

	// Watch for language changes - exit debug mode and reset client when language changes
	let lastDebugLang: typeof lang | undefined = undefined
	$effect(() => {
		const currentLang = lang
		if (lastDebugLang !== undefined && lastDebugLang !== currentLang && debugMode) {
			// Language changed while in debug mode - exit debug mode
			untrack(() => {
				// Stop any running debug session
				if (dapClient) {
					dapClient
						.terminate()
						.catch(() => {})
						.finally(() => {
							dapClient?.disconnect()
						})
				}
				// Reset the singleton
				resetDAPClient()
				dapClient = null
				// Exit debug mode
				debugMode = false
				// Clear decorations
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
			// Enable glyph margin for breakpoints
			monacoEditor.updateOptions({ glyphMargin: true })

			// Add click handler for glyph margin (breakpoint toggle)
			const mouseDownDisposable = monacoEditor.onMouseDown((e) => {
				// MouseTargetType.GUTTER_GLYPH_MARGIN = 2
				if (e.target.type === 2) {
					const line = e.target.position?.lineNumber
					if (line) {
						toggleBreakpoint(line)
					}
				}
			})

			// Add F9 keyboard shortcut for toggling breakpoint at cursor
			monacoEditor.addCommand(120, () => {
				// KeyCode.F9 = 120
				const position = monacoEditor.getPosition()
				if (position) {
					toggleBreakpoint(position.lineNumber)
				}
			})

			// Debug stepping keyboard shortcuts (only active when stopped)
			// F8 = Continue (KeyCode.F8 = 119)
			monacoEditor.addCommand(119, () => {
				if ($debugState.stopped) continueExecution()
			})

			// F6 = Step Over (KeyCode.F6 = 117)
			monacoEditor.addCommand(117, () => {
				if ($debugState.stopped) stepOver()
			})

			// F7 = Step Into (KeyCode.F7 = 118)
			monacoEditor.addCommand(118, () => {
				if ($debugState.stopped) stepIn()
			})

			// Shift+F8 = Step Out (KeyMod.Shift | KeyCode.F8 = 1024 | 119 = 1143)
			monacoEditor.addCommand(1143, () => {
				if ($debugState.stopped) stepOut()
			})

			return () => {
				mouseDownDisposable.dispose()
				// Disable glyph margin when exiting debug mode
				monacoEditor.updateOptions({ glyphMargin: false })
			}
		} else {
			// Ensure glyph margin is disabled when not in debug mode
			monacoEditor.updateOptions({ glyphMargin: false })
		}
	})

	onMount(() => {
		inferSchema(code, { applyInitialArgs: true })
		loadPastTests()
		aiChatManager.saveAndClear()
		aiChatManager.changeMode(AIMode.SCRIPT)
	})

	setLicense()
	export async function setCollaborationMode() {
		await setLicense()
		if (!$enterpriseLicense) {
			sendUserToast(`Multiplayer is an enterprise feature`, true, [
				{
					label: 'Upgrade',
					callback: () => {
						window.open('https://www.windmill.dev/pricing', '_blank')
					}
				}
			])
			return
		}

		const ydoc = new Y.Doc()
		if (wsProvider) {
			wsProvider.destroy()
		}
		let yContentInit = ydoc.getText('content')

		const wsProtocol = BROWSER && window.location.protocol == 'https:' ? 'wss' : 'ws'

		wsProvider = new WebsocketProvider(
			`${wsProtocol}://${window.location.host}/ws_mp/`,
			$workspaceStore + '/' + (path ?? 'no-room-name'),
			ydoc,
			{ connect: false }
		)

		wsProvider.on('sync', (isSynced: boolean) => {
			if (isSynced && yContentInit?.toJSON() == '') {
				showCollabPopup = true
				yContentInit?.insert(0, code)
			}
			yContent = yContentInit
		})

		wsProvider.on('connection-error', (WSErrorEvent) => {
			console.error(WSErrorEvent)
			sendUserToast('Multiplayer server connection had an error', true)
		})
		wsProvider.connect()
		const awareness = wsProvider.awareness

		awareness.setLocalStateField('user', {
			name: $userStore?.username
		})

		function setPeers() {
			peers = Array.from(awareness.getStates().values()).map((x) => x?.['user'])
		}

		setPeers()
		// You can observe when a user updates their awareness information
		awareness.on('change', (changes) => {
			setPeers()
		})
	}

	export function disableCollaboration() {
		if (!wsProvider?.shouldConnect) return
		peers = []
		wsProvider?.disconnect()
		wsProvider.destroy()
		wsProvider = undefined
	}

	onDestroy(() => {
		disableCollaboration()
		aiChatManager.scriptEditorApplyCode = undefined
		aiChatManager.scriptEditorShowDiffMode = undefined
		aiChatManager.scriptEditorGetLintErrors = undefined
		aiChatManager.scriptEditorOptions = undefined
		aiChatManager.saveAndClear()
		aiChatManager.changeMode(AIMode.NAVIGATOR)
		// Clean up debug mode
		if (debugMode) {
			stopDebugging()
			resetDAPClient()
		}
	})

	function asKind(str: string | undefined) {
		return str as 'script' | 'approval' | 'trigger' | undefined
	}

	function collabUrl() {
		let url = new URL(window.location.toString().split('#')[0])
		url.search = ''
		return (
			`${url}?collab=1&workspace=${encodeURIComponent($workspaceStore ?? '')}&lang=${encodeURIComponent(lang ?? '')}` +
			(edit ? '' : `&path=${path}`)
		)
	}

	let showTabs = $derived(hasPreprocessor)
	$effect(() => {
		!hasPreprocessor && (selectedTab = 'main')
	})
	$effect(() => {
		selectedTab && code && untrack(() => inferSchema(code))
	})

	let argsRender = $state(0)
	export async function updateArgs(newArgs: Record<string, any>) {
		if (Object.keys(newArgs).length > 0) {
			args = { ...newArgs }
			argsRender++
		}
	}

	setContext('disableTooltips', customUi?.disableTooltips === true)

	let codePanelSize = $state(70)
	let testPanelSize = $state(30)
	let storedTestPanelSize = untrack(() => testPanelSize)

	function toggleTestPanel() {
		if (testPanelSize > 0) {
			storedTestPanelSize = testPanelSize
			codePanelSize += testPanelSize
			testPanelSize = 0
		} else {
			codePanelSize -= storedTestPanelSize
			testPanelSize = storedTestPanelSize
		}
	}

	function getError(job: Job | undefined) {
		if (job != undefined && job.type === 'CompletedJob' && !job.success) {
			return getStringError(job.result)
		}
		return undefined
	}

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

	let error = $derived(getError(testJob))

	$effect(() => {
		const options: ScriptOptions = {
			code,
			lang: lang as ScriptLang,
			error,
			args: args ?? {},
			path,
			lastSavedCode,
			lastDeployedCode,
			diffMode
		}
		untrack(() => {
			aiChatManager.scriptEditorOptions = options
			aiChatManager.scriptEditorApplyCode = async (code: string, opts?: ReviewChangesOpts) => {
				hideDiffMode()
				await editor?.reviewAndApplyCode(code, opts)
			}
			aiChatManager.scriptEditorShowDiffMode = showDiffMode
			aiChatManager.scriptEditorGetLintErrors = () => {
				return (
					editor?.getLintErrors() ?? { errorCount: 0, warningCount: 0, errors: [], warnings: [] }
				)
			}
		})
	})
</script>

<JobLoader
	noCode={true}
	bind:scriptProgress
	bind:this={jobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>

<svelte:window onkeydown={onKeyDown} />

<!-- Standalone triggerable registration for the script editor -->
<div
	style="display: none"
	use:triggerableByAI={{
		id: 'script-editor',
		description: 'Component to edit a script'
	}}
></div>

<Modal title="Invite others" bind:open={showCollabPopup}>
	<div>Have others join by sharing the following url:</div>
	<div class="flex gap-2 pr-4">
		<input type="text" disabled value={collabUrl()} />

		<Button
			color="light"
			startIcon={{ icon: Copy }}
			iconOnly
			on:click={() => copyToClipboard(collabUrl())}
		/>
	</div>
</Modal>

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

<div class="border-b shadow-sm px-1 pr-4" bind:clientWidth={width}>
	<div class="flex justify-between space-x-2">
		{#if args}
			<EditorBar
				scriptPath={edit ? path : undefined}
				on:toggleCollabMode={() => {
					if (wsProvider?.shouldConnect) {
						disableCollaboration()
					} else {
						setCollaborationMode()
					}
				}}
				on:showDiffMode={showDiffMode}
				on:hideDiffMode={hideDiffMode}
				customUi={customUi?.editorBar}
				collabLive={wsProvider?.shouldConnect}
				{collabMode}
				{validCode}
				iconOnly={width < EDITOR_BAR_WIDTH_THRESHOLD}
				on:collabPopup={() => (showCollabPopup = true)}
				{editor}
				{lang}
				on:createScriptFromInlineScript
				{websocketAlive}
				collabUsers={peers}
				kind={asKind(kind)}
				{template}
				{args}
				{noHistory}
				{saveToWorkspace}
				lastDeployedCode={lastDeployedCode && lastDeployedCode !== code
					? lastDeployedCode
					: undefined}
				{diffMode}
				bind:showHistoryDrawer
			>
				{#snippet right()}
					{@render editor_bar_right?.()}
				{/snippet}
			</EditorBar>
		{/if}
		{#if !noSyncFromGithub && customUi?.editorBar?.useVsCode != false}
			<div class="py-1">
				<Button
					target="_blank"
					href="https://www.windmill.dev/docs/cli_local_dev/vscode-extension"
					variant="subtle"
					unifiedSize="md"
					btnClasses="hidden lg:flex"
					startIcon={{
						icon: Github
					}}
				>
					VScode
				</Button>
			</div>
		{/if}
	</div>
</div>
<SplitPanesWrapper>
	<Splitpanes class="!overflow-visible">
		<Pane bind:size={codePanelSize} minSize={10} class="!overflow-visible">
			{#if lang === 'ansible' && ansibleAlternativeExecutionMode != null}
				<!-- Vertical split for ansible with assets -->
				<Splitpanes horizontal class="!overflow-visible h-full">
					<Pane size={60} minSize={30} class="!overflow-visible">
						{@render editorContent()}
					</Pane>
					<Pane size={40} minSize={20} class="!overflow-visible">
						<div
							class="h-full flex flex-col bg-surface border-l border-gray-200 dark:border-gray-700"
						>
							<div class="p-3 border-b border-gray-200 dark:border-gray-700">
								<h4 class="text-sm font-semibold text-primary">File Browser</h4>
							</div>
							<GitRepoViewer
								gitRepoResourcePath={ansibleAlternativeExecutionMode?.resource || ''}
								gitSshIdentity={ansibleGitSshIdentity}
								bind:commitHashInput={commitHashForGitRepo}
							/>
						</div>
					</Pane>
				</Splitpanes>
			{:else}
				<!-- Original single editor layout -->
				{@render editorContent()}
			{/if}
		</Pane>
		<Pane bind:size={testPanelSize} minSize={0}>
			<div class="flex flex-col h-full">
				{#if showTabs}
					<div transition:slide={{ duration: 200 }}>
						<Tabs bind:selected={selectedTab}>
							<Tab value="main" label="Main" />
							{#if hasPreprocessor}
								<div transition:slide={{ duration: 200, axis: 'x' }}>
									<Tab value="preprocessor" label="Preprocessor" />
								</div>
							{/if}
						</Tabs>
					</div>
				{/if}

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
						/>
					</div>
				{/if}

				<div class="flex justify-center pt-1 relative">
					<div class="absolute top-2 left-2">
						<HideButton
							hidden={false}
							direction="right"
							panelName="Test"
							shortcut="U"
							size="md"
							on:click={() => {
								toggleTestPanel()
							}}
						/>
					</div>
					{#if !(debugMode && isDebuggableScript)}
						<div class="flex flex-row divide-x divide-gray-800 dark:divide-gray-300 items-stretch">
							{#if testIsLoading}
								<Button on:click={jobLoader?.cancelJob} btnClasses="w-full" unifiedSize="md">
									<WindmillIcon
										white={true}
										class="mr-2 text-white"
										height="16px"
										width="20px"
										spin="fast"
									/>
									Cancel
								</Button>
							{:else}
								{@const disableTriggerButton =
									customUi?.previewPanel?.disableTriggerButton === true}
								<Button
									on:click={() => runTest()}
									unifiedSize="md"
									btnClasses="w-full {!disableTriggerButton ? 'rounded-r-none' : ''}"
									variant="accent-secondary"
									startIcon={{ icon: Play, classes: 'animate-none' }}
									shortCut={{ Icon: CornerDownLeft }}
								>
									Test
								</Button>
								{#if !disableTriggerButton}
									<CaptureButton on:openTriggers />
								{/if}
							{/if}
						</div>
					{/if}
					<div class="absolute top-2 right-2"
						><Toggle size="2xs" bind:checked={jsonView} options={{ right: 'JSON' }} /></div
					>
				</div>
				<Splitpanes
					horizontal
					class="!max-h-[calc(100%-{debugMode && isDebuggableScript ? '83' : '43'}px)]"
				>
					<Pane size={33}>
						{#if jsonView}
							<div
								class="py-2"
								style="height: {!schemaHeight || schemaHeight < 600 ? 600 : schemaHeight}px"
								data-schema-picker
							>
								<JsonInputs
									on:select={(e) => {
										if (e.detail) {
											args = e.detail
										}
									}}
									updateOnBlur={false}
									placeholder={`Write args as JSON.<br/><br/>Example:<br/><br/>{<br/>&nbsp;&nbsp;"foo": "12"<br/>}`}
								/>
							</div>
						{:else}
							<div class="px-4">
								<div class="break-words relative font-sans" bind:clientHeight={schemaHeight}>
									{#key argsRender}
										<SchemaForm
											helperScript={{
												source: 'inline',
												code,
												//@ts-ignore
												lang
											}}
											compact
											{schema}
											bind:args
											bind:isValid
											noVariablePicker={customUi?.previewPanel?.disableVariablePicker === true}
											showSchemaExplorer
										/>
									{/key}
								</div>
							</div>
						{/if}
					</Pane>
					<Pane size={67} class="relative">
						<LogPanel
							bind:this={logPanel}
							{lang}
							previewJob={debugMode
								? ({
										id: 'debug',
										logs: $debugState.logs,
										result: $debugState.result,
										success: !$debugState.error,
										type: hasDebugResult ? 'CompletedJob' : 'QueuedJob'
									} as any)
								: testJob}
							{pastPreviews}
							previewIsLoading={debugMode
								? $debugState.running && !$debugState.stopped
								: testIsLoading}
							{editor}
							{diffEditor}
							{args}
							{showCaptures}
							customUi={customUi?.previewPanel}
							showCustomResultPanel={showDebugPanel}
						>
							{#if scriptProgress && !debugMode}
								<!-- Put to the slot in logpanel -->
								<JobProgressBar
									job={testJob}
									{scriptProgress}
									bind:this={jobProgressBar}
									compact={true}
								/>
							{/if}
							{#snippet capturesTab()}
								<div class="h-full p-2">
									<CaptureTable
										bind:this={captureTable}
										{hasPreprocessor}
										canHavePreprocessor={canHavePreprocessor(lang)}
										isFlow={false}
										path={stablePathForCaptures}
										canEdit={true}
										on:applyArgs
										on:updateSchema
										on:addPreprocessor
									/>
								</div>
							{/snippet}
							{#snippet customResultPanel()}
								<DebugPanel
									stackFrames={$debugState.stackFrames}
									scopes={$debugState.scopes}
									variables={$debugState.variables}
									client={dapClient}
									bind:selectedFrameId={selectedDebugFrameId}
								/>
							{/snippet}
						</LogPanel>
					</Pane>
				</Splitpanes>
			</div>
		</Pane>
	</Splitpanes>
</SplitPanesWrapper>

{#snippet editorContent()}
	<div class="h-full !overflow-visible bg-surface dark:bg-[#272D38] relative">
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
			{#if lang === 'ansible' && hasDelegateToGitRepo}
				<Button
					variant="default"
					size="xs"
					onclick={() => (gitRepoResourcePickerOpen = true)}
					startIcon={{ icon: GitBranch }}
					btnClasses="bg-surface hover:bg-surface-hover border border-tertiary/30"
				>
					Delegating to git repo
				</Button>
			{/if}
			{#if testPanelSize === 0}
				<HideButton
					hidden={true}
					direction="right"
					size="md"
					panelName="Test"
					shortcut="U"
					customHiddenIcon={{
						icon: PlayIcon
					}}
					on:click={() => {
						toggleTestPanel()
					}}
					btnClasses="bg-marine-400 hover:bg-marine-200 !text-primary-inverse hover:!text-primary-inverse hover:dark:!text-primary-inverse dark:bg-marine-50 dark:hover:bg-marine-50/70"
					color="marine"
				/>
			{/if}
			{#if !aiChatManager.open && !disableAi}
				{#if customUi?.editorBar?.aiGen != false && SUPPORTED_CHAT_SCRIPT_LANGUAGES.includes(lang ?? '')}
					<HideButton
						hidden={true}
						direction="right"
						panelName="AI"
						shortcut="L"
						size="md"
						usePopoverOverride={!$copilotInfo.enabled}
						customHiddenIcon={{
							icon: WandSparkles
						}}
						btnClasses="!text-ai border border-gray-200 dark:border-gray-600 bg-surface"
						on:click={() => {
							if (!aiChatManager.open) {
								aiChatManager.changeMode(AIMode.SCRIPT)
							}
							aiChatManager.toggleOpen()
						}}
					>
						{#snippet popoverOverride()}
							<div class="text-sm">
								Enable Windmill AI in the <a
									href="{base}/workspace_settings?tab=ai"
									target="_blank"
									class="inline-flex flex-row items-center gap-1"
								>
									workspace settings <ExternalLink size={16} />
								</a>
							</div>
						{/snippet}
					</HideButton>
				{/if}
			{/if}
		</div>

		{#if debugConsoleVisible}
			<!-- Use Splitpanes when debug console is visible for resizing -->
			<Splitpanes horizontal class="h-full !overflow-visible">
				<Pane bind:size={editorPaneSize} minSize={20} class="!overflow-visible">
					{@render editorPane()}
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
			<!-- Normal editor without console -->
			<div class="h-full !overflow-visible">
				{@render editorPane()}
			</div>
		{/if}
	</div>
{/snippet}

{#snippet editorPane()}
	{#key lang}
		<Editor
			lineNumbersMinChars={4}
			folding
			{path}
			bind:code
			bind:websocketAlive
			bind:this={editor}
			{yContent}
			awareness={wsProvider?.awareness}
			on:change={(e) => {
				inferSchema(e.detail)
				// Refresh breakpoint positions when code changes (decorations track their lines)
				if (debugMode && breakpointDecorations.length > 0) {
					refreshBreakpointPositions()
				}
			}}
			on:saveDraft
			on:toggleTestPanel={toggleTestPanel}
			cmdEnterAction={async () => {
				await inferSchema(code)
				runTest()
			}}
			formatAction={async () => {
				await inferSchema(code)
				try {
					localStorage.setItem(path ?? 'last_save', code)
				} catch (e) {
					console.error('Could not save last_save to local storage', e)
				}
				dispatch('format')
			}}
			class="flex flex-1 h-full !overflow-visible"
			scriptLang={lang}
			automaticLayout={true}
			{fixedOverflowWidgets}
			{args}
			{enablePreprocessorSnippet}
			preparedAssetsSqlQueries={preparedSqlQueries.current}
			customTag={tag}
		/>
		<DiffEditor
			className="h-full"
			bind:this={diffEditor}
			modifiedModel={editor?.getModel() as meditor.ITextModel}
			automaticLayout
			defaultLang={scriptLangToEditorLang(lang)}
			{fixedOverflowWidgets}
			buttons={diffMode
				? [
						{
							text: 'See changes history',
							onClick: () => {
								showHistoryDrawer = true
							}
						},
						{
							text: 'Quit diff mode',
							onClick: () => {
								hideDiffMode()
							},
							color: 'red'
						}
					]
				: []}
		/>
	{/key}
{/snippet}

<GitRepoResourcePicker
	bind:open={gitRepoResourcePickerOpen}
	currentResource={ansibleAlternativeExecutionMode?.resource}
	currentCommit={commitHashForGitRepo || ansibleAlternativeExecutionMode?.commit}
	currentInventories={ansibleAlternativeExecutionMode?.inventories_location}
	currentPlaybook={ansibleAlternativeExecutionMode?.playbook}
	gitSshIdentity={ansibleGitSshIdentity}
	on:selected={handleDelegateConfigUpdate}
	on:addInventories={handleAddInventories}
/>

<style global>
	/* Debug breakpoint glyph - red circle in the glyph margin */
	.debug-breakpoint-glyph {
		background-color: #e51400;
		border-radius: 50%;
		width: 10px !important;
		height: 10px !important;
		margin-left: 5px;
		margin-top: 4px;
	}

	/* Current execution line - yellow background */
	.debug-current-line {
		background-color: rgba(255, 238, 0, 0.2);
	}

	/* Current execution line glyph - yellow arrow in the glyph margin */
	.debug-current-line-glyph {
		background-color: #ffcc00;
		clip-path: polygon(0 0, 100% 50%, 0 100%);
		width: 10px !important;
		height: 14px !important;
		margin-left: 5px;
		margin-top: 2px;
	}
</style>
