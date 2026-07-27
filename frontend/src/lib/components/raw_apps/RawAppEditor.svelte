<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { paneMinPercent } from '$lib/utils/splitpaneSizing'
	import RawAppInlineScriptsPanel from './RawAppInlineScriptsPanel.svelte'
	import type { JobById } from '../apps/types'
	import RawAppEditorHeader from './RawAppEditorHeader.svelte'
	import RawAppYamlEditor, { type RawAppYamlUpdate } from './RawAppYamlEditor.svelte'
	import type Drawer from '../common/drawer/Drawer.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import { type Policy, WorkspaceService } from '$lib/gen'
	import DiffDrawer from '../DiffDrawer.svelte'
	import { deepEqual } from 'fast-equals'

	// import { addWmillClient } from './utils'
	import RawAppBackgroundRunner from './RawAppBackgroundRunner.svelte'
	import { workspaceStore } from '$lib/stores'
	import { setRawAppOperatingWorkspace } from './rawAppWorkspace'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import {
		genWmillTs,
		normalizeRawAppRuntimeLogs,
		type Runnable,
		type RawAppRuntimeLogEntry,
		type RawAppRuntimeLogRequester,
		type RawAppRunSummary,
		type RawAppRunsProvider,
		type RawAppScreenshotRequester
	} from './utils'
	import { runDomQueryOnHtml, type RawAppDomQuery, type RawAppDomRequester } from './rawAppDom'
	import InlineElementPrompt from './InlineElementPrompt.svelte'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import RawAppSidebar from './RawAppSidebar.svelte'
	import type { Modules } from './RawAppModules.svelte'
	import { isRunnableByName, isRunnableByPath } from '../apps/inputType'
	import { aiChatManager, AIMode } from '../copilot/chat/AIChatManager.svelte'
	import { onMount, onDestroy, untrack } from 'svelte'
	import type {
		AppDatatableMetadata,
		LintResult,
		InspectorElementInfo
	} from '../copilot/chat/app/core'
	import { createAppSelectedContext, type AppCodeSelectionElement } from '../copilot/chat/context'
	import { captureScale, MAX_IMAGE_EDGE } from '../copilot/chat/imageUtils'
	import { rawAppLintStore } from './lintStore'
	import { dbSchemas } from '$lib/stores'
	import {
		MousePointerSquareDashed,
		RefreshCw,
		Columns2,
		ChevronDown,
		Eye,
		SquareArrowOutUpRight
	} from 'lucide-svelte'
	import DraggableTabs, { type TabItem } from '$lib/components/common/tabs/DraggableTabs.svelte'
	import { runScriptAndPollResult } from '../jobs/utils'
	import { RawAppHistoryManager } from './RawAppHistoryManager.svelte'
	import { sendUserToast } from '$lib/utils'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import {
		buildDataTableWhitelist,
		parseDataTableRef,
		formatDataTableRef,
		isDatatableTableAllowed,
		type RawAppData,
		DEFAULT_DATA
	} from './dataTableRefUtils'
	import { randomUUID } from '$lib/utils/uuid'

	interface Props {
		files?: Record<string, string>
		runnables?: Record<string, Runnable>
		/** Data configuration including tables and creation policy */
		data?: RawAppData
		newApp: boolean
		policy: Policy
		summary?: string
		path: string
		newPath?: string | undefined
		/** Initial labels for the app, threaded from the loaded app data. */
		labels?: string[]
		savedApp?:
			| {
					value: any
					draft?: any
					path: string
					summary: string
					policy: any
					draft_only?: boolean
					/** No deployed counterpart exists (draft-only); disables Diff. */
					no_deployed?: boolean
					custom_path?: string
					labels?: string[]
			  }
			| undefined
		diffDrawer?: DiffDrawer | undefined
		onNavigate?: (item: import('$lib/components/workspacePicker').WorkspaceItem) => void
		/** Fired after a successful deploy; the session preview reloads on it. */
		onDeploy?: (e: { path: string }) => void
		/** Initial collapsed state for the file/runnable sidebar. The user's
		 * toggled preference is persisted under `sidebarStorageKey`; this prop
		 * only seeds the very first open. */
		defaultSidebarCollapsed?: boolean
		/** localStorage key for the persisted sidebar state. Pass a different
		 * key when the editor is rendered in a context that wants its own
		 * preference. */
		sidebarStorageKey?: string
		liveEditorDraftStoragePath?: string
		/** Indicator-only overrides forwarded to RawAppEditorHeader so the
		 *  sessions preview's AutosaveIndicator watches the session's
		 *  (workspace, path). Undefined on the full-page editor. */
		autosaveWorkspace?: string
		autosavePath?: string
		/** Initial value for the "Split with Preview" tab-bar toggle. Defaults
		 * to `true` (split mode, preview always pinned to the right). Set
		 * `false` when the editor mounts inside a context that wants single-
		 * view by default with the Preview tab selected — e.g. session
		 * previews, where the editor pane is already narrow. The user can
		 * still toggle the mode after mount; this prop only seeds the
		 * initial state. */
		defaultSplitWithPreview?: boolean
		/** User-typed path when it differs from `savedApp.path`. The route injects
		 *  it as `draft_path` so the home row shows the friendly name, not `draft_{uuid}`. */
		pendingDraftPath?: string | undefined
		// Threaded to the AutosaveIndicator's "Reset to deployed" button.
		onResetToDeployed?: () => void | Promise<void>
		// See ScriptBuilderProps — same indicator semantics.
		loadedFromDraft?: boolean
		othersDraftsCount?: number
		onOpenOthersDrafts?: () => void
		onRuntimeLogRequester?: (requester: RawAppRuntimeLogRequester | undefined) => void
		onRunsProvider?: (provider: RawAppRunsProvider | undefined) => void
		// Session preview only: expose a live DOM query requester (search/read the
		// rendered preview by CSS selector) and forward inspector element picks so
		// they can be attached to the session chat as selector context chips.
		onDomRequester?: (requester: RawAppDomRequester | undefined) => void
		// `additive` (Shift held) adds to the selection; otherwise it replaces it.
		onInspectorSelect?: (info: InspectorElementInfo, additive: boolean) => void
		// Session preview only: the chat's current DOM-selector chips (source of
		// truth). Pushed into the preview so it renders one highlight per selector.
		selectedDomSelectors?: string[]
		onInspectorDeselect?: (selector: string) => void
		onInspectorClearAll?: () => void
		// Session preview only: send a prompt scoped to a selected element (via the
		// inline mini-composer anchored over it in the preview).
		onInlinePrompt?: (selector: string, prompt: string) => void
		onScreenshotRequester?: (requester: RawAppScreenshotRequester | undefined) => void
		// Restoring an older deployment from the history drawer. A callback prop
		// (not `on:restore` forwarding): forwarding a `createEventDispatcher`
		// event up through these runes-mode components silently drops it.
		onRestore?: (restoredApp: any) => void
		// Deploy created the app at a new path; the page navigates to it. Callback
		// prop for the same reason as `onRestore` — `on:savedNewAppPath` forwarding
		// through these runes-mode components is dropped.
		onSavedNewAppPath?: (path: string) => void
		// Condensed top bar: smaller (sm) buttons, a shorter bar, and the
		// EditorHeader's path/breadcrumb row dropped (summary only). Used by the
		// session preview to save vertical room.
		condensedHeader?: boolean
	}

	let {
		files = $bindable({}),
		runnables = $bindable({}),
		data = $bindable(DEFAULT_DATA),
		newApp,
		policy,
		summary = $bindable(''),
		path,
		newPath = undefined,
		labels = undefined,
		savedApp = $bindable(undefined),
		diffDrawer = undefined,
		onNavigate,
		onDeploy = undefined,
		defaultSidebarCollapsed = false,
		sidebarStorageKey = 'raw-app-sidebar-collapsed',
		liveEditorDraftStoragePath = undefined,
		autosaveWorkspace = undefined,
		autosavePath = undefined,
		defaultSplitWithPreview = true,
		pendingDraftPath = $bindable(undefined),
		onResetToDeployed,
		loadedFromDraft = false,
		othersDraftsCount = 0,
		onOpenOthersDrafts,
		onRuntimeLogRequester = undefined,
		onRunsProvider = undefined,
		onDomRequester = undefined,
		onInspectorSelect = undefined,
		selectedDomSelectors = [],
		onInspectorDeselect = undefined,
		onInspectorClearAll = undefined,
		onInlinePrompt = undefined,
		onScreenshotRequester = undefined,
		onRestore,
		onSavedNewAppPath,
		condensedHeader = false
	}: Props = $props()
	export const version: number | undefined = undefined

	// Workspace this editor operates on: the session's acting workspace when
	// embedded in a session preview (autosaveWorkspace), else the navigation
	// workspace. Deploy/save/background-runner must target it, not $workspaceStore.
	const opWorkspace = $derived(autosaveWorkspace ?? $workspaceStore)
	// Expose it to the sidebar sub-components (inline scripts, datatable/shared-UI
	// drawers, DB selector) so their lookups target the app's workspace too.
	setRawAppOperatingWorkspace(() => opWorkspace)

	// Convert to object format for child components
	let dataTableRefsObjects = $derived(data.tables.map(parseDataTableRef))
	let dataTableWhitelist = $derived(buildDataTableWhitelist(dataTableRefsObjects))

	type DataTableTablesMetadata = {
		datatable_name: string
		schemas: Record<string, string[]>
		error?: string
	}

	function countTables(schemas: Record<string, string[]>): number {
		return Object.values(schemas).reduce((acc, tables) => acc + tables.length, 0)
	}

	function withTableCount(datatable: DataTableTablesMetadata): AppDatatableMetadata {
		return {
			datatable_name: datatable.datatable_name,
			schemas: datatable.schemas,
			tableCount: countTables(datatable.schemas),
			...(datatable.error && { error: datatable.error })
		}
	}

	function isDatatableTableWhitelisted(
		datatableName: string,
		schemaName: string,
		tableName: string
	): boolean {
		return isDatatableTableAllowed(dataTableWhitelist, datatableName, schemaName, tableName)
	}

	function filterDatatableTables(allTables: DataTableTablesMetadata[]): AppDatatableMetadata[] {
		if (dataTableWhitelist.datatables.size === 0) {
			return allTables.map(withTableCount)
		}

		const results: AppDatatableMetadata[] = []
		for (const datatable of allTables) {
			if (!dataTableWhitelist.datatables.has(datatable.datatable_name)) {
				continue
			}

			if (dataTableWhitelist.allTablesDatatables.has(datatable.datatable_name)) {
				results.push(withTableCount(datatable))
				continue
			}

			const allowedTables = dataTableWhitelist.tables.get(datatable.datatable_name)
			if (!allowedTables) {
				results.push(
					withTableCount({
						datatable_name: datatable.datatable_name,
						schemas: {},
						error: datatable.error
					})
				)
				continue
			}

			const filteredSchemas: Record<string, string[]> = {}
			for (const [schemaName, tableNames] of Object.entries(datatable.schemas)) {
				const allowedTablesInSchema = allowedTables.get(schemaName)
				if (!allowedTablesInSchema) continue

				const filteredTables = tableNames.filter((tableName) =>
					allowedTablesInSchema.has(tableName)
				)
				if (filteredTables.length > 0) {
					filteredSchemas[schemaName] = filteredTables
				}
			}

			results.push(
				withTableCount({
					datatable_name: datatable.datatable_name,
					schemas: filteredSchemas,
					error: datatable.error
				})
			)
		}

		return results
	}

	// Initialize history manager
	const historyManager = new RawAppHistoryManager({
		maxEntries: 50,
		autoSnapshotInterval: 5 * 60 * 1000 // 5 minutes
	})
	historyManager.manualSnapshot(files ?? {}, runnables, summary, data)

	let iframe: HTMLIFrameElement | undefined = $state(undefined)
	let previewIframe: HTMLIFrameElement | undefined = $state(undefined)
	let previewIframeLoaded = $state(false)
	let lastBuild: { css: string; js: string } | undefined = undefined
	// Detached preview tab/window rendering the same app-preview bundle as the
	// inline pane. Kept live-synced: every build is replayed into it until the
	// user closes it. Not reactive — it's a window handle, not UI state.
	let externalPreviewWindow: Window | null = null
	let inspectorEnabled = $state(false)
	let bundlerType: 'esbuild' | 'rolldown' = $state('esbuild')

	// Build/bundler logs forwarded from the UI Builder iframe. We render
	// them as an overlay inside the preview pane (right side) so they're
	// visually tied to the build output, not to the source editor.
	let logs = $state('')

	// Latest UI Builder error; cleared on next successful build.
	let buildError = $state<string | undefined>(undefined)
	// Latest uncaught runtime error thrown by the rendered app; cleared on next build.
	let runtimeError = $state<string | undefined>(undefined)
	// Set when a build ran cleanly but never mounted anything into #root — the
	// entrypoint defines a component without ever mounting it. Cleared on next build.
	let emptyRender = $state(false)
	// The repair hint above has to match the app's framework: only React apps have
	// an `index.tsx` and `createRoot`; Svelte and Vue mount from `index.ts`. Keyed
	// off file extensions rather than exact template filenames, which users rename.
	let mountHint = $derived.by(() => {
		const paths = Object.keys(files ?? {}).map((p) => p.replace(/^\//, ''))
		const entrypoint = paths.find((p) => /^index\.(tsx|jsx|ts|js)$/.test(p))
		if (paths.some((p) => p.endsWith('.svelte'))) {
			return {
				entrypoint: entrypoint ?? 'index.ts',
				call: "mount(App, { target: document.getElementById('root')! })"
			}
		}
		if (paths.some((p) => p.endsWith('.vue'))) {
			return { entrypoint: entrypoint ?? 'index.ts', call: "createApp(App).mount('#root')" }
		}
		return {
			entrypoint: entrypoint ?? 'index.tsx',
			call: "createRoot(document.getElementById('root')!).render(<App />)"
		}
	})
	let logsCollapsed = $state(false)
	let logsDiv: HTMLDivElement | undefined = $state(undefined)
	$effect(() => {
		if (logsDiv && logs && !logsCollapsed) {
			const t = setTimeout(() => logsDiv?.scrollTo(0, logsDiv.scrollHeight), 50)
			return () => clearTimeout(t)
		}
	})

	// Tab system — the source side of the editor area. The sidebar stays the
	// primary navigation; tabs are a secondary surface that's useful when the
	// sidebar is collapsed and on narrow viewports.
	const PREVIEW_TAB_ID = 'preview'
	const FILE_PREFIX = 'file:'
	const RUNNABLE_PREFIX = 'runnable:'
	const previewTab: TabItem = {
		id: PREVIEW_TAB_ID,
		label: 'Preview',
		icon: Eye,
		iconClass: 'text-accent',
		labelClass: 'text-accent',
		closable: false,
		pinned: 'right'
	}
	let tabs: TabItem[] = $state([previewTab])
	let activeTabId: string = $state(PREVIEW_TAB_ID)
	// Seed from the prop, then own the state locally so the user's toggle
	// after mount sticks even if the prop reference changes.
	let splitWithPreview: boolean = $state(untrack(() => defaultSplitWithPreview))
	const activeTabKind = $derived<'file' | 'runnable' | 'preview'>(
		activeTabId === PREVIEW_TAB_ID
			? 'preview'
			: activeTabId.startsWith(FILE_PREFIX)
				? 'file'
				: 'runnable'
	)
	// Tint the Preview tab red so the error is visible when Preview isn't active.
	function tintPreviewOnError(t: TabItem): TabItem {
		if (t.id !== PREVIEW_TAB_ID || !buildError) return t
		const errClass = 'text-red-600 dark:text-red-400'
		return { ...t, iconClass: errClass, labelClass: errClass }
	}
	const tintTabs = (ts: TabItem[]) => ts.map(tintPreviewOnError)
	// Single mode: both bars mirror the full list (the visible pane carries
	// every tab). Split mode: left = files/runnables, right = Preview only.
	const leftPaneTabs = $derived<TabItem[]>(
		tintTabs(splitWithPreview ? tabs.filter((t) => t.id !== PREVIEW_TAB_ID) : tabs)
	)
	const rightPaneTabs = $derived<TabItem[]>(
		tintTabs(splitWithPreview ? tabs.filter((t) => t.id === PREVIEW_TAB_ID) : tabs)
	)
	// In split mode the right bar always highlights Preview, regardless of the
	// left pane's active file/runnable.
	const rightPaneActiveId = $derived(splitWithPreview ? PREVIEW_TAB_ID : activeTabId)
	const showSource = $derived(activeTabKind === 'file')
	const showRunnable = $derived(activeTabKind === 'runnable')
	// Mount the UI Builder iframe the first time a file is shown (paneA has
	// width then; mounting it at 0-width breaks the VS Code workbench), and
	// keep it mounted so tab switches don't reload it. Mount it as soon as
	// either pane needs it: `showSource` for the source-editor view, OR the
	// preview tab is active — the Preview iframe is fed by `preview`
	// postMessages bundled by the UI Builder iframe, so it needs to be
	// mounted even when the user opens the editor straight on Preview (e.g.
	// session previews seeded with `defaultSplitWithPreview=false`).
	let iframeShouldMount = $state(false)
	$effect(() => {
		if (showSource || activeTabKind === 'preview') iframeShouldMount = true
	})
	// Width of the editor area (both inner panes). The UI Builder iframe is
	// pre-mounted while it's the inactive tab so the editor is ready instantly;
	// but the VS Code workbench inside crashes if it boots at 0 size. So while
	// inactive we keep the iframe at this real width and hide it with
	// `visibility` instead of collapsing it — Monaco boots correctly and
	// revealing a file is just an unhide (no reload, no relayout, no latency).
	let editorAreaWidth = $state(0)

	// Inner pane sizes are a pure function of mode + active tab → derived.
	// `paneARatio` is the user's last manual split drag (set by rememberPaneDrag).
	let paneARatio = $state(50)
	const paneALeftSize = $derived(
		splitWithPreview && activeTabKind !== 'preview'
			? paneARatio
			: activeTabKind === 'preview'
				? 0
				: 100
	)
	const paneBRightSize = $derived(100 - paneALeftSize)
	// Keep a manual drag only when it's a genuine in-between split (0/100 are
	// the programmatic collapsed/full states, not user intent).
	function rememberPaneDrag(v: number) {
		if (splitWithPreview && activeTabKind !== 'preview' && v > 0 && v < 100) {
			paneARatio = v
		}
	}

	function fileTabId(filePath: string) {
		return FILE_PREFIX + filePath
	}
	function runnableTabId(key: string) {
		return RUNNABLE_PREFIX + key
	}
	function fileBaseName(filePath: string) {
		return filePath.split('/').pop() || filePath
	}

	// Default file to open on load: prefer App.*, then index.*, then any source file.
	function pickDefaultFile(f: Record<string, string> | undefined): string | undefined {
		if (!f) return undefined
		const keys = Object.keys(f).filter((p) => !p.endsWith('/'))
		if (keys.length === 0) return undefined
		const isSource = (p: string) =>
			/\.(tsx|jsx|ts|js|svelte|vue)$/i.test(p) && !/package(-lock)?\.json$/i.test(p)
		return (
			keys.find((p) => /(^|\/)App\.(tsx|jsx|ts|js|svelte|vue)$/i.test(p)) ??
			keys.find((p) => /(^|\/)index\.(tsx|jsx|ts|js)$/i.test(p)) ??
			keys.find(isSource) ??
			keys[0]
		)
	}

	function activateTab(id: string, opts?: { force?: boolean }) {
		const tab = tabs.find((t) => t.id === id)
		if (!tab) return
		// In split mode Preview is always shown on the right, so clicking it is a
		// no-op (would collapse the left pane). `force` lets closeTab fall back to it.
		if (!opts?.force && splitWithPreview && id === PREVIEW_TAB_ID) return
		activeTabId = id
		if (tab.id === PREVIEW_TAB_ID) {
			selectedRunnable = undefined
		} else if (tab.id.startsWith(FILE_PREFIX)) {
			const filePath = tab.id.slice(FILE_PREFIX.length)
			selectedRunnable = undefined
			// `populateFiles` reads this on iframe load, so set it even if the
			// iframe isn't ready yet (the postMessage below is then skipped).
			selectedDocument = filePath
			if (iframeLoaded) {
				iframe?.contentWindow?.postMessage({ type: 'selectFile', path: filePath }, '*')
			}
		} else if (tab.id.startsWith(RUNNABLE_PREFIX)) {
			const key = tab.id.slice(RUNNABLE_PREFIX.length)
			if (selectedRunnable !== key) selectedRunnable = key
		}
	}

	function ensureFileTab(filePath: string): string {
		const id = fileTabId(filePath)
		if (!tabs.some((t) => t.id === id)) {
			// Insert before the pinned preview tab.
			const insertAt = tabs.findIndex((t) => t.pinned === 'right')
			const newTab: TabItem = {
				id,
				label: fileBaseName(filePath),
				closable: true
			}
			tabs = [
				...tabs.slice(0, insertAt === -1 ? tabs.length : insertAt),
				newTab,
				...tabs.slice(insertAt === -1 ? tabs.length : insertAt)
			]
		}
		return id
	}

	function ensureRunnableTab(key: string): string {
		const id = runnableTabId(key)
		if (!tabs.some((t) => t.id === id)) {
			const insertAt = tabs.findIndex((t) => t.pinned === 'right')
			const newTab: TabItem = {
				id,
				label: key,
				closable: true
			}
			tabs = [
				...tabs.slice(0, insertAt === -1 ? tabs.length : insertAt),
				newTab,
				...tabs.slice(insertAt === -1 ? tabs.length : insertAt)
			]
		}
		return id
	}

	function closeTab(id: string) {
		const idx = tabs.findIndex((t) => t.id === id)
		const tab = tabs[idx]
		if (!tab || tab.closable === false) return
		const wasActive = activeTabId === id
		// Clear selection before removal so the cleanup $effect doesn't recreate it.
		if (wasActive && tab.id.startsWith(RUNNABLE_PREFIX)) {
			selectedRunnable = undefined
		}
		tabs = tabs.filter((t) => t.id !== id)
		if (wasActive) {
			// Fall back to the previous tab (force, in case it's Preview in split).
			const fallback = tabs[Math.max(0, idx - 1)] ?? previewTab
			activateTab(fallback.id, { force: true })
		}
	}

	function reorderTabs(next: TabItem[]) {
		// Rebuild from the reordered `next` plus any tabs that bar didn't show
		// (e.g. files when the Preview-only right bar fires in split mode), with
		// Preview kept pinned at the end — so a reorder can never drop a tab.
		const seen = new Set(next.map((t) => t.id))
		const rest = tabs.filter((t) => !seen.has(t.id))
		tabs = [...next, ...rest].filter((t) => t.id !== PREVIEW_TAB_ID).concat(previewTab)
	}

	function toggleSplit() {
		if (splitWithPreview) {
			splitWithPreview = false
			return
		}
		// single → split: if Preview is active, move focus to the last
		// file/runnable tab (it's leaving the left bar).
		if (activeTabId === PREVIEW_TAB_ID) {
			const lastUserTab = tabs
				.slice()
				.reverse()
				.find((t) => t.id !== PREVIEW_TAB_ID)
			if (lastUserTab) activeTabId = lastUserTab.id
		}
		splitWithPreview = true
	}
	let yamlEditorDrawer: Drawer | undefined = $state(undefined)

	// Sidebar: honor the user's `%` but never shrink below SIDEBAR_PX_MIN.
	// (svelte-splitpanes' minSize only blocks drag, so we clamp ourselves.)
	let rawSidebarSize = $state(15)
	let splitContainerWidth = $state(0)
	const SIDEBAR_PX_MIN = 160
	const sidebarMinPercent = $derived(paneMinPercent(splitContainerWidth, SIDEBAR_PX_MIN))
	const sidebarPanelSize = $derived(Math.max(rawSidebarSize, sidebarMinPercent))

	// Persisted across opens. Seeded with `defaultSidebarCollapsed` only when
	// localStorage has no entry yet — callers (like the session preview pane)
	// can default the initial state without overriding a user who has already
	// expressed a preference.
	const sidebarCollapsed = useLocalStorageValue(
		sidebarStorageKey,
		defaultSidebarCollapsed,
		'boolean'
	)

	// Auto-compact when the editor opens in a narrow container (e.g. the session
	// preview pane): drop to the merged single-pane view and retract the file
	// sidebar. Applied once, on the first measured layout — later resizes are the
	// user's call. The sidebar is set without persisting so a transient narrow
	// open never overrides the user's saved expand/collapse preference.
	let rootWidth = $state(0)
	const NARROW_PX = 900
	let appliedNarrowDefault = false
	$effect(() => {
		const w = rootWidth
		if (appliedNarrowDefault || w <= 0) return
		appliedNarrowDefault = true
		if (w < NARROW_PX) {
			splitWithPreview = false
			sidebarCollapsed.setWithoutPersist(true)
		}
	})

	function handleYamlApply(update: RawAppYamlUpdate) {
		if (update.summary !== undefined) {
			summary = update.summary
		}
		if (update.files !== undefined) {
			files = update.files
		}
		if (update.runnables !== undefined) {
			runnables = update.runnables
		}
		if (update.data !== undefined) {
			data = update.data
		}
		historyManager.manualSnapshot(files ?? {}, runnables, summary, data, true)
	}

	let jobs: string[] = $state([])
	let jobsById: Record<string, JobById> = $state({})

	// Helper function to convert internal Runnable to BackendRunnable format
	function convertToBackendRunnable(key: string, runnable: Runnable): any | undefined {
		if (!runnable) return undefined

		const backendRunnable: any = {
			name: runnable.name ?? key
		}

		if (isRunnableByName(runnable)) {
			backendRunnable.type = 'inline'
			if (runnable.inlineScript) {
				// Map frontend language to backend API language
				let language = runnable.inlineScript.language
				// Backend API only supports 'bun' and 'python3' for inline scripts
				// Map TypeScript variants to 'bun'
				if (language === 'nativets' || language === 'deno') {
					language = 'bun'
				}
				backendRunnable.inlineScript = {
					language: language as 'bun' | 'python3',
					content: runnable.inlineScript.content ?? ''
				}
			}
		} else if (isRunnableByPath(runnable)) {
			// Determine type based on runType
			if (runnable.runType === 'flow') {
				backendRunnable.type = 'flow'
			} else if (runnable.runType === 'hubscript') {
				backendRunnable.type = 'hubscript'
			} else {
				backendRunnable.type = 'script'
			}
			backendRunnable.path = runnable.path
		}

		// Extract static inputs from fields
		if (runnable.fields) {
			const staticInputs: Record<string, any> = {}
			Object.entries(runnable.fields).forEach(([fieldKey, field]) => {
				if (field.type === 'static') {
					staticInputs[fieldKey] = field.value
				}
			})
			if (Object.keys(staticInputs).length > 0) {
				backendRunnable.staticInputs = staticInputs
			}
		}

		return backendRunnable
	}

	let iframeLoaded = $state(false) // @hmr:keep
	// Briefly drops the `setActiveDocument` echo VS Code fires while we're
	// pushing the initial file set — the iframe auto-opens a default editor
	// during boot which we don't want to treat as a user-driven activation.
	let suppressSetActiveDocument = false
	let suppressTimer: ReturnType<typeof setTimeout> | undefined

	let sharedUiFiles: Record<string, string> = $state({})
	let sharedUiVersion = $state(0)
	let sharedUiLoaded = $state(false)

	async function loadSharedUi() {
		if (!opWorkspace) return
		try {
			const res = (await WorkspaceService.getSharedUi({
				workspace: opWorkspace
			})) as { files?: Record<string, string>; version?: number }
			sharedUiFiles = res.files ?? {}
			sharedUiVersion = res.version ?? 0
		} catch (e) {
			console.warn('Failed to load shared UI for raw app editor:', e)
			sharedUiFiles = {}
		} finally {
			sharedUiLoaded = true
		}
	}

	function setSharedUiInIframe() {
		const filesSnap = $state.snapshot(sharedUiFiles)
		iframe?.contentWindow?.postMessage(
			{
				type: 'setSharedUi',
				files: filesSnap,
				version: sharedUiVersion
			},
			'*'
		)
	}

	function populateFiles() {
		if (files) {
			suppressSetActiveDocument = true
			if (suppressTimer !== undefined) clearTimeout(suppressTimer)
			suppressTimer = setTimeout(() => {
				suppressSetActiveDocument = false
				suppressTimer = undefined
			}, 500)
			const doc = untrack(() => selectedDocument)
			if (doc) {
				setFilesAndSelectInIframe(files, doc)
			} else {
				setFilesInIframe(files)
			}
		}
	}
	function setFilesInIframe(newFiles: Record<string, string>) {
		const files = Object.fromEntries(
			Object.entries(newFiles).filter(([path, _]) => !path.endsWith('/'))
		)
		iframe?.contentWindow?.postMessage(
			{
				type: 'setFiles',
				files: files
			},
			'*'
		)
	}

	function setFilesAndSelectInIframe(newFiles: Record<string, string>, pathToSelect: string) {
		const files = Object.fromEntries(
			Object.entries(newFiles).filter(([path, _]) => !path.endsWith('/'))
		)
		iframe?.contentWindow?.postMessage(
			{
				type: 'setFilesAndSelect',
				files: files,
				pathToSelect: pathToSelect
			},
			'*'
		)
	}

	function populateRunnables() {
		iframe?.contentWindow?.postMessage(
			{
				type: 'setRunnables',
				dts: genWmillTs(runnables)
			},
			'*'
		)
	}

	onMount(() => {
		aiChatManager.saveAndClear()
		aiChatManager.changeMode(AIMode.APP)
		rawAppLintStore.enable()
		loadSharedUi()

		// Initialize aiChatManager.datatableCreationPolicy from stored data
		aiChatManager.datatableCreationPolicy = {
			enabled: data.datatable !== undefined,
			datatable: data.datatable,
			schema: data.schema
		}

		// Start auto-snapshot
		historyManager.startAutoSnapshot(() => ({
			files: files ?? {},
			runnables,
			summary,
			data
		}))

		return () => {
			rawAppLintStore.disable()
			historyManager.destroy()
		}
	})

	// Sync data with aiChatManager.datatableCreationPolicy (bidirectional)
	$effect(() => {
		// Read the current policy from aiChatManager
		const policy = aiChatManager.datatableCreationPolicy
		// Only update if different to avoid infinite loops
		if (data.datatable !== policy.datatable || data.schema !== policy.schema) {
			data.datatable = policy.datatable
			data.schema = policy.schema
		}
	})

	$effect(() => {
		function lint(): LintResult {
			const snapshot = rawAppLintStore.getSnapshot()

			// Convert MonacoLintError[] to string[] for each runnable
			const backendErrors: Record<string, string[]> = {}
			const backendWarnings: Record<string, string[]> = {}

			for (const [key, errors] of Object.entries(snapshot.errors)) {
				backendErrors[key] = errors.map((e) => `Line ${e.startLineNumber}: ${e.message}`)
			}

			for (const [key, warnings] of Object.entries(snapshot.warnings)) {
				backendWarnings[key] = warnings.map((w) => `Line ${w.startLineNumber}: ${w.message}`)
			}

			// Count total errors and warnings
			const errorCount = Object.values(snapshot.errors).reduce((acc, arr) => acc + arr.length, 0)
			const warningCount = Object.values(snapshot.warnings).reduce(
				(acc, arr) => acc + arr.length,
				0
			)

			return {
				errors: {
					frontend: {},
					backend: backendErrors
				},
				warnings: {
					frontend: {},
					backend: backendWarnings
				},
				errorCount,
				warningCount
			}
		}
		return aiChatManager.setAppHelpers({
			lint,
			listFrontendFiles: () => {
				return [...Object.keys(files ?? {}), '/wmill.d.ts']
			},
			getFrontendFile: (path) => {
				if (path === '/wmill.d.ts') {
					return genWmillTs(runnables)
				}
				return $state.snapshot((files ?? {})[path])
			},
			getFrontendFiles: () => {
				const frontendFiles = {
					...$state.snapshot(files ?? {}),
					'/wmill.d.ts': genWmillTs(runnables)
				}
				return frontendFiles
			},
			setFrontendFile: (path, content): LintResult => {
				console.log('setting frontend file', path, content)
				if (!files) {
					files = {}
				}
				files[path] = content
				selectedDocument = path
				// Use combined setFilesAndSelect to avoid race condition
				setFilesAndSelectInIframe(files, path)
				return lint()
			},
			deleteFrontendFile: (path) => {
				if (!files) {
					files = {}
				}
				delete files[path]
				setFilesInIframe(files)
			},
			listBackendRunnables: () => {
				return Object.entries(runnables).map(([key, runnable]) => ({
					key,
					name: runnable?.name ?? key
				}))
			},
			getBackendRunnable: (key) => {
				return convertToBackendRunnable(key, runnables[key])
			},
			getBackendRunnables: () => {
				const backendRunnables: Record<string, any> = {}
				Object.entries(runnables).forEach(([key, runnable]) => {
					const converted = convertToBackendRunnable(key, runnable)
					if (converted) {
						backendRunnables[key] = converted
					}
				})
				return backendRunnables
			},
			setBackendRunnable: async (key, runnable): Promise<LintResult> => {
				if (runnable.type === 'inline' && runnable.inlineScript) {
					runnables[key] = {
						name: runnable.name,
						type: 'inline',
						inlineScript: {
							content: runnable.inlineScript.content,
							language: runnable.inlineScript.language
						},
						fields: runnable.staticInputs
							? Object.fromEntries(
									Object.entries(runnable.staticInputs).map(([k, v]) => [
										k,
										{ type: 'static', value: v, fieldType: 'object' }
									])
								)
							: {}
					}
				} else if (runnable.path) {
					runnables[key] = {
						name: runnable.name,
						type: 'path',
						runType: runnable.type as 'script' | 'flow' | 'hubscript',
						path: runnable.path,
						fields: runnable.staticInputs
							? Object.fromEntries(
									Object.entries(runnable.staticInputs).map(([k, v]) => [
										k,
										{ type: 'static', value: v, fieldType: 'object' }
									])
								)
							: {},
						schema: {}
					}
				}
				populateRunnables()

				// Switch UI to show this runnable so Monaco can analyze it
				selectedRunnable = key

				// Wait 2 seconds for Monaco to analyze the code
				await new Promise((resolve) => setTimeout(resolve, 1000))

				return lint()
			},
			deleteBackendRunnable: (key) => {
				delete runnables[key]
				rawAppLintStore.clearDiagnostics(key)
				populateRunnables()
			},
			getFiles: () => {
				return {
					frontend: $state.snapshot(files ?? {}),
					backend: aiChatManager.appAiChatHelpers?.getBackendRunnables() ?? {}
				}
			},
			getSelectedContext: () => {
				return createAppSelectedContext({
					inspectorElement: inspectorElement,
					clearInspector: clearInspectorSelection,
					codeSelection: codeSelection,
					clearCodeSelection: () => {
						codeSelection = undefined
					}
				})
			},
			snapshot: () => {
				// Force create snapshot for AI - it needs a restore point
				return (
					historyManager.manualSnapshot(files ?? {}, runnables, summary, data, true)?.id ??
					historyManager.getId()
				)
			},
			revertToSnapshot: (id: number) => {
				console.log('reverting to snapshot', id)
				handleHistorySelect(id)
			},
			listDatatableTables: async (): Promise<AppDatatableMetadata[]> => {
				if (!opWorkspace) {
					return []
				}

				const tables = await WorkspaceService.listDataTableTables({
					workspace: opWorkspace
				})
				return filterDatatableTables(tables)
			},
			getDatatableTableSchema: async (
				datatableName: string,
				schemaName: string,
				tableName: string
			): Promise<Record<string, string>> => {
				if (!opWorkspace) {
					return {}
				}

				if (!isDatatableTableWhitelisted(datatableName, schemaName, tableName)) {
					const tableRef = formatDataTableRef({
						datatable: datatableName,
						schema: schemaName === 'public' ? undefined : schemaName,
						table: tableName
					})
					throw new Error(`Table '${tableRef}' is not configured in this app`)
				}

				const schema = await WorkspaceService.getDataTableTableSchema({
					workspace: opWorkspace,
					datatableName,
					schemaName,
					tableName
				})
				return schema.columns
			},
			getAvailableDatatableNames: (): string[] => {
				// Get unique datatable names from dataTableRefs
				return [...dataTableWhitelist.datatables]
			},
			execDatatableSql: async (
				datatableName: string,
				sql: string,
				newTable?: { schema: string; name: string }
			): Promise<{ success: boolean; result?: Record<string, any>[]; error?: string }> => {
				if (!opWorkspace) {
					return { success: false, error: 'Workspace not available' }
				}

				try {
					const result = await runScriptAndPollResult({
						workspace: opWorkspace,
						requestBody: {
							language: 'postgresql',
							content: sql,
							args: { database: `datatable://${datatableName}` }
						}
					})

					// If newTable was specified and the query succeeded, add it to data.tables
					if (newTable) {
						const newRef = formatDataTableRef({
							datatable: datatableName,
							schema: newTable.schema === 'public' ? undefined : newTable.schema,
							table: newTable.name
						})
						// Only add if not already present
						if (!data.tables.includes(newRef)) {
							data.tables = [...data.tables, newRef]
							// Clear the cached schema so it gets refreshed with the new table
							const resourcePath = `datatable://${datatableName}`
							delete $dbSchemas[resourcePath]
							delete $dbSchemas[`${opWorkspace}:${resourcePath}`]
						}
					}

					void aiChatManager.refreshDatatables()

					// Check if result is an array (SELECT) or something else
					if (Array.isArray(result)) {
						return { success: true, result }
					} else {
						return { success: true, result: [] }
					}
				} catch (e) {
					const errorMsg = e instanceof Error ? e.message : String(e)
					return { success: false, error: errorMsg }
				}
			},
			addTableToWhitelist: (datatableName: string, schemaName: string, tableName: string) => {
				// Format the table reference
				const newRef = formatDataTableRef({
					datatable: datatableName,
					schema: schemaName === 'public' ? undefined : schemaName,
					table: tableName
				})
				// Only add if not already present
				if (!data.tables.includes(newRef)) {
					data.tables = [...data.tables, newRef]
					void aiChatManager.refreshDatatables()
				}
			}
		})
	})
	let selectedRunnable: string | undefined = $state(undefined)
	let selectedDocument: string | undefined = $state(undefined)
	let inspectorElement: InspectorElementInfo | undefined = $state(undefined)
	let codeSelection: AppCodeSelectionElement | undefined = $state(undefined)

	let modules = $state({}) as Modules

	// Normalize Windows-style path separators to Linux-style
	function normalizeFilePaths(
		filesObj: Record<string, string> | undefined
	): Record<string, string> {
		if (!filesObj) return {}
		return Object.fromEntries(
			Object.entries(filesObj).map(([path, content]) => [path.replace(/\\/g, '/'), content])
		)
	}

	function listener(e: MessageEvent) {
		// The detached preview window asks for the build every time it (re)loads,
		// including a manual browser refresh — its app-preview.html shell starts
		// blank and the one-shot `load` feed can't survive the tab reloading
		// itself. Re-feed it here so it repaints. Gated to our own window handle
		// AND a same-origin sender: the preview runs user app code that can
		// navigate the window away, and a cross-origin doc must not be able to
		// trigger a bundle replay (the build can carry app source/secrets).
		if (
			e.data?.type === 'appPreviewReady' &&
			e.source === externalPreviewWindow &&
			e.origin === window.location.origin
		) {
			feedExternalPreview()
			return
		}

		// Two children speak to us now: the UI Builder iframe (source editor)
		// and the preview iframe (rendered user app). Gate by source so they
		// can't be confused or spoofed.
		const fromUiBuilder = e.source === iframe?.contentWindow
		const fromPreview = e.source === previewIframe?.contentWindow
		if (!fromUiBuilder && !fromPreview) return

		// Build output: UI Builder finished bundling. Cache it and forward to
		// the preview iframe so it renders the new app.
		if (fromUiBuilder && e.data.type === 'preview') {
			lastBuild = { css: e.data.css, js: e.data.js }
			feedPreviewIframe(lastBuild)
			syncExternalPreview()
			return
		}

		// Build/bundler logs from the UI Builder iframe — rendered as an
		// overlay inside the preview pane (see the panel below). Two
		// shapes are accepted: a full snapshot (`setLogs`) for backwards
		// compat, and an incremental delta (`appendLogs`) used during
		// heavy bundler activity to avoid O(n²) postMessage traffic.
		if (fromUiBuilder && e.data.type === 'setLogs') {
			logs = String(e.data.logs ?? '')
			return
		}
		if (fromUiBuilder && e.data.type === 'appendLogs') {
			logs += String(e.data.delta ?? '')
			return
		}

		// `message: undefined` arrives on the next successful build and clears the banner.
		if (fromUiBuilder && e.data.type === 'buildError') {
			buildError = typeof e.data.message === 'string' ? e.data.message : undefined
			return
		}

		if (fromPreview && e.data.type === 'runtimeLogsResponse') {
			resolvePendingRuntimeLogRequest(e.data.requestId, normalizeRawAppRuntimeLogs(e.data.logs))
			return
		}

		// The build ran without mounting the app, so the preview is blank with
		// nothing to report — surfaced as a hint naming the missing mount call.
		// `renderAppeared` withdraws it if a mount lands after the grace window.
		if (fromPreview && e.data.type === 'emptyRender') {
			emptyRender = true
			return
		}
		if (fromPreview && e.data.type === 'renderAppeared') {
			emptyRender = false
			return
		}

		// Uncaught error/rejection from the rendered app — surfaced in the preview
		// overlay so a runtime crash isn't a silent blank error.
		if (fromPreview && e.data.type === 'runtimeError') {
			runtimeError =
				typeof e.data.message === 'string' && e.data.message
					? e.data.message
					: 'Unknown runtime error'
			return
		}

		// Inspector events come exclusively from the preview iframe.
		if (fromPreview && e.data.type === 'inspectorSelect') {
			inspectorElement = e.data.element as InspectorElementInfo
			// Session preview: forward the pick so it can be attached to the chat as a
			// selector context chip (the app-mode SelectedContext path is separate).
			// App mode picks one element then exits; the session stays on to keep
			// picking (the chip list, not the harness, holds the selection). Shift
			// held → add to the selection; a plain click replaces it.
			if (onInspectorSelect) onInspectorSelect(inspectorElement, !!e.data.additive)
			else inspectorEnabled = false
			return
		}
		if (fromPreview && e.data.type === 'inspectorDeselect') {
			// User clicked × on a selected overlay in the preview — drop that chip.
			if (typeof e.data.selector === 'string') onInspectorDeselect?.(e.data.selector)
			return
		}
		if (fromPreview && e.data.type === 'inspectorClear') {
			inspectorElement = undefined
			// A rebuild invalidates the selection — clear all session chips.
			onInspectorClearAll?.()
			return
		}

		// Everything below this point is editor metadata from the UI Builder.
		if (!fromUiBuilder) return

		if (e.data.type === 'setFiles') {
			// Normalize Windows-style path separators to Linux-style
			const normalizedFiles = normalizeFilePaths(e.data.files)
			// Only mark pending changes if files actually changed (ignore echo from setFilesInIframe)
			if (!deepEqual(files, normalizedFiles)) {
				files = normalizedFiles
				historyManager.markPendingChanges()
			}
		} else if (e.data.type === 'getBundle') {
			getBundleResolve?.(e.data.bundle)
		} else if (e.data.type === 'updateModules') {
			modules = e.data.modules
		} else if (e.data.type === 'setActiveDocument') {
			if (suppressSetActiveDocument) return
			// Normalize Windows-style path separators to Linux-style
			selectedDocument = e.data.path?.replace(/\\/g, '/')
			// If VS Code switched to a file we don't have a tab for (e.g. via
			// the file explorer's reveal-in-editor, or our own auto-open of
			// the main app file at boot), backfill a tab.
			if (selectedDocument) {
				const id = fileTabId(selectedDocument)
				if (!tabs.some((t) => t.id === id)) {
					ensureFileTab(selectedDocument)
					// Don't auto-activate — the user's tab choice wins.
					// But if no file tab is currently active, fall in line.
					// Skip this auto-activation in single-view-with-preview
					// mode (the caller seeded `defaultSplitWithPreview=false`
					// because Preview is the intended starting tab); the
					// iframe's first setActiveDocument shouldn't fight that.
					if (splitWithPreview && activeTabKind === 'preview' && tabs.length === 2) {
						activateTab(id)
					}
				}
			}
		} else if (e.data.type === 'editorSelection') {
			// Handle code selection from the iframe editor
			const selection = e.data.selection
			if (selection === null) {
				// Selection cleared
				codeSelection = undefined
			} else {
				// Normalize path
				const normalizedPath = selection.path?.replace(/\\/g, '/')
				codeSelection = {
					type: 'app_code_selection',
					source: normalizedPath,
					sourceType: 'frontend',
					title: `${normalizedPath}:L${selection.range.startLine}-L${selection.range.endLine}`,
					content: selection.content,
					startLine: selection.range.startLine,
					endLine: selection.range.endLine,
					startColumn: selection.range.startColumn,
					endColumn: selection.range.endColumn
				}
			}
		}
	}

	function postToExternalPreview(msg: Record<string, unknown>) {
		if (!externalPreviewWindow || externalPreviewWindow.closed) {
			externalPreviewWindow = null
			return
		}
		// Restrict to our own origin: the detached window loads same-origin
		// app-preview.html, but user app code can navigate it elsewhere — don't
		// post the build (potential app source/secrets) to a cross-origin doc.
		externalPreviewWindow.postMessage(msg, window.location.origin)
	}

	function syncExternalPreview() {
		if (lastBuild) {
			postToExternalPreview({ type: 'preview', css: lastBuild.css, js: lastBuild.js })
		}
	}

	// Feed a build into the inline preview iframe. Clears the previous run's
	// overlays first: a fresh render supersedes the old crash or blank, and
	// app-preview.html re-posts if the new render fails the same way.
	function feedPreviewIframe(build: { css: string; js: string }) {
		runtimeError = undefined
		emptyRender = false
		previewIframe?.contentWindow?.postMessage(
			{ type: 'preview', css: build.css, js: build.js },
			'*'
		)
	}

	// Full (re)feed of the detached window: theme first, then the build. Used
	// when (re)attaching to a window — open, focus-reuse, load, handshake — so
	// it always matches the editor's current state. Plain rebuilds use
	// `syncExternalPreview` alone (the theme hasn't changed).
	function feedExternalPreview() {
		postToExternalPreview({ type: 'setDarkMode', dark: darkMode })
		syncExternalPreview()
	}

	onDestroy(() => {
		// Don't leave a detached preview behind when the editor unmounts: it
		// would stop receiving builds and, once refreshed, has no opener to
		// re-feed it — a permanently blank orphan.
		if (externalPreviewWindow && !externalPreviewWindow.closed) externalPreviewWindow.close()
		externalPreviewWindow = null
	})

	function openExternalPreview() {
		// Reuse an already-open window instead of spawning duplicates.
		if (externalPreviewWindow && !externalPreviewWindow.closed) {
			externalPreviewWindow.focus()
			feedExternalPreview()
			return
		}
		// Scope the window name per app path so two open editors don't fight over
		// (or take over / close) one shared OS-level preview window.
		const win = window.open(
			'/ui_builder/app-preview.html',
			`windmillRawAppPreview:${encodeURIComponent(path)}`
		)
		if (!win) {
			sendUserToast('Could not open the preview window (popup blocked?)', true)
			return
		}
		externalPreviewWindow = win
		// Initial feed: fires once when the freshly opened tab loads. This is the
		// only feed path against an app-preview.html that predates the
		// `appPreviewReady` handshake, so the window isn't blank on first open
		// regardless of the pinned UI Builder artifact. A manual refresh is
		// covered separately by the handshake in `listener` (this listener is
		// bound to the now-stale document and won't fire again).
		win.addEventListener('load', () => feedExternalPreview())
	}

	let getBundleResolve: (({ css, js }: { css: string; js: string }) => void) | undefined = undefined

	async function getBundle(): Promise<{ css: string; js: string }> {
		return new Promise((resolve) => {
			getBundleResolve = resolve
			iframe?.contentWindow?.postMessage(
				{
					type: 'getBundle'
				},
				'*'
			)
		})
	}

	const RUNTIME_LOGS_TIMEOUT_MS = 2000
	type PendingRuntimeLogRequest = {
		resolve: (entries: RawAppRuntimeLogEntry[] | undefined) => void
		timer: ReturnType<typeof setTimeout>
	}
	const pendingRuntimeLogReqs = new Map<string, PendingRuntimeLogRequest>()

	function resolvePendingRuntimeLogRequest(
		requestId: string,
		entries: RawAppRuntimeLogEntry[] | undefined
	) {
		const pending = pendingRuntimeLogReqs.get(requestId)
		if (!pending) return
		clearTimeout(pending.timer)
		pendingRuntimeLogReqs.delete(requestId)
		pending.resolve(entries)
	}

	const requestRuntimeLogs: RawAppRuntimeLogRequester = (limit) => {
		const win = previewIframe?.contentWindow
		if (!win || !previewIframeLoaded) return Promise.resolve(undefined)
		const requestId = randomUUID()
		return new Promise<RawAppRuntimeLogEntry[] | undefined>((resolve) => {
			const timer = setTimeout(() => {
				resolvePendingRuntimeLogRequest(requestId, undefined)
			}, RUNTIME_LOGS_TIMEOUT_MS)
			pendingRuntimeLogReqs.set(requestId, { resolve, timer })
			win.postMessage({ type: 'getRuntimeLogs', requestId, limit }, '*')
		})
	}

	// Live DOM inspection for the session chat. Same-origin: the preview iframe is a
	// same-origin document (see the load listener below that reads its contentWindow),
	// so we read `contentDocument` directly — no postMessage, no ui_builder change. The
	// element is re-read on every call, so the model always sees the current render.
	const requestDomQuery: RawAppDomRequester = async (query: RawAppDomQuery) => {
		const doc = previewIframe?.contentDocument
		if (!doc || !previewIframeLoaded) return undefined
		const selector = query.selector?.trim()
		let el: Element | null
		let matchCount: number
		if (selector) {
			let matches: NodeListOf<Element>
			try {
				matches = doc.querySelectorAll(selector)
			} catch (e) {
				return {
					text: `Invalid CSS selector "${selector}": ${e instanceof Error ? e.message : String(e)}`
				}
			}
			matchCount = matches.length
			el = matches[0] ?? null
		} else {
			el = doc.body
			matchCount = el ? 1 : 0
		}
		if (!el) {
			return {
				text: `No element matches selector "${selector}". It may not be rendered yet, or the selector is wrong. Try a broader selector or omit it to read the whole page.`
			}
		}
		// A <script> is source the browser executes, not rendered output; an
		// `.inspector-label` is chrome this inspector injects. The descendant strips
		// below can't reach either when it IS the clone root (querySelectorAll skips
		// the root), so reject such a root here — otherwise a `script` query would
		// serialize the whole compiled bundle and an `.inspector-label` query would
		// return inspector chrome.
		if (el.tagName.toLowerCase() === 'script') {
			return {
				text: `The "${selector}" selector matches a <script> element — source code the browser executes, not rendered output. Omit the selector to read the whole page, or target a rendered element.`
			}
		}
		if (el.classList.contains('inspector-label')) {
			return {
				text: `The "${selector}" selector matches an inspector label — UI chrome injected by the element picker, not part of the app. Target a rendered app element instead.`
			}
		}
		// Strip the inspector's own artifacts so the model never sees them: the
		// label pills it injects into <body>, and the outline classes it adds to
		// app elements (highlights are element outlines, not overlay nodes).
		const clone = el.cloneNode(true) as Element
		// The preview harness appends the compiled app bundle as a <script> under
		// <body>. It is source, not rendered output: serializing it would let
		// search_dom match strings that exist only in source and pollute / truncate
		// read_dom's line window with bundle JS. Drop every script from the clone.
		clone.querySelectorAll('script').forEach((n) => n.remove())
		clone.querySelectorAll('.inspector-label').forEach((n) => n.remove())
		// The outline classes sit on the selected element itself (the clone root),
		// not only its descendants — querySelectorAll skips the root, so strip it too.
		clone.classList.remove('inspector-hover', 'inspector-picked')
		clone
			.querySelectorAll('.inspector-hover, .inspector-picked')
			.forEach((n) => n.classList.remove('inspector-hover', 'inspector-picked'))
		const text = await runDomQueryOnHtml(clone.outerHTML, query, {
			selector: selector ?? 'body',
			tagName: el.tagName.toLowerCase(),
			matchCount
		})
		return { text }
	}

	// ---- Inline "prompt this element" mini-composer (session preview only) ----
	// Anchored over the most-recently selected element in the live preview. It's a
	// host overlay (position: fixed in the top document) computed from the element's
	// rect inside the same-origin preview iframe, repositioned as the preview scrolls.
	let inlinePromptDismissed = $state(false)
	let inlinePromptPos = $state<{ x: number; y: number } | undefined>(undefined)
	let inlinePromptLabel = $state('')
	const inlinePromptSelector = $derived(
		onInlinePrompt && selectedDomSelectors.length > 0
			? selectedDomSelectors[selectedDomSelectors.length - 1]
			: undefined
	)

	function shortElementLabel(el: Element): string {
		const tag = el.tagName.toLowerCase()
		const id = el.id ? `#${el.id}` : ''
		const cls =
			typeof el.className === 'string'
				? el.className
						.trim()
						.split(/\s+/)
						.filter((c) => c && !c.startsWith('inspector-'))[0]
				: undefined
		return `${tag}${id}${cls ? `.${cls}` : ''}`
	}

	function updateInlinePromptPos() {
		const sel = inlinePromptSelector
		const doc = previewIframe?.contentDocument
		if (!sel || !doc || !previewIframeLoaded || inlinePromptDismissed) {
			inlinePromptPos = undefined
			return
		}
		let el: Element | null
		try {
			el = doc.querySelector(sel)
		} catch (_) {
			el = null
		}
		if (!el) {
			inlinePromptPos = undefined
			return
		}
		const r = el.getBoundingClientRect()
		const ir = previewIframe!.getBoundingClientRect()
		// Hide while the element is scrolled outside the preview's visible area.
		if (r.bottom < 0 || r.top > ir.height || r.right < 0 || r.left > ir.width) {
			inlinePromptPos = undefined
			return
		}
		const inputW = 260
		const inputH = 42
		// The harness draws the element's name+size pill ~20px above its top edge;
		// clear it so the input sits above the label rather than covering it.
		const labelGutter = 16
		let top = ir.top + r.top - inputH - labelGutter
		// No room above → drop it just below the element's top edge instead (the
		// label is above the element, so this still doesn't cover it).
		if (top < ir.top + 4) top = ir.top + Math.max(r.top, 0) + 4
		// Anchored to the element's top-left edge (aligned with the name+size pill),
		// clamped to the viewport.
		let left = ir.left + r.left
		left = Math.max(4, Math.min(left, window.innerWidth - inputW - 4))
		inlinePromptPos = { x: left, y: top }
		inlinePromptLabel = shortElementLabel(el)
	}

	// Reset the dismissed flag whenever the anchored element changes.
	$effect(() => {
		inlinePromptSelector
		untrack(() => {
			inlinePromptDismissed = false
		})
	})

	$effect(() => {
		// Recompute on selection change, preview (re)load, or dismiss; and keep the
		// overlay pinned as the preview (or the host) scrolls/resizes.
		inlinePromptSelector
		selectedDomSelectors
		previewIframeLoaded
		inlinePromptDismissed
		updateInlinePromptPos()
		if (!previewIframe || !previewIframeLoaded) return
		const win = previewIframe.contentWindow
		const onScroll = () => updateInlinePromptPos()
		win?.addEventListener('scroll', onScroll, true)
		window.addEventListener('scroll', onScroll, true)
		window.addEventListener('resize', onScroll)
		return () => {
			win?.removeEventListener('scroll', onScroll, true)
			window.removeEventListener('scroll', onScroll, true)
			window.removeEventListener('resize', onScroll)
		}
	})

	const getRuns: RawAppRunsProvider = () => {
		const out: RawAppRunSummary[] = []
		for (const id of jobs) {
			const j = jobsById[id]
			if (!j) continue
			const run: RawAppRunSummary = {
				job_id: j.job ?? id,
				component: j.component,
				status: j.result !== undefined || j.duration_ms !== undefined ? 'completed' : 'running'
			}
			if (j.created_at !== undefined) run.created_at = j.created_at
			if (j.started_at !== undefined) run.started_at = j.started_at
			if (j.duration_ms !== undefined) run.duration_ms = j.duration_ms
			out.push(run)
		}
		return out.reverse()
	}

	// Only values whose non-wrapping counterpart collapses whitespace identically.
	// `pre-line`/`break-spaces` have no such counterpart: forcing them to nowrap
	// would eat their preserved newlines, so they are left to re-wrap.
	const NON_WRAPPING_EQUIVALENT: Record<string, string> = {
		normal: 'nowrap',
		'pre-wrap': 'pre'
	}

	// getClientRects yields a rect per contained node, not per line box, so the
	// count alone says nothing: `a <b>b</b>` is two rects on one line. Rects
	// sharing a line overlap vertically, and `top` alone would split a line that
	// mixes font sizes — so count vertically disjoint runs.
	function countLines(range: Range): number {
		const rects = Array.from(range.getClientRects()).filter((r) => r.width > 0 || r.height > 0)
		if (rects.length === 0) return 0
		rects.sort((a, b) => a.top - b.top)
		let lines = 1
		let lineBottom = rects[0].bottom
		for (const r of rects) {
			if (r.top >= lineBottom) {
				lines++
				lineBottom = r.bottom
			} else {
				lineBottom = Math.max(lineBottom, r.bottom)
			}
		}
		return lines
	}

	// A box that shrink-wraps its text can have zero sub-pixel slack (a 208.59px box
	// holding a 208.59px text run). The capture re-runs layout in whole pixels, so
	// the text no longer fits, wraps, and is then clipped out of the box entirely.
	// Pinning runs that are already single-line is a no-op on the live DOM but stops
	// the re-layout from re-deciding where they break.
	function pinSingleLineText(root: HTMLElement): () => void {
		const doc = root.ownerDocument
		const view = doc.defaultView
		if (!view) return () => {}
		// Measure every candidate before mutating any of them: interleaving reads and
		// writes forces a synchronous reflow per element.
		const pending: Array<[HTMLElement, string]> = []
		const walker = doc.createTreeWalker(root, NodeFilter.SHOW_ELEMENT)
		let node: Node | null
		while ((node = walker.nextNode())) {
			const el = node as HTMLElement
			if (!(el instanceof view.HTMLElement)) continue
			const hasOwnText = Array.from(el.childNodes).some(
				(c) => c.nodeType === Node.TEXT_NODE && (c.textContent ?? '').trim() !== ''
			)
			if (!hasOwnText) continue
			const replacement = NON_WRAPPING_EQUIVALENT[view.getComputedStyle(el).whiteSpace]
			if (!replacement) continue
			const range = doc.createRange()
			range.selectNodeContents(el)
			if (countLines(range) !== 1) continue // already wraps — leave its breaks alone
			pending.push([el, replacement])
		}
		const restores = pending.map(([el, replacement]) => {
			const prev = el.style.getPropertyValue('white-space')
			const prio = el.style.getPropertyPriority('white-space')
			el.style.setProperty('white-space', replacement, 'important')
			return () => {
				if (prev) el.style.setProperty('white-space', prev, prio)
				else el.style.removeProperty('white-space')
			}
		})
		return () => restores.forEach((r) => r())
	}

	// Capture the live preview as a PNG data URL. The preview iframe
	// (/ui_builder/app-preview.html) is same-origin with no sandbox, so its rendered
	// document is reachable and can be serialized from here. There is no native
	// element-screenshot API; modern-screenshot reconstructs the DOM into an SVG
	// foreignObject, so a WebGL canvas is only captured when its context was created
	// with preserveDrawingBuffer. Lazy-imported so the library only loads on demand.
	const captureScreenshot: RawAppScreenshotRequester = async () => {
		const target = previewIframe?.contentDocument?.body
		if (!previewIframe || !previewIframeLoaded || !target) {
			throw new Error('App preview is not ready')
		}
		// Collapsing the preview leaves the iframe mounted and populated at zero
		// width, which passes every check above and then fails inside the rasteriser
		// as an opaque decode error. Name the cause so the agent can act on it.
		if (!target.clientWidth || !target.clientHeight) {
			throw new Error(
				'The app preview is collapsed, so there is nothing to capture. Ask the user to expand the preview panel, then try again.'
			)
		}
		const { domToPng } = await import('modern-screenshot')
		// Above CSS resolution for small previews (a 1× capture of a ~900px preview
		// reads blurry next to the live render), sub-1× for oversized bodies — see
		// captureScale. maximumCanvasSize is the belt over that math: the rasterised
		// box can exceed the body's client size, and an unbounded canvas on a tall
		// scrolling app can freeze the tab before normalize ever bounds the pixels.
		const scale = captureScale(Math.max(target.clientWidth, target.clientHeight))
		const restore = pinSingleLineText(target)
		try {
			return await domToPng(target, {
				backgroundColor: '#ffffff',
				scale,
				maximumCanvasSize: MAX_IMAGE_EDGE
			})
		} finally {
			restore()
		}
	}

	onMount(() => {
		onRuntimeLogRequester?.(requestRuntimeLogs)
		onRunsProvider?.(getRuns)
		onDomRequester?.(requestDomQuery)
		onScreenshotRequester?.(captureScreenshot)
		return () => {
			onRuntimeLogRequester?.(undefined)
			onRunsProvider?.(undefined)
			onDomRequester?.(undefined)
			onScreenshotRequester?.(undefined)
			for (const requestId of Array.from(pendingRuntimeLogReqs.keys()))
				resolvePendingRuntimeLogRequest(requestId, undefined)
		}
	})

	let darkMode: boolean = $state(false)
	// Host's computed `text-xs` size in px. Windmill bumps :root to 18px at
	// ≥1760px viewports, so this re-evaluates on resize via the listener below.
	let editorFontSize = $state(12)
	function recomputeEditorFontSize() {
		const rootPx = parseFloat(getComputedStyle(document.documentElement).fontSize)
		// text-xs is 0.75rem
		editorFontSize = rootPx * 0.75
	}
	$effect(() => {
		recomputeEditorFontSize()
		const onResize = () => recomputeEditorFontSize()
		window.addEventListener('resize', onResize)
		return () => window.removeEventListener('resize', onResize)
	})
	$effect(() => {
		iframe?.addEventListener('load', () => {
			iframeLoaded = true
		})
	})
	$effect(() => {
		previewIframe?.addEventListener('load', () => {
			previewIframeLoaded = true
			// Replay the last build so the preview repopulates without
			// waiting for the user to trigger another bundle.
			if (lastBuild) {
				feedPreviewIframe(lastBuild)
			}
			// Escape inside the preview exits inspect mode — the keydown fires in
			// the iframe's document, so the parent window listener can't see it.
			// We also want Escape to dismiss a lingering green "selected" overlay
			// after the user picked an element (which auto-disables hover).
			previewIframe?.contentWindow?.addEventListener(
				'keydown',
				(e) => {
					if (
						e.key === 'Escape' &&
						(inspectorEnabled || inspectorElement || selectedDomSelectors.length > 0)
					) {
						disableInspector()
					}
				},
				true
			)
		})
	})
	$effect(() => {
		// Push dark mode to both children. The UI Builder iframe and the
		// preview iframe each listen for `setDarkMode` separately.
		if (iframe && iframeLoaded) {
			iframe.contentWindow?.postMessage({ type: 'setDarkMode', dark: darkMode }, '*')
		}
		if (previewIframe && previewIframeLoaded) {
			previewIframe.contentWindow?.postMessage({ type: 'setDarkMode', dark: darkMode }, '*')
		}
		postToExternalPreview({ type: 'setDarkMode', dark: darkMode })
	})
	$effect(() => {
		// Match VS Code's editor font size to Windmill's text-xs.
		if (iframe && iframeLoaded) {
			iframe.contentWindow?.postMessage({ type: 'setFontSize', px: editorFontSize }, '*')
		}
	})
	$effect(() => {
		// Push the chat's DOM-selector chips into the preview (source of truth):
		// the harness renders one highlight per selector. Re-posts on every
		// selection change and on preview (re)load.
		const selectors = selectedDomSelectors
		if (previewIframe && previewIframeLoaded) {
			previewIframe.contentWindow?.postMessage(
				{ type: 'inspectorSetSelection', selectors: [...selectors] },
				'*'
			)
		}
	})
	$effect(() => {
		iframe && iframeLoaded && files && populateFiles()
	})
	$effect(() => {
		iframe && iframeLoaded && runnables && populateRunnables()
	})
	$effect(() => {
		// Re-push the shared UI whenever the iframe (re)loads or the
		// fetched files change. We gate on `sharedUiLoaded` so the empty
		// initial state isn't sent before the API call resolves.
		if (iframe && iframeLoaded && sharedUiLoaded) {
			// Touch the version so reassignments after a refresh re-fire this.
			void sharedUiVersion
			untrack(() => setSharedUiInIframe())
		}
	})

	function clearInspectorSelection() {
		inspectorElement = undefined
		// Inspector lives in the preview iframe, so clear its overlay there.
		previewIframe?.contentWindow?.postMessage({ type: 'inspectorClear' }, '*')
	}

	function handleSelectFile(path: string) {
		// Adding the tab activates it; activateTab posts the selectFile message
		// to the UI Builder iframe and clears any selected runnable.
		const id = ensureFileTab(path)
		activateTab(id)
	}

	// Track previous values for change detection
	let prevSelectedRunnable: string | undefined = undefined
	let prevSelectedDocument: string | undefined = undefined

	// Clear inspector when selection changes
	$effect(() => {
		if (selectedRunnable !== prevSelectedRunnable || selectedDocument !== prevSelectedDocument) {
			// Only clear if we're actually switching to something different
			if (prevSelectedRunnable !== undefined || prevSelectedDocument !== undefined) {
				clearInspectorSelection()
			}
			prevSelectedRunnable = selectedRunnable
			prevSelectedDocument = selectedDocument
		}
	})

	// Mirror sidebar runnable selection into the tab system. When the user
	// picks a runnable from the sidebar, `selectedRunnable` flips via
	// `bind:selectedRunnable`; ensure a tab for it exists and is active.
	$effect(() => {
		const key = selectedRunnable
		if (!key) return
		const id = runnableTabId(key)
		untrack(() => {
			if (!tabs.some((t) => t.id === id)) ensureRunnableTab(key)
			if (activeTabId !== id) activeTabId = id
		})
	})

	// Open a default file on mount (boots the iframe in split mode and gives
	// the user something to edit on the left). When the caller seeded
	// `defaultSplitWithPreview=false` we instead want the Preview tab as the
	// only-visible / active surface, so skip the file-tab activation — the
	// iframe still boots via `populateFiles`/`setFilesInIframe` even without
	// a selected document.
	onMount(() => {
		if (!splitWithPreview) return
		if (tabs.length === 1) {
			const def = pickDefaultFile(files)
			if (def) activateTab(ensureFileTab(def))
		}
	})

	// Drop tabs whose file/runnable no longer exists.
	$effect(() => {
		void files
		void runnables
		untrack(() => {
			const filesSet = files ?? {}
			const runnablesSet = runnables ?? {}
			const stale = tabs.filter((t) => {
				if (t.id.startsWith(FILE_PREFIX)) {
					const fp = t.id.slice(FILE_PREFIX.length)
					return filesSet[fp] === undefined
				}
				if (t.id.startsWith(RUNNABLE_PREFIX)) {
					const k = t.id.slice(RUNNABLE_PREFIX.length)
					return runnablesSet[k] === undefined
				}
				return false
			})
			for (const t of stale) closeTab(t.id)
		})
	})

	function handleUndo() {
		// Create a snapshot if we're at the latest position with pending changes
		if (historyManager.needsSnapshotBeforeNav) {
			historyManager.manualSnapshot(files ?? {}, runnables, summary, data)
		}

		const entry = historyManager.undo()
		if (entry) {
			applyEntry(entry)
		}
	}

	function handleRedo() {
		const entry = historyManager.redo()
		if (entry) {
			applyEntry(entry)
		}
	}

	function handleHistorySelect(id: number) {
		// Save current state temporarily (not as a snapshot) when navigating to history
		// Only if we're currently at the "current" state (not already viewing history)
		if (historyManager.selectedEntryId === undefined) {
			historyManager.saveTemporaryCurrentState(files ?? {}, runnables, summary, data)
		}

		const entry = historyManager.selectEntry(id)
		if (entry) {
			applyEntry(entry)
		}
	}

	function applyEntry(entry: {
		files: Record<string, string>
		runnables: Record<string, Runnable>
		summary: string
		data: RawAppData
	}) {
		try {
			files = structuredClone($state.snapshot(entry.files))
			runnables = structuredClone($state.snapshot(entry.runnables))
			summary = entry.summary
			data = structuredClone($state.snapshot(entry.data))

			// If there's a selected document that exists in the new files, use the combined message
			if (selectedDocument && entry.files[selectedDocument] !== undefined) {
				// Use combined setFilesAndSelect message to avoid race condition
				setFilesAndSelectInIframe(entry.files, selectedDocument)
			} else {
				// Otherwise just set files normally
				setFilesInIframe(entry.files)
			}
			populateRunnables()
		} catch (error) {
			console.error('Failed to apply entry:', error)
			sendUserToast('Failed to apply entry: ' + (error as Error).message, true)
		}
	}

	function disableInspector() {
		// Picking an element auto-clears `inspectorEnabled`, so Escape after a
		// pick gets here with hover already off but the green selection still
		// up. In a session the chat's DOM chips are the source of truth for the
		// selection (highlights + the inline prompt), so a chip-only selection
		// (picking already toggled off) must still be dismissable. Bail only
		// when there is genuinely nothing to dismiss.
		if (!inspectorEnabled && !inspectorElement && selectedDomSelectors.length === 0) return
		inspectorEnabled = false
		// `inspectorDisable` only stops hover/click; the green "selected"
		// overlay from a prior pick persists until we explicitly clear it.
		// Escape should reset both, so the iframe goes back to its idle look.
		previewIframe?.contentWindow?.postMessage({ type: 'inspectorDisable' }, '*')
		previewIframe?.contentWindow?.postMessage({ type: 'inspectorClear' }, '*')
		inspectorElement = undefined
		// Session mode: clear the chips too. Emptying the selection hides the
		// inline prompt (it's driven by the chips) and drops the highlights (the
		// push effect re-posts the now-empty set) — otherwise the overlays clear
		// but the chips + inline prompt stay stranded.
		onInspectorClearAll?.()
		inlinePromptDismissed = true
	}

	// Escape exits inspect mode. We listen in the capture phase because a global
	// handler swallows Escape before it bubbles to <svelte:window>. The preview
	// iframe (separate document) is covered by its own listener on load.
	$effect(() => {
		const onEscapeCapture = (e: KeyboardEvent) => {
			if (
				e.key === 'Escape' &&
				(inspectorEnabled || inspectorElement || selectedDomSelectors.length > 0)
			) {
				disableInspector()
				e.stopImmediatePropagation()
				e.preventDefault()
			}
		}
		window.addEventListener('keydown', onEscapeCapture, true)
		return () => window.removeEventListener('keydown', onEscapeCapture, true)
	})

	// Force an immediate flush. No toast — the AutosaveIndicator narrates the
	// result, and `flush` never rejects (postSave routes errors to the failures map).
	function flushDraft() {
		if (!opWorkspace || !liveEditorDraftStoragePath) return
		void UserDraftDbSyncer.flush({
			workspace: opWorkspace,
			itemKind: 'raw_app',
			path: liveEditorDraftStoragePath
		})
	}

	// The VS Code workbench iframe's keydowns don't bubble out, so the window
	// handler can't see Ctrl/Cmd+S while editing code. Attach a capture listener
	// inside the iframe per load (it dies with the iframe, so no leak). No
	// preventDefault: VS Code's own save still runs; we just flush alongside it.
	function attachIframeSaveShortcut() {
		const win = iframe?.contentWindow
		if (!win) return
		win.addEventListener(
			'keydown',
			(e: KeyboardEvent) => {
				if ((e.ctrlKey || e.metaKey) && !e.shiftKey && (e.key === 's' || e.key === 'S')) {
					flushDraft()
				}
			},
			true
		)
	}

	// Monaco swallows Ctrl/Cmd+S in inline editors; Editor/SimpleEditor
	// re-broadcast it as `wm-monaco-save-shortcut` (untyped, hence manual listener).
	$effect(() => {
		window.addEventListener('wm-monaco-save-shortcut', flushDraft)
		return () => window.removeEventListener('wm-monaco-save-shortcut', flushDraft)
	})

	function handleKeydown(e: KeyboardEvent) {
		// Ctrl/Cmd + S — catch this BEFORE the input/Monaco guard below so
		// the shortcut fires regardless of focus.
		if ((e.ctrlKey || e.metaKey) && !e.shiftKey && (e.key === 's' || e.key === 'S')) {
			e.preventDefault()
			flushDraft()
			return
		}

		// Skip when typing in an input, textarea, or Monaco editor.
		const classes = (e.target as HTMLElement | null)?.className
		if (
			(typeof classes === 'string' && classes.includes('inputarea')) ||
			['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName ?? '')
		) {
			return
		}

		// Ctrl/Cmd + Shift + H for manual snapshot
		if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'H') {
			e.preventDefault()
			historyManager.manualSnapshot(files ?? {}, runnables, summary, data)
			return
		}

		// Ctrl/Cmd + B toggles the file sidebar
		if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key === 'b') {
			e.preventDefault()
			sidebarCollapsed.val = !sidebarCollapsed.val
		}
	}
</script>

<svelte:window onmessage={listener} onkeydown={handleKeydown} />
<DarkModeObserver bind:darkMode />

<RawAppBackgroundRunner
	workspace={opWorkspace ?? ''}
	editor
	iframe={previewIframe}
	bind:jobs
	bind:jobsById
	{runnables}
	{path}
	gateJobIds={false}
	extraSourceWindow={() => externalPreviewWindow}
/>
<div bind:clientWidth={rootWidth} class="max-h-full overflow-hidden h-full min-h-0 flex flex-col">
	<RawAppEditorHeader
		bind:jobs
		bind:jobsById
		bind:savedApp
		bind:summary
		bind:pendingDraftPath
		{onRestore}
		{onSavedNewAppPath}
		{policy}
		{diffDrawer}
		{newApp}
		{newPath}
		{labels}
		appPath={path}
		{liveEditorDraftStoragePath}
		{autosaveWorkspace}
		{autosavePath}
		{files}
		{data}
		{runnables}
		{getBundle}
		{onNavigate}
		{onDeploy}
		{onResetToDeployed}
		{loadedFromDraft}
		{othersDraftsCount}
		{onOpenOthersDrafts}
		canUndo={historyManager.canUndo}
		canRedo={historyManager.canRedo}
		onUndo={handleUndo}
		onRedo={handleRedo}
		onOpenYamlEditor={() => yamlEditorDrawer?.openDrawer()}
		sidebarCollapsed={sidebarCollapsed.val}
		onToggleSidebar={() => (sidebarCollapsed.val = !sidebarCollapsed.val)}
		{condensedHeader}
	/>

	<RawAppYamlEditor
		bind:drawer={yamlEditorDrawer}
		{summary}
		{files}
		{runnables}
		{data}
		onApply={handleYamlApply}
	/>

	<div bind:clientWidth={splitContainerWidth} class="grow min-h-0 flex flex-col">
		<Splitpanes id="o2" class="grow min-h-0 border-t">
			{#if !sidebarCollapsed.val}
				<Pane
					bind:size={() => sidebarPanelSize, (v) => (rawSidebarSize = v)}
					minSize={sidebarMinPercent}
					class="h-full overflow-y-auto relative"
				>
					<RawAppSidebar
						bind:files={
							() => files,
							(newFiles) => {
								files = newFiles
								setFilesInIframe(newFiles ?? {})
							}
						}
						onSelectFile={handleSelectFile}
						bind:selectedRunnable
						bind:selectedDocument
						dataTableRefs={dataTableRefsObjects}
						onDataTableRefsChange={(newRefs) => {
							data.tables = newRefs.map(formatDataTableRef)
						}}
						defaultDatatable={data.datatable}
						defaultSchema={data.schema}
						onDefaultChange={(datatable, schema) => {
							data.datatable = datatable
							data.schema = schema
							// Also sync to aiChatManager
							aiChatManager.datatableCreationPolicy = {
								...aiChatManager.datatableCreationPolicy,
								datatable,
								schema
							}
						}}
						{runnables}
						{modules}
						{historyManager}
						historySelectedId={historyManager.selectedEntryId}
						onHistorySelect={handleHistorySelect}
						onHistorySelectCurrent={() => {
							// Restore the temporary current state if it exists
							const tempState = historyManager.getAndClearTemporaryState()
							if (tempState) {
								applyEntry(tempState)
							}
							// Clear selection to indicate we're at current state
							historyManager.clearSelection()
						}}
						onManualSnapshot={() => {
							historyManager.manualSnapshot(files ?? {}, runnables, summary, data, true)
						}}
					></RawAppSidebar>
				</Pane>
			{/if}
			<Pane>
				<!--
				Per-pane tab bars (VS Code-style). Each pane carries its own
				DraggableTabs instance so the splitter goes floor-to-ceiling
				through tabs AND content in split mode. In single mode both
				bars mirror the FULL tab list — whichever pane is visible
				keeps every tab accessible, fixing the bug where activating
				Preview previously hid every tab.
			-->
				<div
					bind:clientWidth={editorAreaWidth}
					class="h-full w-full min-h-0 {splitWithPreview && activeTabKind !== 'preview'
						? 'tabs-content-split'
						: 'tabs-content-single'}"
				>
					<Splitpanes>
						<Pane bind:size={() => paneALeftSize, (v) => rememberPaneDrag(v)} minSize={0}>
							<div class="flex flex-col h-full w-full min-h-0">
								<DraggableTabs
									tabs={leftPaneTabs}
									activeId={activeTabId}
									onSelect={(id) => activateTab(id)}
									onClose={(id) => closeTab(id)}
									onReorder={(next) => reorderTabs(next)}
								>
									{#snippet trailing()}
										<div class="flex items-center gap-1 px-2">
											<button
												title={splitWithPreview
													? 'Move preview back into a tab'
													: 'Pin preview to the right'}
												aria-label="Toggle split with preview"
												aria-pressed={splitWithPreview}
												class={splitWithPreview
													? 'cursor-pointer bg-surface-accent-selected text-accent border border-border-selected w-7 h-7 rounded-md inline-flex items-center justify-center'
													: 'cursor-pointer bg-surface hover:bg-surface-hover border border-border-light text-primary w-7 h-7 rounded-md inline-flex items-center justify-center'}
												onclick={toggleSplit}
											>
												<Columns2 size={14} />
											</button>
										</div>
									{/snippet}
								</DraggableTabs>
								<div class="flex-1 min-h-0 relative">
									<!--
									Keep the UI Builder iframe mounted at a real (non-zero) size even
									when it isn't the active tab: a hidden 0×0 mount crashes the VS
									Code workbench's layout ("Unable to figure out browser width and
									height") and wedges it on "Loading editor" with no recovery. While
									inactive we size it to the editor area's width and hide it with
									`visibility` (not `display`), so Monaco boots correctly and
									revealing a file is an instant unhide.
									-->
									<div
										class="absolute inset-0"
										style={showSource
											? ''
											: `right: auto; width: ${editorAreaWidth || 800}px; visibility: hidden; pointer-events: none;`}
									>
										{#if iframeShouldMount}
											<iframe
												bind:this={iframe}
												title="UI builder"
												src="/ui_builder/index.html"
												class="w-full h-full block"
												onload={attachIframeSaveShortcut}
											></iframe>
										{/if}
									</div>
									<div class="absolute inset-0" style="display: {showRunnable ? 'block' : 'none'}">
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<div class="flex h-full w-full">
											{#if selectedRunnable !== undefined}
												<RawAppInlineScriptsPanel
													appPath={path}
													{selectedRunnable}
													bind:runnables
													onSelectionChange={(selection) => {
														if (selection === null) {
															codeSelection = undefined
														} else if (selectedRunnable) {
															codeSelection = {
																type: 'app_code_selection',
																source: selectedRunnable,
																sourceType: 'backend',
																title: `${selectedRunnable}:L${selection.startLine}-L${selection.endLine}`,
																content: selection.content,
																startLine: selection.startLine,
																endLine: selection.endLine,
																startColumn: selection.startColumn,
																endColumn: selection.endColumn
															}
														}
													}}
												/>
											{/if}
										</div>
									</div>
								</div>
							</div>
						</Pane>
						<Pane bind:size={() => paneBRightSize, (v) => rememberPaneDrag(100 - v)} minSize={0}>
							<div class="flex flex-col h-full w-full min-h-0 relative">
								<DraggableTabs
									tabs={rightPaneTabs}
									activeId={rightPaneActiveId}
									onSelect={(id) => activateTab(id)}
									onClose={(id) => closeTab(id)}
									onReorder={(next) => reorderTabs(next)}
								>
									{#snippet trailing()}
										<div class="flex items-center gap-1 px-2">
											<button
												class="cursor-pointer bg-surface hover:bg-surface-hover border border-border-light text-primary px-2 h-7 rounded-md text-xs"
												title="Switch bundler"
												onclick={() => {
													const next = bundlerType === 'esbuild' ? 'rolldown' : 'esbuild'
													bundlerType = next
													iframe?.contentWindow?.postMessage(
														{ type: 'setBundlerType', bundlerType: next },
														'*'
													)
												}}>{bundlerType}</button
											>
											<button
												title={inspectorEnabled
													? 'Click to disable element inspector'
													: 'Click to enable element inspector'}
												class={inspectorEnabled
													? 'cursor-pointer bg-surface-accent-selected text-accent border border-border-selected w-7 h-7 rounded-md inline-flex items-center justify-center'
													: 'cursor-pointer bg-surface hover:bg-surface-hover border border-border-light text-primary w-7 h-7 rounded-md inline-flex items-center justify-center'}
												aria-label="Toggle element inspector"
												onclick={() => {
													if (inspectorEnabled) {
														// Turning off is a full exit: stop picking and clear the
														// selection + inline prompt (mirrors Escape).
														disableInspector()
													} else {
														inspectorEnabled = true
														previewIframe?.contentWindow?.postMessage(
															{ type: 'inspectorEnable' },
															'*'
														)
													}
												}}
											>
												<MousePointerSquareDashed size={14} />
											</button>
											<button
												class="cursor-pointer bg-surface hover:bg-surface-hover border border-border-light text-primary w-7 h-7 rounded-md inline-flex items-center justify-center"
												title="Replay the last build into the preview"
												aria-label="Rebuild"
												onclick={() => {
													if (lastBuild) {
														feedPreviewIframe(lastBuild)
													}
												}}
											>
												<RefreshCw size={14} />
											</button>
											<button
												class="cursor-pointer bg-surface hover:bg-surface-hover border border-border-light text-primary w-7 h-7 rounded-md inline-flex items-center justify-center"
												title="Open preview in a separate window"
												aria-label="Open preview in a separate window"
												onclick={openExternalPreview}
											>
												<SquareArrowOutUpRight size={14} />
											</button>
											<button
												title={splitWithPreview
													? 'Move preview back into a tab'
													: 'Pin preview to the right'}
												aria-label="Toggle split with preview"
												aria-pressed={splitWithPreview}
												class={splitWithPreview
													? 'cursor-pointer bg-surface-accent-selected text-accent border border-border-selected w-7 h-7 rounded-md inline-flex items-center justify-center'
													: 'cursor-pointer bg-surface hover:bg-surface-hover border border-border-light text-primary w-7 h-7 rounded-md inline-flex items-center justify-center'}
												onclick={toggleSplit}
											>
												<Columns2 size={14} />
											</button>
										</div>
									{/snippet}
								</DraggableTabs>
								<iframe
									bind:this={previewIframe}
									title="App preview"
									src="/ui_builder/app-preview.html"
									class="w-full flex-1 block"
								></iframe>
								{#if buildError}
									<!-- top-12 clears the tab bar; `before:bg-surface` backs the
									     Alert's translucent red; `isolate` pins the pseudo's stacking context. -->
									<div class="absolute top-12 left-2 right-2 z-20 isolate" role="alert">
										<Alert
											type="error"
											title="Build failed"
											class="relative before:absolute before:inset-0 before:-z-10 before:rounded-md before:bg-surface before:content-['']"
										>
											<pre class="overflow-auto whitespace-pre-wrap text-xs max-h-60"
												>{buildError}</pre
											>
										</Alert>
									</div>
								{:else if runtimeError}
									<div class="absolute top-12 left-2 right-2 z-20 isolate" role="alert">
										<Alert
											type="error"
											title="Runtime error"
											class="relative before:absolute before:inset-0 before:-z-10 before:rounded-md before:bg-surface before:content-['']"
										>
											<pre class="overflow-auto whitespace-pre-wrap text-xs max-h-60"
												>{runtimeError}</pre
											>
										</Alert>
									</div>
								{:else if emptyRender}
									<div class="absolute top-12 left-2 right-2 z-20 isolate" role="alert">
										<Alert
											type="error"
											title="Nothing was mounted"
											class="relative before:absolute before:inset-0 before:-z-10 before:rounded-md before:bg-surface before:content-['']"
										>
											<span class="text-xs">
												The build succeeded but nothing mounted into <code>#root</code>. Add a mount
												call to <code>{mountHint.entrypoint}</code>:
												<code class="block mt-1 whitespace-pre-wrap break-all"
													>{mountHint.call}</code
												>
											</span>
										</Alert>
									</div>
								{/if}
								{#if logs}
									<div
										class="absolute right-0 bottom-0 z-20 max-w-[500px] w-full flex flex-col text-xs p-1 border border-border-light rounded-tl-md bg-surface text-primary {logsCollapsed
											? 'h-6'
											: 'max-h-60 h-full'}"
									>
										<button
											class="cursor-pointer flex items-center gap-2 w-full text-xs font-normal text-secondary -mt-0.5 px-2 text-left"
											onclick={() => (logsCollapsed = !logsCollapsed)}
										>
											Logs
											<ChevronDown
												size={12}
												class="transition duration-200"
												style="transform: {logsCollapsed ? 'rotate(180deg)' : 'rotate(0deg)'}"
											/>
											<span class="text-secondary">({logs.split('\n').length})</span>
										</button>
										<div bind:this={logsDiv} class="logs-scroll grow w-full overflow-auto">
											{#if !logsCollapsed}
												<pre>{logs}</pre>
											{/if}
										</div>
									</div>
								{/if}
							</div>
						</Pane>
					</Splitpanes>
				</div>
			</Pane>
		</Splitpanes>
	</div>
</div>

{#if inlinePromptPos && inlinePromptSelector && onInlinePrompt}
	<!-- Key on the selector so selecting a different element remounts the input
	     (fresh value + autofocus); repositioning on scroll keeps the same key so
	     it never steals focus mid-scroll. -->
	{#key inlinePromptSelector}
		<InlineElementPrompt
			x={inlinePromptPos.x}
			y={inlinePromptPos.y}
			label={inlinePromptLabel}
			onSend={(prompt) => {
				onInlinePrompt?.(inlinePromptSelector!, prompt)
				inlinePromptDismissed = true
			}}
			onClose={() => (inlinePromptDismissed = true)}
		/>
	{/key}
{/if}

<style>
	/* Remove the splitter from the inner content-area Splitpanes when we're
	   not actually in split-with-preview mode (one pane is at 0%). The user
	   uses the explicit Split toggle in the tab bar to flip modes; a
	   visible-but-non-functional drag handle would be confusing. We use
	   `display: none` rather than `width: 0` because svelte-splitpanes' own
	   splitter-width rule otherwise wins and leaves a 1px sliver beside the
	   preview content. */
	:global(.tabs-content-single .splitpanes__splitter) {
		display: none;
	}

	/* Logs overlay scrollbar — small, themed, matching the previous in-iframe
	   panel's look. */
	.logs-scroll::-webkit-scrollbar {
		width: 8px;
		height: 8px;
	}
	.logs-scroll::-webkit-scrollbar-track {
		background: rgb(var(--color-surface-sunken));
	}
	.logs-scroll::-webkit-scrollbar-thumb {
		background: rgb(var(--color-surface-secondary));
		border-radius: 4px;
	}
	.logs-scroll::-webkit-scrollbar-thumb:hover {
		background: rgb(var(--color-border-light));
	}
</style>
