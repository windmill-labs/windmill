<script lang="ts">
	import { buildWsUrl } from '$lib/wsUrl'
	import { paneMinPercent } from '$lib/utils/splitpaneSizing'
	import { processSecretArgs } from './secretArgUtils'
	import type { Schema, SupportedLanguage } from '$lib/common'
	import {
		type CompletedJob,
		type Job,
		JobService,
		type Preview,
		type ScriptLang,
		type ScriptModule
	} from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { copyToClipboard, emptySchema, sendUserToast } from '$lib/utils'
	import Editor from './Editor.svelte'
	import { inferArgs, inferAssets, inferAnsibleExecutionMode } from '$lib/infer'
	import { parsePipelineAnnotations } from '$lib/components/assets/AssetGraph/parsePipelineAnnotations'
	import { isWorkflowAsCode } from '$lib/components/graph/wacToFlow'
	import WacDiagram from '$lib/components/graph/WacDiagram.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import SchemaForm from './SchemaForm.svelte'
	import PowerShellCommonParams from './PowerShellCommonParams.svelte'
	import LogPanel from './scriptEditor/LogPanel.svelte'
	import EditorBar, {
		EDITOR_BAR_WIDTH_THRESHOLD,
		EDITOR_BAR_HELPERS_COMPACT_THRESHOLD
	} from './EditorBar.svelte'
	import JobLoader from './JobLoader.svelte'
	import JobProgressBar from '$lib/components/jobs/JobProgressBar.svelte'
	import { createEventDispatcher, getContext, onDestroy, onMount, untrack } from 'svelte'
	import { Button } from './common'
	import SplitPanesWrapper from './splitPanes/SplitPanesWrapper.svelte'
	import WindmillIcon from './icons/WindmillIcon.svelte'
	import * as Y from 'yjs'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import { langToExt } from '$lib/editorLangUtils'
	import { WebsocketProvider } from 'y-websocket'
	import Modal from './common/modal/Modal.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import DiffEditor from './DiffEditor.svelte'
	import {
		Bug,
		ChevronDown,
		Copy,
		CornerDownLeft,
		Disc,
		Download,
		ExternalLink,
		Github,
		GitBranch,
		Play,
		PlayIcon,
		Plus,
		Target,
		Terminal,
		Pencil,
		WandSparkles,
		X,
		Zap
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
		signMultiplayerRequest,
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
	import DispatchEventsButton from '$lib/components/runs/DispatchEventsButton.svelte'
	import { SUPPORTED_CHAT_SCRIPT_LANGUAGES } from './copilot/chat/script/core'
	import { getStringError } from './copilot/chat/utils'
	import type { ScriptOptions } from './copilot/chat/ContextManager.svelte'
	import { aiChatManager, AIMode } from './copilot/chat/AIChatManager.svelte'

	// Forward-looking hook for the upcoming session-pane feature: that PR will
	// `setContext('aiChatManager', ...)` from the session wrapper so this editor
	// can detect it and hide its own AI/VS Code controls. On main today nothing
	// sets the context, so `inSessionPane` is always false and these buttons
	// render normally — keep the check anyway to avoid re-touching this file
	// when the session-pane PR lands. Untyped getContext to avoid coupling to
	// the AIChatManager class export (which lives on the chat-visuals PR).
	const inSessionPane = !!getContext('aiChatManager')
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import AssetsDropdownButton from './assets/AssetsDropdownButton.svelte'
	import { canHavePreprocessor } from '$lib/script_helpers'
	import { assetEq, type AssetWithAltAccessType } from './assets/lib'
	import type { ColumnLineage } from './assets/AssetGraph/parsePipelineAnnotations'
	import {
		computeContractMarkers,
		type ContractMarker,
		type SchemaContractGraphContext
	} from './assets/AssetGraph/schemaContracts'
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
	import { createScriptRecording } from './recording/scriptRecording.svelte'
	import { setActiveRecording } from './recording/flowRecording.svelte'
	import type { ScriptRecording } from './recording/types'
	import DropdownV2 from './DropdownV2.svelte'

	interface Props {
		// Exported
		schema?: Schema | any
		code: string
		path: string | undefined
		lang: Preview['language']
		kind?: string | undefined
		autoKind?: string | undefined
		template?:
			| 'pgsql'
			| 'mysql'
			| 'script'
			| 'docker'
			| 'powershell'
			| 'bunnative'
			| 'claudesandbox'
			| 'wac_python'
			| 'wac_typescript'
			| 'ci_test_bun'
			| 'ci_test_python'
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
		// Pipeline editor: an asset-parse failure should also turn the
		// "parsable" badge red — asset lineage is load-bearing there, so a
		// green badge must mean both the main function AND the asset parse
		// succeeded.
		requireValidAssets?: boolean
		args: Record<string, any>
		// Emitted with the test-form's full-schema validity whenever it changes, so
		// a host (the pipeline editor) can gate a data-upload entry's readiness on
		// whether every required field is filled, not just the S3 file. A callback
		// rather than a bindable so we don't hit the `$bindable(default)` ban.
		onIsValidChange?: (isValid: boolean) => void
		// Custom timeout (in seconds) from the script settings. Forwarded to the
		// preview run so "Test" honors the same timeout a deployed run would,
		// instead of silently falling back to the instance default.
		timeout?: number
		selectedTab?: 'main' | 'preprocessor' | 'diagram'
		hasPreprocessor?: boolean
		captureTable?: CaptureTable | undefined
		showCaptures?: boolean
		stablePathForCaptures?: string
		lastSavedCode?: string | undefined
		lastDeployedCode?: string | undefined
		disableAi?: boolean
		assets?: AssetWithAltAccessType[]
		// Body-inferred column lineage (DuckDB SQL AST), surfaced alongside
		// `assets` so the pipeline editor can render inferred column lineage on
		// the live graph. Empty/undefined for non-DuckDB or when the parser
		// build predates the inference.
		inferredColumnLineage?: ColumnLineage[]
		modules?: { [key: string]: ScriptModule } | null
		editorBarRight?: import('svelte').Snippet
		enablePreprocessorSnippet?: boolean
		/**
		 * Layout for the test/preview panel relative to the code editor.
		 * `right` (default) splits horizontally with the test panel on the
		 * right; `bottom` splits vertically with the test panel below — same
		 * orientation the flow editor uses for module step editing.
		 */
		previewLayout?: 'right' | 'bottom'
		/**
		 * Fires whenever the test/preview run state changes — used by the
		 * pipeline editor to mirror the running script onto the canvas
		 * (animate its incoming/outgoing edges) so Test feels equivalent
		 * to clicking a run button on the graph.
		 */
		onTestStateChange?: (running: boolean) => void
		// Fired whenever a test run is started from this editor, with the
		// preview job id. Used by whitelabel embedders to track test jobs.
		onTestJob?: (e: { jobId: string }) => void
		// When true the right-hand test/run pane mounts collapsed. The user
		// can still expand it via `toggleTestPanel`. Defaults to false so the
		// regular /scripts/edit route keeps its current open-by-default UX;
		// the session preview opts in to save vertical real estate.
		initialTestPanelCollapsed?: boolean
		// Producer-side facts for the live schema-contract diagnostics
		// (`on_schema_change=ignore` suppression + scd2 `_current` fallback),
		// built by the pipeline page from the resolved graph. Absent outside the
		// pipeline editor — the check still runs, just without suppression.
		schemaContractContext?: SchemaContractGraphContext
	}

	let {
		schema = $bindable(),
		code = $bindable(),
		path,
		lang,
		kind = undefined,
		autoKind = undefined,
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
		requireValidAssets = false,
		args = $bindable(),
		onIsValidChange,
		timeout = undefined,
		selectedTab = $bindable('main'),
		hasPreprocessor = $bindable(false),
		captureTable = $bindable(undefined),
		showCaptures = true,
		stablePathForCaptures = '',
		lastSavedCode = undefined,
		lastDeployedCode = undefined,
		disableAi = false,
		assets = $bindable(),
		inferredColumnLineage = $bindable(),
		modules = $bindable(undefined),
		editorBarRight,
		enablePreprocessorSnippet = false,
		previewLayout = 'right',
		onTestStateChange,
		onTestJob,
		initialTestPanelCollapsed = false,
		schemaContractContext = undefined
	}: Props = $props()

	$effect(() => {
		onTestStateChange?.(testIsLoading)
	})

	let initialArgs = structuredClone($state.snapshot(args))
	let jsonView = $state(false)
	let schemaHeight = $state(0)
	// Asset-trigger cascade choice for the Test button. Only meaningful when
	// `customUi.previewPanel.downstreamSubscribers > 0`. Defaults to off so
	// iteration on a pipeline script doesn't accidentally fan out runs to
	// downstream subscribers. The user can flip it via the split-button
	// caret next to Test, and the choice sticks for the rest of the session.
	let cascadeDownstream = $state(false)
	let cascadeMenuOpen = $state(false)
	let psCommonParams: Record<string, any> = $state({})
	let showPsCommonParams = $derived(lang === 'powershell' && /^\s*\[CmdletBinding/im.test(code))

	// Module tab state
	let activeModuleTab: string | null = $state(null)
	// Per-module test panel state (args + schema), persisted across tab switches
	let moduleTestState: Record<string, { args: Record<string, any>; schema: Schema }> = $state({})
	let testPanelArgs: Record<string, any> = $state({})
	let testPanelSchema: Schema = $state(emptySchema())
	// editorCode is what the editor shows; code always holds the main script content
	let editorCode: string = $state(code)
	// Sync editorCode when code changes externally (template reset, copilot,
	// new-script template init, etc.). We also re-run schema inference in that
	// case — otherwise if the code prop is set after ScriptEditor's onMount (as
	// happens on /scripts/add when initContent sets script.content after the
	// editor has already mounted with an empty string), the schema would stay
	// empty until the user typed into the editor.
	let lastSyncedCode = code
	$effect.pre(() => {
		if (activeModuleTab === null && code !== lastSyncedCode) {
			editorCode = code
			lastSyncedCode = code
			editor?.setCode(editorCode) // immediate sync, don't wait for the 800ms debounce
			untrack(() => inferSchema(code))
		}
	})

	function switchToModule(modulePath: string) {
		if (activeModuleTab !== null && modules && activeModuleTab !== modulePath) {
			// Switching from another module: save its content and test state
			modules[activeModuleTab] = { ...modules[activeModuleTab], content: editorCode }
			moduleTestState[activeModuleTab] = { args: testPanelArgs, schema: testPanelSchema }
		}
		if (modules && modules[modulePath]) {
			activeModuleTab = modulePath
			editorCode = modules[modulePath].content
			editor?.setCode(editorCode)
			// Restore or initialize test state for the new module
			if (moduleTestState[modulePath]) {
				testPanelArgs = moduleTestState[modulePath].args
				testPanelSchema = moduleTestState[modulePath].schema
			} else {
				testPanelArgs = {}
				testPanelSchema = emptySchema()
				inferModuleSchema()
			}
		}
	}

	function switchToMain() {
		if (activeModuleTab !== null && modules) {
			// Save current module content and test state
			modules[activeModuleTab] = { ...modules[activeModuleTab], content: editorCode }
			moduleTestState[activeModuleTab] = { args: testPanelArgs, schema: testPanelSchema }
		}
		activeModuleTab = null
		editorCode = code
		lastSyncedCode = code
		editor?.setCode(editorCode)
	}

	let effectiveLang = $derived(
		activeModuleTab && modules?.[activeModuleTab]
			? (modules[activeModuleTab].language as Preview['language'])
			: lang
	)

	let isWacV2 = $derived.by(() => {
		const mainCode = code
		const isTsWac =
			mainCode.includes('windmill-client') &&
			mainCode.includes('workflow') &&
			mainCode.includes('task')
		const isPyWac =
			(mainCode.includes('import wmill') || mainCode.includes('from wmill')) &&
			mainCode.includes('workflow') &&
			mainCode.includes('task')
		return isTsWac || isPyWac
	})
	let supportsModules = $derived((lang === 'bun' || lang === 'python3') && isWacV2)
	let mainFileName = $derived('script.' + langToExt(scriptLangToEditorLang(lang)))

	let modulePathInput = $state('')
	let showAddModulePopover = $state(false)
	let modulePathInputEl: HTMLInputElement | undefined = $state(undefined)
	let modulePathError = $state('')

	let renameModuleInput = $state('')
	let renameModuleError = $state('')
	let renameModuleInputEl: HTMLInputElement | undefined = $state(undefined)

	const ALL_MODULE_EXTENSIONS: Record<string, ScriptModule['language']> = {
		'.ts': 'bun',
		'.py': 'python3',
		'.go': 'go',
		'.sh': 'bash',
		'.ps1': 'powershell',
		'.sql': 'postgresql',
		'.gql': 'graphql',
		'.php': 'php',
		'.rs': 'rust',
		'.yml': 'ansible',
		'.cs': 'csharp',
		'.nu': 'nu',
		'.java': 'java',
		'.rb': 'ruby'
	}

	/** Map main script language to allowed module file extensions. */
	const LANG_MODULE_EXTENSIONS: Partial<Record<Preview['language'] & string, string[]>> = {
		python3: ['.py'],
		bun: ['.ts'],
		deno: ['.ts'],
		nativets: ['.ts'],
		go: ['.go'],
		bash: ['.sh'],
		powershell: ['.ps1'],
		postgresql: ['.sql'],
		mysql: ['.sql'],
		bigquery: ['.sql'],
		snowflake: ['.sql'],
		mssql: ['.sql'],
		oracledb: ['.sql'],
		duckdb: ['.sql'],
		graphql: ['.gql'],
		php: ['.php'],
		rust: ['.rs'],
		ansible: ['.yml'],
		csharp: ['.cs'],
		nu: ['.nu'],
		java: ['.java'],
		ruby: ['.rb'],
		bunnative: ['.ts']
	}

	let allowedModuleExtensions = $derived(
		lang
			? (LANG_MODULE_EXTENSIONS[lang] ?? Object.keys(ALL_MODULE_EXTENSIONS))
			: Object.keys(ALL_MODULE_EXTENSIONS)
	)

	function inferModuleLang(filePath: string): ScriptModule['language'] | undefined {
		for (const [ext, moduleLang] of Object.entries(ALL_MODULE_EXTENSIONS)) {
			if (filePath.endsWith(ext)) return moduleLang
		}
		return undefined
	}

	function getModuleDefaultContent(filePath: string): string {
		if (filePath.endsWith('.py')) {
			return `def hello() -> str:\n    return "world"\n`
		} else if (filePath.endsWith('.ts')) {
			return `export function hello(): string {\n  return "world"\n}\n`
		} else if (filePath.endsWith('.go')) {
			return `package inner\n\nfunc Hello() string {\n\treturn "world"\n}\n`
		} else if (filePath.endsWith('.sh')) {
			return `#!/bin/bash\necho "world"\n`
		} else if (filePath.endsWith('.ps1')) {
			return `function Hello {\n    return "world"\n}\n`
		} else if (filePath.endsWith('.sql')) {
			return `SELECT 'world' as result;\n`
		} else if (filePath.endsWith('.gql')) {
			return `query Hello {\n  hello\n}\n`
		} else if (filePath.endsWith('.php')) {
			return `<?php\nfunction hello(): string {\n    return "world";\n}\n`
		} else if (filePath.endsWith('.rs')) {
			return `pub fn hello() -> String {\n    "world".to_string()\n}\n`
		} else if (filePath.endsWith('.yml')) {
			return `---\n- name: Hello\n  debug:\n    msg: "world"\n`
		} else if (filePath.endsWith('.cs')) {
			return `public static string Hello() {\n    return "world";\n}\n`
		} else if (filePath.endsWith('.nu')) {
			return `def hello [] {\n    "world"\n}\n`
		} else if (filePath.endsWith('.java')) {
			return `public class Helper {\n    public static String hello() {\n        return "world";\n    }\n}\n`
		} else if (filePath.endsWith('.rb')) {
			return `def hello\n  "world"\nend\n`
		}
		return ''
	}

	function validateModulePath(path: string): string {
		if (!path.trim()) return ''
		const moduleLang = inferModuleLang(path)
		if (!moduleLang) {
			const exts = allowedModuleExtensions.join(', ')
			return `File must end with a supported extension: ${exts}`
		}
		const matchedExt = allowedModuleExtensions.find((ext) => path.endsWith(ext))
		if (!matchedExt) {
			const exts = allowedModuleExtensions.join(', ')
			return `File must end with a supported extension for this language: ${exts}`
		}
		if (modules?.[path.trim()]) {
			return `Module ${path.trim()} already exists`
		}
		return ''
	}

	function addModule() {
		const modulePath = modulePathInput.trim()
		if (!modulePath) return
		const error = validateModulePath(modulePath)
		if (error) {
			modulePathError = error
			return
		}
		if (!modules) {
			modules = {}
		}
		modules[modulePath] = {
			content: getModuleDefaultContent(modulePath),
			language: inferModuleLang(modulePath)!
		}
		modulePathInput = ''
		modulePathError = ''
		showAddModulePopover = false
		switchToModule(modulePath)
	}

	function removeModule(modulePath: string) {
		if (!modules) return
		if (activeModuleTab === modulePath) {
			switchToMain()
		}
		delete modules[modulePath]
		delete moduleTestState[modulePath]
		modules = { ...modules }
	}

	function validateRenameModulePath(newPath: string, oldPath: string): string {
		if (!newPath.trim()) return ''
		const moduleLang = inferModuleLang(newPath)
		if (!moduleLang) {
			const exts = allowedModuleExtensions.join(', ')
			return `File must end with a supported extension: ${exts}`
		}
		const matchedExt = allowedModuleExtensions.find((ext) => newPath.endsWith(ext))
		if (!matchedExt) {
			const exts = allowedModuleExtensions.join(', ')
			return `File must end with a supported extension for this language: ${exts}`
		}
		if (newPath.trim() !== oldPath && modules?.[newPath.trim()]) {
			return `Module ${newPath.trim()} already exists`
		}
		return ''
	}

	function renameModule(oldPath: string) {
		const newPath = renameModuleInput.trim()
		if (!newPath || newPath === oldPath) {
			return
		}
		const error = validateRenameModulePath(newPath, oldPath)
		if (error) {
			renameModuleError = error
			return
		}
		if (!modules) return
		const mod = modules[oldPath]
		const newLang = inferModuleLang(newPath)
		delete modules[oldPath]
		modules[newPath] = { ...mod, language: newLang ?? mod.language }
		modules = { ...modules }
		if (moduleTestState[oldPath]) {
			moduleTestState[newPath] = moduleTestState[oldPath]
			delete moduleTestState[oldPath]
		}
		if (activeModuleTab === oldPath) {
			activeModuleTab = newPath
		}
		renameModuleInput = ''
		renameModuleError = ''
	}

	/** Save the active module tab's editor content back into the modules map (no UI side-effects). */
	function flushModuleContent() {
		if (activeModuleTab !== null && modules) {
			modules[activeModuleTab] = { ...modules[activeModuleTab], content: editorCode }
		}
	}

	/** Flush module content and reset the editor back to the main script tab. */
	export function flushModuleState() {
		if (activeModuleTab !== null && modules) {
			flushModuleContent()
			moduleTestState[activeModuleTab] = { args: testPanelArgs, schema: testPanelSchema }
			activeModuleTab = null
			editorCode = code
		}
	}

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
	// Asset-parse validity for the editor badge. `undefined` while loading (so
	// the badge doesn't flicker red); only an explicit parser error counts as
	// invalid. Surfaced only when `requireValidAssets` (pipeline editor).
	let validAssets = $derived(
		requireValidAssets ? inferAssetsRes.current?.status !== 'error' : undefined
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
			if (!inferAssetsRes.current || inferAssetsRes.current?.status === 'error') {
				// Clear stale lineage on parse error / unset, so a script switch
				// whose new body fails to parse can't leave the previous script's
				// inferred column lineage bound to the new path.
				if (inferredColumnLineage !== undefined) inferredColumnLineage = undefined
				return
			}
			let newAssets = inferAssetsRes.current.assets as AssetWithAltAccessType[]
			for (const asset of newAssets) {
				const old = assets?.find((a) => assetEq(a, asset))
				if (old?.alt_access_type) asset.alt_access_type = old.alt_access_type
			}
			const normalizedAssets = newAssets.length > 0 ? newAssets : undefined
			if (!deepEqual(assets, normalizedAssets)) assets = normalizedAssets

			const newLineage = inferAssetsRes.current.column_lineage
			const normalizedLineage = newLineage && newLineage.length > 0 ? newLineage : undefined
			if (!deepEqual(inferredColumnLineage, normalizedLineage))
				inferredColumnLineage = normalizedLineage
		}
	)

	// Live schema-contract diagnostics (pipelines gap #2b): diff the buffer's
	// asset refs against the captured producer schemas and surface mismatches
	// as Monaco warning squiggles — the as-you-type mirror of the authoritative
	// save-time check. The result is a prop on Editor (not an imperative call)
	// because this can resolve before Monaco initializes on mount. Sequenced so
	// a slow schema fetch can't overwrite the markers of a newer keystroke.
	let contractMarkers: ContractMarker[] = $state([])
	let contractCheckSeq = 0
	watch([() => inferAssetsRes.current, () => schemaContractContext], () => {
		const res = inferAssetsRes.current
		const workspace = $workspaceStore
		const seq = ++contractCheckSeq
		if (!workspace || !res || res.status === 'error') {
			contractMarkers = []
			return
		}
		const bufferCode = code
		computeContractMarkers(
			workspace,
			bufferCode,
			(res.assets ?? []) as AssetWithAltAccessType[],
			schemaContractContext
		)
			.then((markers) => {
				if (seq === contractCheckSeq) contractMarkers = markers
			})
			.catch((e) => console.error('schema-contract diagnostics failed', e))
	})

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
	// Mirror the test-form validity out to an optional host callback.
	$effect(() => {
		onIsValidChange?.(isValid)
	})
	let scriptProgress = $state(undefined)

	let logPanel: LogPanel | undefined = $state(undefined)
	// Test
	let testIsLoading = $state(false)
	let testJob: Job | undefined = $state()
	let pastPreviews: CompletedJob[] = $state([])
	let historyTabActive = false
	let pastPreviewsRequest: ReturnType<typeof JobService.listCompletedJobs> | undefined
	let validCode = $state(true)

	// Recording
	let scriptRecording = createScriptRecording()
	let lastRecording: ScriptRecording | undefined = $state(undefined)

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
	let debugMode = $state(false)
	let debugBreakpoints = new SvelteSet<number>()
	let breakpointDecorations: string[] = $state([])
	let currentLineDecoration: string[] = $state([])
	let hoverBreakpointDecoration: string[] = $state([])
	// Line currently showing the ghost breakpoint, used to short-circuit redundant
	// deltaDecorations calls on every mousemove event.
	let hoverBreakpointLine: number | null = null
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

	// Ghost breakpoint shown while hovering the gutter on a line without a breakpoint
	const hoverBreakpointDecorationType: meditor.IModelDecorationOptions = {
		glyphMarginClassName: 'debug-breakpoint-glyph-hover',
		glyphMarginHoverMessage: { value: 'Click to add a breakpoint' },
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

	export async function runTest(opts?: { cascade?: boolean; skipDdlGuard?: boolean }) {
		// Intercept DDL statements (offer to turn them into data table migrations)
		// on every run path, not just the editor's Cmd+Enter. `skipDdlGuard` is set
		// by the Cmd+Enter action, which already guarded before calling us.
		if (!opts?.skipDdlGuard) {
			if ((await editor?.guardDdlBeforeRun()) === false) return
			// The guard may have rewritten the code (migrated statements stripped);
			// `editorCode` is kept in sync by the editor binding, so mirror the
			// on:change handler and pull it into `code` before we run.
			if (activeModuleTab === null) code = editorCode
		}
		// When the caller forces a cascade choice (e.g. the canvas runnable
		// menu's "Run + trigger N downstream"), also flip the persistent
		// `cascadeDownstream` state so the split button's label/icon reflect
		// the active mode after the run kicks off — keeps "what mode am I in"
		// visible instead of having a one-off run silently disagree with the
		// UI. The caller is welcome to bump the signal repeatedly; we just
		// keep the latest choice as the active mode.
		if (opts?.cascade !== undefined) cascadeDownstream = opts.cascade
		// Discard any previous recording when running a normal test
		if (!scriptRecording.active) {
			lastRecording = undefined
		}
		// Not defined if JobProgressBar not loaded
		jobProgressBar?.reset()
		// Flush module edits back to modules map before running preview
		flushModuleContent()

		const testCode = activeModuleTab !== null ? editorCode : code
		const testLang = activeModuleTab !== null ? effectiveLang : lang
		const rawTestArgs =
			activeModuleTab !== null
				? testPanelArgs
				: selectedTab === 'preprocessor' || kind === 'preprocessor'
					? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...(args ?? {}) }
					: (args ?? {})
		const testSchema = activeModuleTab !== null ? testPanelSchema : schema
		const testArgs = await processSecretArgs(rawTestArgs, testSchema)
		if (showPsCommonParams) {
			for (const [k, v] of Object.entries(psCommonParams)) {
				if (v !== undefined && v !== false && v !== '') {
					testArgs[k] = v
				}
			}
		}
		// In pipeline-editor contexts (downstreamSubscribers prop set), default
		// the test run to *just this step* by passing the backend opt-out arg.
		// The user can run with cascade via the split button's caret option,
		// which flips `cascadeDownstream` and lets dispatch fire normally.
		if ((customUi?.previewPanel?.downstreamSubscribers ?? 0) > 0 && !cascadeDownstream) {
			testArgs['_wmill_skip_asset_dispatch'] = true
		}

		//@ts-ignore
		let job = await jobLoader.runPreview(
			path,
			testCode,
			testLang,
			testArgs,
			tag,
			undefined,
			undefined,
			{
				done(_x) {
					if (scriptRecording.active) {
						lastRecording = scriptRecording.stop()
						setActiveRecording(undefined)
					}
					if (historyTabActive) {
						loadPastTests()
					}
				},
				doneError({ error }) {
					if (scriptRecording.active) {
						lastRecording = scriptRecording.stop()
						setActiveRecording(undefined)
					}
					console.error(error)
				}
			},
			undefined,
			activeModuleTab !== null ? undefined : modules,
			undefined,
			timeout
		)
		if (job) {
			onTestJob?.({ jobId: job })
		}
		logPanel?.setFocusToLogs()
		return job
	}

	async function recordAndTest() {
		lastRecording = undefined
		scriptRecording.start(path ?? '', code, lang ?? '', args ?? {}, schema)
		setActiveRecording(scriptRecording)
		await runTest()
	}

	function downloadRecording() {
		if (lastRecording) {
			scriptRecording.download(lastRecording)
		}
	}

	async function loadPastTests(): Promise<void> {
		pastPreviewsRequest?.cancel()
		const req = JobService.listCompletedJobs({
			workspace: $workspaceStore!,
			jobKinds: 'preview',
			createdBy: $userStore?.username,
			scriptPathExact: path,
			hasNullParent: true
		})
		pastPreviewsRequest = req
		try {
			const result = await req
			if (pastPreviewsRequest === req) {
				pastPreviews = result
			}
		} catch (err) {
			if (!(err instanceof Error) || err.name !== 'CancelError') {
				throw err
			}
		} finally {
			if (pastPreviewsRequest === req) {
				pastPreviewsRequest = undefined
			}
		}
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
					(selectedTab === 'preprocessor' ? !result?.auto_kind : result?.has_preprocessor) ?? false

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
			injectPartitionArg(nschema, args, nlang ?? lang, code)
			schema = nschema
		} catch (e) {
			validCode = false
		}
	}

	// A `// partitioned` pipeline script is materialized one slice at a time and
	// receives the slice as a runtime `partition` arg (the cascade injects it in
	// production). It isn't a code parameter, so schema inference doesn't see it —
	// surface it in the test form so a partitioned script can be run manually.
	function injectPartitionArg(
		s: any,
		a: Record<string, any> | undefined,
		l: string | undefined,
		c: string
	) {
		try {
			if (l !== 'duckdb' || !s?.properties) return
			const part = parsePipelineAnnotations(c).partition
			if (!part) return
			// Date-based partition kinds render a date / datetime picker; a dynamic
			// key is a free-form string.
			const format =
				part.kind === 'hourly'
					? 'date-time'
					: part.kind === 'daily' || part.kind === 'weekly' || part.kind === 'monthly'
						? 'date'
						: undefined
			if (!s.properties['partition']) {
				s.properties['partition'] = {
					type: 'string',
					...(format ? { format } : {}),
					// ISO output so partition keys sort lexicographically (the date
					// picker defaults to dd-MM-yyyy otherwise).
					...(format === 'date' ? { dateFormat: 'yyyy-MM-dd' } : {}),
					description:
						part.kind === 'dynamic'
							? 'Partition key value to materialize.'
							: `Partition (${part.kind}) to materialize.`
				}
				if (Array.isArray(s.order) && !s.order.includes('partition')) {
					s.order = ['partition', ...s.order]
				}
			}
			// Pre-fill the *test* arg with the current slice for date kinds — a
			// convenience default, kept on the args (not baked into the schema,
			// where it would persist to the deployed script and go stale).
			if (format && a && (a['partition'] == null || a['partition'] === '')) {
				const now = new Date()
				a['partition'] =
					format === 'date' ? now.toISOString().slice(0, 10) : now.toISOString().slice(0, 16)
			}
		} catch (e) {}
	}

	async function inferModuleSchema() {
		if (activeModuleTab === null) return
		try {
			await inferArgs(effectiveLang, editorCode, testPanelSchema)
			injectPartitionArg(testPanelSchema, testPanelArgs, effectiveLang, editorCode)
			moduleTestState[activeModuleTab] = { args: testPanelArgs, schema: testPanelSchema }
		} catch (e) {
			// Module code may be in-progress; silently ignore
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
		clearHoverBreakpointDecoration()
	}

	function updateHoverBreakpointDecoration(line: number): void {
		if (hoverBreakpointLine === line) return
		const monacoEditor = editor?.getEditor?.()
		if (!monacoEditor) return

		const decorations = [
			{
				range: { startLineNumber: line, startColumn: 1, endLineNumber: line, endColumn: 1 },
				options: hoverBreakpointDecorationType
			}
		]

		const oldDecorations = untrack(() => hoverBreakpointDecoration)
		hoverBreakpointDecoration = monacoEditor.deltaDecorations(oldDecorations, decorations)
		hoverBreakpointLine = line
	}

	function clearHoverBreakpointDecoration(): void {
		if (hoverBreakpointLine === null) return
		const monacoEditor = editor?.getEditor?.()
		if (!monacoEditor) return
		const oldDecorations = untrack(() => hoverBreakpointDecoration)
		if (oldDecorations.length > 0) {
			hoverBreakpointDecoration = monacoEditor.deltaDecorations(oldDecorations, [])
		}
		hoverBreakpointLine = null
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
			debugMode = true
		}
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
				if (e.target.type === meditor.MouseTargetType.GUTTER_GLYPH_MARGIN) {
					const line = e.target.position?.lineNumber
					if (line) {
						toggleBreakpoint(line)
					}
				}
			})

			// Show a ghost breakpoint while hovering anywhere in the gutter on an empty line.
			// Hover area is intentionally wider than the click target — clicks still only
			// toggle when landing on the glyph margin itself, but the ghost helps users find it.
			const mouseMoveDisposable = monacoEditor.onMouseMove((e) => {
				const t = e.target.type
				const isGutter =
					t === meditor.MouseTargetType.GUTTER_GLYPH_MARGIN ||
					t === meditor.MouseTargetType.GUTTER_LINE_NUMBERS ||
					t === meditor.MouseTargetType.GUTTER_LINE_DECORATIONS
				const line = e.target.position?.lineNumber
				if (isGutter && line && !debugBreakpoints.has(line)) {
					updateHoverBreakpointDecoration(line)
				} else {
					clearHoverBreakpointDecoration()
				}
			})

			const mouseLeaveDisposable = monacoEditor.onMouseLeave(() => {
				clearHoverBreakpointDecoration()
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
				mouseMoveDisposable.dispose()
				mouseLeaveDisposable.dispose()
				clearHoverBreakpointDecoration()
				// Disable glyph margin when exiting debug mode
				monacoEditor.updateOptions({ glyphMargin: false })
			}
		} else {
			// Ensure glyph margin is disabled when not in debug mode
			monacoEditor.updateOptions({ glyphMargin: false })
		}
	})

	onMount(async () => {
		await inferSchema(code, { applyInitialArgs: true })
		// Retry once if the initial inference failed silently (e.g. transient WASM
		// init race). Without this, users had to modify the code to trigger a
		// second on:change-driven inference.
		if (!validCode && code && lang) {
			await inferSchema(code, { applyInitialArgs: true })
		}
		aiChatManager.saveAndClear()
		aiChatManager.changeMode(AIMode.SCRIPT)
		if (customUi?.previewPanel?.loadLastRunOnMount) {
			void loadLastRunIntoTestPanel()
		}
	})

	// Pull the most recent top-level completed job for this script path and
	// feed it through the JobLoader so the preview pane renders its logs +
	// result. Best-effort: any error (no job yet, network blip) leaves the
	// panel empty rather than surfacing a toast — this is a passive "show
	// what's there" affordance, not a user action. Skipped when a test is
	// already running so a live job's stream is never clobbered.
	async function loadLastRunIntoTestPanel(): Promise<void> {
		if (!path || !$workspaceStore) return
		if (testIsLoading || testJob !== undefined) return
		try {
			const jobs = await JobService.listCompletedJobs({
				workspace: $workspaceStore,
				scriptPathExact: path,
				hasNullParent: true,
				perPage: 1,
				orderDesc: true
			})
			const lastId = jobs[0]?.id
			if (lastId && !testIsLoading && testJob === undefined) {
				await jobLoader?.watchJob(lastId)
			}
		} catch (e) {
			// Silent: empty preview pane is the natural fallback.
		}
	}

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

		let token: string | undefined
		try {
			token = await signMultiplayerRequest($workspaceStore ?? '')
		} catch (e) {
			console.error('Failed to sign multiplayer request:', e)
			sendUserToast('Failed to authorize multiplayer session', true)
			return
		}

		const ydoc = new Y.Doc()
		if (wsProvider) {
			wsProvider.destroy()
		}
		let yContentInit = ydoc.getText('content')

		wsProvider = new WebsocketProvider(
			buildWsUrl('/ws_mp/'),
			$workspaceStore + '/' + (path ?? 'no-room-name'),
			ydoc,
			{ connect: false, params: { token } }
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
		pastPreviewsRequest?.cancel()
		pastPreviewsRequest = undefined
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

	const WAC_CONTEXT_LANGUAGES = ['python3', 'bun']
	let isWac = $derived(
		template === 'wac_python' ||
			template === 'wac_typescript' ||
			autoKind === 'wac' ||
			(code && lang ? isWorkflowAsCode(code, lang) : false)
	)
	let workflowAsCodeAiContext = $derived(
		activeModuleTab === null && isWac && WAC_CONTEXT_LANGUAGES.includes(lang ?? '')
	)
	let showTabs = $derived(hasPreprocessor || isWac)
	$effect(() => {
		!hasPreprocessor && (selectedTab = 'main')
	})
	$effect(() => {
		// Only depend on selectedTab (preprocessor ↔ main toggle).
		// Code changes are handled by the editor on:change handler and
		// explicit inferSchema calls (initContent, onMount), so we read
		// `code` inside untrack to avoid a redundant double-inference race.
		selectedTab && untrack(() => code && inferSchema(code))
	})

	let argsRender = $state(0)
	export async function updateArgs(newArgs: Record<string, any>) {
		if (Object.keys(newArgs).length > 0) {
			args = { ...newArgs }
			argsRender++
		}
	}

	setContext('disableTooltips', untrack(() => customUi)?.disableTooltips === true)

	// Pixel minimum size for the test pane while open. The pane's `minSize`
	// is a percentage of the *split axis* (width for `right` layout, height
	// for `bottom`), so we derive it from whichever container dimension the
	// splitter actually distributes. Without this layout switch, the bottom
	// layout would translate a 400px width-fraction into a height-fraction,
	// which on wide screens is far too tall and on tall narrow screens far
	// too short.
	let splitContainerWidth = $state(0)
	let splitContainerHeight = $state(0)
	const testPaneMinPx = $derived(previewLayout === 'bottom' ? 220 : 400)
	const splitAxisExtent = $derived(
		previewLayout === 'bottom' ? splitContainerHeight : splitContainerWidth
	)
	const testPaneMinPercent = $derived(paneMinPercent(splitAxisExtent, testPaneMinPx))

	// Raw user-controlled test size (what the splitter wrote, or what the
	// toggle set). The size we actually pass to <Pane> is clamped to the
	// dynamic minimum below — so when the editor shrinks, the displayed test
	// pane grows to honor the new minimum without needing an effect. The code
	// pane's size is purely derived from it (100 - test).
	// `initialTestPanelCollapsed` seeds the raw value at 0 (collapsed) while
	// keeping the "remembered" size at 30, so the user's first toggle expands
	// the pane to a sensible width rather than 0.
	let rawTestPanelSize = $state(untrack(() => (initialTestPanelCollapsed ? 0 : 30)))
	let storedTestPanelSize = 30
	const testPanelSize = $derived(
		rawTestPanelSize === 0 ? 0 : Math.max(rawTestPanelSize, testPaneMinPercent)
	)
	const codePanelSize = $derived(100 - testPanelSize)

	function toggleTestPanel() {
		if (testPanelSize > 0) {
			// Store the raw (unclamped) preference so reopening on a wider screen
			// restores the user's intent, not the pixel-min that inflated the pane.
			storedTestPanelSize = rawTestPanelSize
			rawTestPanelSize = 0
		} else {
			rawTestPanelSize = Math.max(storedTestPanelSize, testPaneMinPercent)
		}
	}

	// When the compact preview shows a SchemaForm above the logs
	// (`argsAboveLogs`), give the preview pane extra height so the args
	// form doesn't shrink the logs/result area. This is a deliberate
	// $effect (not $derived): the schema loads async and the same
	// code/test sizes are two-way bound to the splitpane drag handle, so
	// the offset is folded into the user-resizable state once on the
	// has-args transition — manual resizing still works afterwards.
	let hasArgsAboveLogs = $derived(
		customUi?.previewPanel?.argsAboveLogs === true &&
			!!schema?.properties &&
			Object.keys(schema.properties).length > 0
	)
	let argsHeightBonus = $state(0)
	$effect(() => {
		const want = hasArgsAboveLogs ? 18 : 0
		untrack(() => {
			const delta = want - argsHeightBonus
			if (delta === 0) return
			if (testPanelSize > 0) {
				rawTestPanelSize += delta
			} else {
				// preview collapsed — bake the bonus into the size it
				// restores to so it lands correctly on expand.
				storedTestPanelSize += delta
			}
			argsHeightBonus = want
		})
	})

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
		;[
			editor,
			lastSavedCode,
			lastDeployedCode,
			diffMode,
			workflowAsCodeAiContext,
			args,
			error,
			lang,
			path
		]
		untrack(() => {
			const options: ScriptOptions = {
				getCode: () => code,
				lang: lang as ScriptLang,
				error,
				args: args ?? {},
				path,
				lastSavedCode,
				lastDeployedCode,
				diffMode,
				workflowAsCode: workflowAsCodeAiContext
			}
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
				{validAssets}
				iconOnly={width < EDITOR_BAR_WIDTH_THRESHOLD}
				compactHelpers={width < EDITOR_BAR_HELPERS_COMPACT_THRESHOLD}
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
					{@render editorBarRight?.()}
				{/snippet}
			</EditorBar>
		{/if}
		{#if !noSyncFromGithub && customUi?.editorBar?.useVsCode != false && !inSessionPane}
			<div class="py-1">
				<Button
					target="_blank"
					href="https://www.windmill.dev/docs/cli_local_dev/vscode-extension"
					variant="subtle"
					unifiedSize="md"
					iconOnly={width < EDITOR_BAR_WIDTH_THRESHOLD}
					title="Use VS Code"
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
	<div
		bind:clientWidth={splitContainerWidth}
		bind:clientHeight={splitContainerHeight}
		class="h-full"
	>
		<Splitpanes class="!overflow-visible" horizontal={previewLayout === 'bottom'}>
			<Pane size={codePanelSize} minSize={0} class="!overflow-visible">
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
			<Pane
				bind:size={() => testPanelSize, (v) => (rawTestPanelSize = v)}
				minSize={testPaneMinPercent}
				class={customUi?.previewPanel?.hideArgs ? '!overflow-visible' : ''}
			>
				<div
					data-test-panel
					class={customUi?.previewPanel?.hideArgs
						? 'flex flex-col h-full !overflow-visible'
						: 'flex flex-col h-full'}
				>
					{#if showTabs}
						<div transition:slide={{ duration: 200 }}>
							<Tabs bind:selected={selectedTab}>
								<Tab value="main" label="Main" />
								{#if hasPreprocessor}
									<div transition:slide={{ duration: 200, axis: 'x' }}>
										<Tab value="preprocessor" label="Preprocessor" />
									</div>
								{/if}
								{#if isWac}
									<div transition:slide={{ duration: 200, axis: 'x' }}>
										<Tab value="diagram" label="Diagram" />
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

					{#if selectedTab === 'diagram'}
						<div class="flex-1 min-h-0">
							<WacDiagram {code} language={lang ?? ''} />
						</div>
					{:else}
						{#if previewLayout !== 'bottom'}
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
									<div class="flex flex-row gap-2">
										<div
											class="flex flex-row divide-x divide-gray-800 dark:divide-gray-300 items-stretch"
										>
											{#if testIsLoading}
												{@render cancelTestButton('md', 'w-full')}
											{:else}
												{@const disableTriggerButton =
													customUi?.previewPanel?.disableTriggerButton === true}
												{@render runTestButton(
													'md',
													`w-full ${!disableTriggerButton ? 'rounded-r-none' : ''}`
												)}
												{#if !disableTriggerButton}
													<CaptureButton on:openTriggers />
												{/if}
											{/if}
										</div>
										{#if lastRecording}
											<Button
												on:click={downloadRecording}
												unifiedSize="md"
												startIcon={{ icon: Download }}
												iconOnly
												title="Download recording"
											/>
										{/if}
									</div>
								{/if}
								<div class="absolute top-2 right-2 flex items-center gap-2">
									{#if customUi?.previewPanel?.disableJsonView !== true}
										<Toggle size="2xs" bind:checked={jsonView} options={{ right: 'JSON' }} />
									{/if}
									<DropdownV2
										size="xs"
										items={[
											{
												displayName: 'Test & record',
												icon: Disc,
												action: () => recordAndTest()
											}
										]}
									/>
								</div>
							</div>
						{/if}
						{#if customUi?.previewPanel?.hideArgs}
							<!-- Compact preview layout used by the pipeline editor:
						     no args column (the script is known to take no
						     inputs), LogPanel takes the full width, with a
						     small Test/Cancel button at the top-left of the
						     preview band. The earlier `-translate-y-1/2`
						     "float onto the editor" version was clipped in
						     Firefox (transform + overflow: visible interaction
						     on the parent splitpane Pane), so the button is
						     now positioned inside the panel — `top-1` keeps
						     it visually pinned to the top edge without
						     relying on cross-browser overflow behaviour. -->
							<div class="relative h-full pt-9 flex flex-col">
								{#if testJob?.id && testJob.type === 'CompletedJob' && $workspaceStore}
									<!-- Right-side affordances when we're displaying a *completed*
									     job (either the user just ran a test, or the on-mount
									     last-run loader populated the panel). The job-id link
									     opens the full run page in a new tab; the dispatch
									     button mounts a popover that shows what downstream
									     jobs this run triggered (rendered only when the run
									     actually dispatched anything). -->
									<div class="absolute top-1 right-2 z-10 flex items-center gap-2">
										<a
											class="text-3xs text-blue-600 hover:underline font-mono"
											href={`${base}/run/${testJob.id}?workspace=${$workspaceStore}`}
											target="_blank"
											rel="noopener noreferrer"
											title="Open this run"
										>
											{testJob.id.slice(0, 8)}… ↗
										</a>
										<DispatchEventsButton
											workspace={testJob.workspace_id ?? $workspaceStore}
											jobId={testJob.id}
										/>
									</div>
								{/if}
								<div class="absolute top-1 left-2 z-10">
									{#if testIsLoading}
										{@render cancelTestButton('sm', 'shadow-md')}
									{:else if (customUi?.previewPanel?.downstreamSubscribers ?? 0) > 0 || customUi?.previewPanel?.onBoundedRun}
										<!-- Split button: primary "Test" runs just this step
									     (skips the asset-trigger cascade); the caret
									     opens a popover with the cascade option labelled
									     by the downstream count. The active mode is
									     reflected in both the button label and the
									     check-mark on the menu item so the user always
									     knows whether the next run will fan out. A
									     pure-reader-only root has no subscriber downstream
									     but still gets `onBoundedRun`, so the split button
									     also opens for it (with the cascade item hidden). -->
										{@const downstream = customUi?.previewPanel?.downstreamSubscribers ?? 0}
										<div class="flex items-stretch shadow-md rounded-md overflow-hidden">
											<Button
												on:click={() => runTest()}
												unifiedSize="sm"
												btnClasses="rounded-r-none"
												variant="accent-secondary"
												startIcon={{
													icon: cascadeDownstream ? Zap : Play,
													classes: 'animate-none'
												}}
												shortCut={{ Icon: CornerDownLeft }}
											>
												{cascadeDownstream ? `Test + trigger ${downstream}` : 'Test'}
											</Button>
											<Popover
												placement="bottom-end"
												bind:isOpen={cascadeMenuOpen}
												usePointerDownOutside
												contentClasses="p-0"
											>
												{#snippet trigger()}
													<!-- min-h-7 matches Button's unifiedSize="sm" (28px);
												     the bg/text/hover classes mirror Button's
												     `variant="accent-secondary"` so the split reads as
												     one connected control instead of two mismatched
												     buttons. The left seam uses a translucent
												     foreground-tinted border so it stays visible
												     against both the base and hover bg without
												     fighting the variant's color. -->
													<button
														type="button"
														class="self-stretch min-h-7 px-1.5 flex items-center justify-center bg-surface-accent-secondary hover:bg-surface-accent-secondary-hover focus-visible:bg-surface-accent-secondary-clicked text-white dark:text-deep-blue-900 border-l border-white/30 dark:border-deep-blue-900/30 transition-colors"
														title="Run options"
														aria-label="Run options"
													>
														<ChevronDown size={14} />
													</button>
												{/snippet}
												{#snippet content({ close })}
													<div class="w-72 py-1 text-xs">
														<button
															type="button"
															class="w-full text-left px-3 py-2 hover:bg-surface-hover flex items-start gap-2"
															onclick={() => {
																cascadeDownstream = false
																close()
																void runTest()
															}}
														>
															<Play
																size={14}
																class="mt-0.5 shrink-0 text-emerald-700 dark:text-emerald-400"
															/>
															<div class="flex flex-col min-w-0">
																<span class="font-medium">
																	Test {!cascadeDownstream ? '(current)' : ''}
																</span>
																<span class="text-2xs text-secondary"
																	>Run just this step. Downstream subscribers are not triggered.</span
																>
															</div>
														</button>
														{#if downstream > 0}
															<button
																type="button"
																class="w-full text-left px-3 py-2 hover:bg-surface-hover flex items-start gap-2"
																onclick={() => {
																	cascadeDownstream = true
																	close()
																	void runTest()
																}}
															>
																<Zap
																	size={14}
																	class="mt-0.5 shrink-0 text-amber-600 dark:text-amber-400"
																/>
																<div class="flex flex-col min-w-0">
																	<span class="font-medium">
																		Test + trigger {downstream} downstream {cascadeDownstream
																			? '(current)'
																			: ''}
																	</span>
																	<span class="text-2xs text-secondary">
																		Let the asset-trigger cascade fan out to the {downstream}
																		subscribed script{downstream === 1 ? '' : 's'} after this run succeeds.
																	</span>
																</div>
															</button>
														{/if}
														{#if customUi?.previewPanel?.onBoundedRun}
															<button
																type="button"
																class="w-full text-left px-3 py-2 hover:bg-surface-hover flex items-start gap-2 border-t"
																onclick={() => {
																	close()
																	customUi!.previewPanel!.onBoundedRun!()
																}}
															>
																<Target
																	size={14}
																	class="mt-0.5 shrink-0 text-blue-600 dark:text-blue-400"
																/>
																<div class="flex flex-col min-w-0">
																	<span class="font-medium">Run downstream up to…</span>
																	<span class="text-2xs text-secondary">
																		Pick end node(s) on the graph, then run only the cascade between
																		this script and them.
																	</span>
																</div>
															</button>
														{/if}
													</div>
												{/snippet}
											</Popover>
										</div>
									{:else}
										{@render runTestButton('sm', 'shadow-md')}
									{/if}
								</div>
								{#if customUi?.previewPanel?.argsAboveLogs && schema?.properties && Object.keys(schema.properties).length > 0}
									<div
										class="shrink-0 overflow-auto max-h-[45%] border-b border-gray-200 dark:border-gray-700 px-4 py-3"
									>
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
								{/if}
								<div class="grow min-h-0">
									{@render testLogPanel()}
								</div>
							</div>
						{:else}
							{#key previewLayout}
								<Splitpanes
									horizontal={previewLayout !== 'bottom'}
									class="!max-h-[calc(100%-{debugMode && isDebuggableScript
										? '83'
										: previewLayout === 'bottom'
											? '0'
											: '43'}px)]"
								>
									<Pane size={previewLayout === 'bottom' ? 40 : 33}>
										{#if previewLayout === 'bottom' && !(debugMode && isDebuggableScript)}
											<div class="px-3 pt-2 pb-1 flex items-center gap-2">
												{#if testIsLoading}
													{@render cancelTestButton('sm', 'w-full')}
												{:else}
													{@render runTestButton('sm', 'w-full')}
												{/if}
											</div>
										{/if}
										{#if jsonView}
											<div
												class="py-2"
												style="height: {!schemaHeight || schemaHeight < 600 ? 600 : schemaHeight}px"
												data-schema-picker
											>
												<JsonInputs
													on:select={(e) => {
														if (e.detail) {
															if (activeModuleTab !== null) {
																testPanelArgs = e.detail
															} else {
																args = e.detail
															}
														}
													}}
													updateOnBlur={false}
													placeholder={`Write args as JSON.<br/><br/>Example:<br/><br/>{<br/>&nbsp;&nbsp;"foo": "12"<br/>}`}
												/>
											</div>
										{:else}
											<div class="px-4">
												<div
													class="break-words relative font-sans"
													bind:clientHeight={schemaHeight}
												>
													{#key argsRender}
														{#if activeModuleTab !== null}
															<SchemaForm
																helperScript={{
																	source: 'inline',
																	code: editorCode,
																	//@ts-ignore
																	lang: effectiveLang
																}}
																compact
																schema={testPanelSchema}
																bind:args={testPanelArgs}
																bind:isValid
																noVariablePicker={customUi?.previewPanel?.disableVariablePicker ===
																	true}
																showSchemaExplorer
															/>
														{:else}
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
																noVariablePicker={customUi?.previewPanel?.disableVariablePicker ===
																	true}
																showSchemaExplorer
															/>
														{/if}
													{/key}
													{#if showPsCommonParams}
														<div class="mt-2">
															<PowerShellCommonParams bind:args={psCommonParams} />
														</div>
													{/if}
												</div>
											</div>
										{/if}
									</Pane>
									<Pane size={previewLayout === 'bottom' ? 60 : 67} class="relative">
										{@render testLogPanel()}
									</Pane>
								</Splitpanes>
							{/key}
						{/if}
					{/if}
				</div>
			</Pane>
		</Splitpanes>
	</div>
</SplitPanesWrapper>

{#snippet cancelTestButton(size: 'sm' | 'md', btnClasses: string)}
	<Button on:click={() => jobLoader?.cancelJob()} unifiedSize={size} {btnClasses}>
		<WindmillIcon
			white={true}
			class="mr-2 text-white"
			height={size === 'md' ? '16px' : '14px'}
			width={size === 'md' ? '20px' : '16px'}
			spin="fast"
		/>
		Cancel
	</Button>
{/snippet}

{#snippet runTestButton(size: 'sm' | 'md', btnClasses: string)}
	<Button
		on:click={() => runTest()}
		unifiedSize={size}
		{btnClasses}
		variant="accent-secondary"
		startIcon={{ icon: Play, classes: 'animate-none' }}
		shortCut={{ Icon: CornerDownLeft }}
	>
		Test
	</Button>
{/snippet}

<!-- Single source of truth for the preview LogPanel — rendered by both the
     compact (hideArgs) layout and the splitpane layouts. One invocation
     prevents prop drift between copies (the history tab's lazy-load via
     onTabChange was lost in one copy when they diverged). -->
{#snippet testLogPanel()}
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
		onTabChange={(tab) => {
			historyTabActive = tab === 'history'
			if (historyTabActive) {
				loadPastTests()
			}
		}}
		previewIsLoading={debugMode ? $debugState.running && !$debugState.stopped : testIsLoading}
		{editor}
		{diffEditor}
		args={activeModuleTab !== null ? testPanelArgs : args}
		{showCaptures}
		customUi={customUi?.previewPanel}
		showCustomResultPanel={showDebugPanel}
	>
		{#if scriptProgress && !debugMode}
			<!-- Put to the slot in logpanel -->
			<JobProgressBar job={testJob} {scriptProgress} bind:this={jobProgressBar} compact={true} />
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
{/snippet}

{#snippet addModuleForm(close: () => void)}
	<div class="flex flex-col gap-2">
		<label for="module-name-input" class="text-xs font-semibold text-emphasis">File name</label>
		<input
			id="module-name-input"
			type="text"
			class="border rounded px-2 py-1.5 text-sm bg-surface"
			bind:this={modulePathInputEl}
			bind:value={modulePathInput}
			placeholder={'helper' + (allowedModuleExtensions[0] ?? '.ts')}
			oninput={() => {
				modulePathError = validateModulePath(modulePathInput)
			}}
			onkeydown={(e) => {
				if (e.key === 'Enter') addModule()
				if (e.key === 'Escape') close()
			}}
		/>
		{#if modulePathError}
			<p class="text-red-500 text-2xs">{modulePathError}</p>
		{/if}
		<p class="text-tertiary text-2xs"
			>Supports subfolders, e.g. <code class="text-2xs"
				>utils/math{allowedModuleExtensions[0] ?? '.ts'}</code
			></p
		>
		<div class="flex justify-end gap-2">
			<Button
				variant="default"
				size="xs"
				onclick={() => {
					modulePathInput = ''
					modulePathError = ''
					close()
				}}>Cancel</Button
			>
			<Button
				variant="accent"
				size="xs"
				onclick={addModule}
				disabled={!modulePathInput.trim() || !!modulePathError}>Add</Button
			>
		</div>
	</div>
{/snippet}

{#snippet renameModuleForm(oldPath: string, close: () => void)}
	<div class="flex flex-col gap-2">
		<label for="rename-module-input" class="text-xs font-semibold text-emphasis"
			>Rename module</label
		>
		<input
			id="rename-module-input"
			type="text"
			class="border rounded px-2 py-1.5 text-sm bg-surface"
			bind:this={renameModuleInputEl}
			bind:value={renameModuleInput}
			oninput={() => {
				renameModuleError = validateRenameModulePath(renameModuleInput, oldPath)
			}}
			onkeydown={(e) => {
				if (e.key === 'Enter') {
					renameModule(oldPath)
					close()
				}
				if (e.key === 'Escape') close()
			}}
		/>
		{#if renameModuleError}
			<p class="text-red-500 text-2xs">{renameModuleError}</p>
		{/if}
		<div class="flex justify-end gap-2">
			<Button
				variant="default"
				size="xs"
				onclick={() => {
					renameModuleInput = ''
					renameModuleError = ''
					close()
				}}>Cancel</Button
			>
			<Button
				variant="accent"
				size="xs"
				onclick={() => {
					renameModule(oldPath)
					close()
				}}
				disabled={!renameModuleInput.trim() ||
					renameModuleInput.trim() === oldPath ||
					!!renameModuleError}>Rename</Button
			>
		</div>
	</div>
{/snippet}

{#snippet editorContent()}
	<div class="h-full !overflow-visible bg-surface dark:bg-[#272D38] relative flex flex-col">
		{#if supportsModules}
			<div
				class="flex items-center border-b border-tertiary/30 bg-surface-secondary px-1 gap-0.5 text-xs overflow-x-auto shrink-0"
			>
				<button
					class="px-2 py-1 rounded-t {activeModuleTab === null
						? 'bg-surface font-semibold border-b-2 border-blue-500'
						: 'hover:bg-surface-hover'}"
					onclick={() => switchToMain()}
				>
					{mainFileName}
				</button>
				{#each Object.keys(modules ?? {}) as modulePath}
					<div
						class="group rounded-t flex items-center {activeModuleTab === modulePath
							? 'bg-surface font-semibold border-b-2 border-blue-500'
							: 'hover:bg-surface-hover'}"
					>
						<button class="pl-2 py-1 flex items-center" onclick={() => switchToModule(modulePath)}>
							{modulePath}
						</button>
						<div class="flex items-center pr-1 w-[32px] justify-end">
							<Popover
								placement="bottom-start"
								openFocus={renameModuleInputEl}
								contentClasses="p-3 w-72"
							>
								{#snippet trigger()}
									<span
										class="opacity-0 group-hover:opacity-100 hover:text-blue-500 transition-opacity"
										role="button"
										tabindex="0"
										onclick={(e) => {
											e.stopPropagation()
											renameModuleInput = modulePath
											renameModuleError = ''
										}}
										onkeydown={(e) => {
											if (e.key === 'Enter') {
												e.stopPropagation()
												renameModuleInput = modulePath
												renameModuleError = ''
											}
										}}
									>
										<Pencil size={12} />
									</span>
								{/snippet}
								{#snippet content({ close })}
									{@render renameModuleForm(modulePath, close)}
								{/snippet}
							</Popover>
							<span
								class="opacity-0 group-hover:opacity-100 hover:text-red-500 transition-opacity"
								role="button"
								tabindex="0"
								onclick={(e) => {
									e.stopPropagation()
									removeModule(modulePath)
								}}
								onkeydown={(e) => {
									if (e.key === 'Enter') {
										e.stopPropagation()
										removeModule(modulePath)
									}
								}}
							>
								<X size={12} />
							</span>
						</div>
					</div>
				{/each}
				<Popover
					placement="bottom-start"
					bind:isOpen={showAddModulePopover}
					openFocus={modulePathInputEl}
					contentClasses="p-3 w-72"
				>
					{#snippet trigger()}
						<span class="px-2 py-1 rounded-t hover:bg-surface-hover inline-flex items-center">
							<Plus size={12} />
						</span>
					{/snippet}
					{#snippet content({ close })}
						{@render addModuleForm(close)}
					{/snippet}
				</Popover>
			</div>
		{/if}
		<div class="relative flex-1 !overflow-visible">
			<div class="absolute bg-surface top-2 right-4 z-10 flex flex-row gap-2">
				{#if assets?.length}
					<AssetsDropdownButton {assets} />
				{/if}

				{#if isDebuggableScript && customUi?.editorBar?.debug != false}
					<Button
						variant={debugMode ? 'accent' : 'default'}
						unifiedSize="sm"
						onclick={toggleDebugMode}
						startIcon={{ icon: Bug }}
						iconOnly
						btnClasses={debugMode
							? ''
							: 'bg-surface hover:bg-surface-hover border border-tertiary/30'}
						title={debugMode ? 'Exit Debug Mode' : 'Toggle Debug Mode'}
					/>
				{/if}
				{#if showDebugPanel && !showDebugConsole}
					<Button
						variant="default"
						unifiedSize="sm"
						onclick={() => (showDebugConsole = true)}
						startIcon={{ icon: Terminal }}
						title="Show Debug Console"
					>
						Console
					</Button>
				{/if}
				{#if lang === 'ansible' && hasDelegateToGitRepo}
					<Button
						variant="default"
						unifiedSize="sm"
						onclick={() => (gitRepoResourcePickerOpen = true)}
						startIcon={{ icon: GitBranch }}
					>
						Delegating to git repo
					</Button>
				{/if}
				{#if testPanelSize === 0}
					<HideButton
						hidden={true}
						direction="right"
						unifiedSize="sm"
						variant="accent-secondary"
						panelName="test"
						shortcut="U"
						customHiddenIcon={{
							icon: PlayIcon
						}}
						on:click={() => {
							toggleTestPanel()
						}}
					/>
				{/if}
				{#if !aiChatManager.open && !disableAi && !inSessionPane}
					{#if customUi?.editorBar?.aiGen != false && SUPPORTED_CHAT_SCRIPT_LANGUAGES.includes(lang ?? '')}
						<HideButton
							hidden={true}
							direction="right"
							panelName="AI"
							shortcut="L"
							unifiedSize="sm"
							usePopoverOverride={!$copilotInfo.enabled}
							customHiddenIcon={{
								icon: WandSparkles
							}}
							btnClasses="!text-ai"
							variant="default"
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
	</div>
{/snippet}

{#snippet editorPane()}
	{#key effectiveLang}
		<Editor
			lineNumbersMinChars={4}
			folding
			{path}
			bind:code={editorCode}
			bind:websocketAlive
			bind:this={editor}
			schemaContractMarkers={contractMarkers}
			{yContent}
			awareness={wsProvider?.awareness}
			on:change={(e) => {
				if (activeModuleTab === null) {
					code = editorCode
					lastSyncedCode = code
					inferSchema(e.detail)
				} else {
					flushModuleContent()
					inferModuleSchema()
				}
				// Refresh breakpoint positions when code changes (decorations track their lines)
				if (debugMode && breakpointDecorations.length > 0) {
					refreshBreakpointPositions()
				}
			}}
			on:saveDraft
			on:toggleTestPanel={toggleTestPanel}
			cmdEnterAction={async () => {
				if (activeModuleTab === null) {
					await inferSchema(editorCode)
				} else {
					await inferModuleSchema()
				}
				// The Editor already ran the DDL guard before invoking this action.
				runTest({ skipDdlGuard: true })
			}}
			formatAction={async () => {
				if (activeModuleTab === null) {
					await inferSchema(editorCode)
				}
				dispatch('format')
			}}
			class="flex flex-1 h-full !overflow-visible"
			scriptLang={effectiveLang}
			automaticLayout={true}
			{fixedOverflowWidgets}
			{args}
			workflowAsCode={workflowAsCodeAiContext}
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

	/* Ghost breakpoint shown on gutter hover before the user clicks */
	.debug-breakpoint-glyph-hover {
		background-color: #e51400;
		opacity: 0.35;
		border-radius: 50%;
		width: 10px !important;
		height: 10px !important;
		margin-left: 5px;
		margin-top: 4px;
		cursor: pointer;
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
