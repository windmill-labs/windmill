<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import RawAppInlineScriptsPanel from './RawAppInlineScriptsPanel.svelte'
	import type { JobById } from '../apps/types'
	import RawAppEditorHeader from './RawAppEditorHeader.svelte'
	import { type Policy, WorkspaceService } from '$lib/gen'
	import DiffDrawer from '../DiffDrawer.svelte'
	import { encodeState } from '$lib/utils'
	import { deepEqual } from 'fast-equals'

	// import { addWmillClient } from './utils'
	import RawAppBackgroundRunner from './RawAppBackgroundRunner.svelte'
	import { workspaceStore } from '$lib/stores'
	import { genWmillTs, type Runnable } from './utils'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import RawAppSidebar from './RawAppSidebar.svelte'
	import type { Modules } from './RawAppModules.svelte'
	import { isRunnableByName, isRunnableByPath } from '../apps/inputType'
	import { aiChatManager, AIMode } from '../copilot/chat/AIChatManager.svelte'
	import { onMount, untrack } from 'svelte'
	import type { LintResult, DataTableSchema, InspectorElementInfo } from '../copilot/chat/app/core'
	import type { AppCodeSelectionElement } from '../copilot/chat/context'
	import { rawAppLintStore } from './lintStore'
	import { dbSchemas } from '$lib/stores'
	import { runScriptAndPollResult } from '../jobs/utils'
	import { RawAppHistoryManager } from './RawAppHistoryManager.svelte'
	import { sendUserToast } from '$lib/utils'
	import {
		parseDataTableRef,
		formatDataTableRef,
		type RawAppData,
		DEFAULT_DATA
	} from './dataTableRefUtils'

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
		savedApp?:
			| {
					value: any
					draft?: any
					path: string
					summary: string
					policy: any
					draft_only?: boolean
					custom_path?: string
			  }
			| undefined
		diffDrawer?: DiffDrawer | undefined
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
		savedApp = $bindable(undefined),
		diffDrawer = undefined
	}: Props = $props()
	export const version: number | undefined = undefined

	// Convert to object format for child components
	let dataTableRefsObjects = $derived(data.tables.map(parseDataTableRef))

	// Initialize history manager
	const historyManager = new RawAppHistoryManager({
		maxEntries: 50,
		autoSnapshotInterval: 5 * 60 * 1000 // 5 minutes
	})
	historyManager.manualSnapshot(files ?? {}, runnables, summary, data)

	let draftTimeout: number | undefined = undefined
	function saveFrontendDraft() {
		draftTimeout && clearTimeout(draftTimeout)
		draftTimeout = setTimeout(() => {
			try {
				localStorage.setItem(
					path != '' ? `rawapp-${path}` : 'rawapp',
					encodeState({
						files,
						runnables: runnables,
						data: data
					})
				)
			} catch (err) {
				console.error(err)
			}
		}, 500)
	}

	let iframe: HTMLIFrameElement | undefined = $state(undefined)

	let sidebarPanelSize = $state(15)

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
	let suppressSetActiveDocument = false

	function populateFiles() {
		if (files) {
			// Suppress iframe's automatic setActiveDocument for a short window
			// after sending files, to prevent it from resetting to App.tsx.
			suppressSetActiveDocument = true
			setTimeout(() => { suppressSetActiveDocument = false }, 500)
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
			saveFrontendDraft()
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
				const baseContext = {
					inspectorElement: inspectorElement,
					selectionExcluded: selectionExcludedFromPrompt,
					toggleSelectionExcluded: toggleSelectionExcluded,
					clearInspector: clearInspectorSelection,
					clearRunnable: handleClearRunnable,
					codeSelection: codeSelection,
					clearCodeSelection: () => {
						codeSelection = undefined
					}
				}
				if (selectedRunnable) {
					const runnable = convertToBackendRunnable(selectedRunnable, runnables[selectedRunnable])
					return {
						type: 'backend' as const,
						backendKey: selectedRunnable,
						backendRunnable: runnable,
						...baseContext
					}
				}
				if (selectedDocument) {
					return {
						type: 'frontend' as const,
						frontendPath: selectedDocument,
						frontendContent: files?.[selectedDocument],
						...baseContext
					}
				}
				return {
					type: 'none' as const,
					...baseContext
				}
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
			getDatatables: async (): Promise<DataTableSchema[]> => {
				if (!$workspaceStore) {
					return []
				}

				// Get all datatable schemas from the backend
				const allSchemas = await WorkspaceService.listDataTableSchemas({
					workspace: $workspaceStore
				})

				// Get unique datatable names from dataTableRefs (the whitelisted tables)
				const whitelistedDatatables = new Set(dataTableRefsObjects.map((ref) => ref.datatable))

				// If no datatables are configured, return all available datatables
				// This allows users to see all datatables in the @ context menu
				if (whitelistedDatatables.size === 0) {
					return allSchemas
				}

				// Build a map of whitelisted tables per datatable: datatable -> schema -> Set<table>
				const whitelistedTables = new Map<string, Map<string, Set<string>>>()
				for (const ref of dataTableRefsObjects) {
					if (!ref.table) continue
					if (!whitelistedTables.has(ref.datatable)) {
						whitelistedTables.set(ref.datatable, new Map())
					}
					const schemaKey = ref.schema || 'public'
					const schemaMap = whitelistedTables.get(ref.datatable)!
					if (!schemaMap.has(schemaKey)) {
						schemaMap.set(schemaKey, new Set())
					}
					schemaMap.get(schemaKey)!.add(ref.table)
				}

				// Filter schemas to only include whitelisted datatables and tables
				const results: DataTableSchema[] = []
				for (const schema of allSchemas) {
					if (!whitelistedDatatables.has(schema.datatable_name)) {
						continue
					}

					const allowedTables = whitelistedTables.get(schema.datatable_name)
					if (!allowedTables) {
						// Include the datatable but with empty schemas
						results.push({
							datatable_name: schema.datatable_name,
							schemas: {},
							error: schema.error
						})
						continue
					}

					// Filter schemas to only include whitelisted tables
					const filteredSchemas: Record<string, Record<string, Record<string, string>>> = {}
					for (const [schemaName, tables] of Object.entries(schema.schemas)) {
						const allowedTablesInSchema = allowedTables.get(schemaName)
						if (!allowedTablesInSchema) continue

						const filteredTables: Record<string, Record<string, string>> = {}
						for (const [tableName, columns] of Object.entries(tables)) {
							if (allowedTablesInSchema.has(tableName)) {
								filteredTables[tableName] = columns
							}
						}

						if (Object.keys(filteredTables).length > 0) {
							filteredSchemas[schemaName] = filteredTables
						}
					}

					results.push({
						datatable_name: schema.datatable_name,
						schemas: filteredSchemas,
						error: schema.error
					})
				}

				return results
			},
			getAvailableDatatableNames: (): string[] => {
				// Get unique datatable names from dataTableRefs
				return [...new Set(dataTableRefsObjects.map((ref) => ref.datatable))]
			},
			execDatatableSql: async (
				datatableName: string,
				sql: string,
				newTable?: { schema: string; name: string }
			): Promise<{ success: boolean; result?: Record<string, any>[]; error?: string }> => {
				if (!$workspaceStore) {
					return { success: false, error: 'Workspace not available' }
				}

				try {
					const result = await runScriptAndPollResult({
						workspace: $workspaceStore,
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
							saveFrontendDraft()
							// Clear the cached schema so it gets refreshed with the new table
							const resourcePath = `datatable://${datatableName}`
							delete $dbSchemas[resourcePath]
						}
					}

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
					saveFrontendDraft()
				}
			}
		})
	})
	let selectedRunnable: string | undefined = $state(undefined)
	let selectedDocument: string | undefined = $state(undefined)
	let inspectorElement: InspectorElementInfo | undefined = $state(undefined)
	let selectionExcludedFromPrompt: boolean = $state(false)
	let codeSelection: AppCodeSelectionElement | undefined = $state(undefined)

	function toggleSelectionExcluded() {
		selectionExcludedFromPrompt = !selectionExcludedFromPrompt
	}

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
		} else if (e.data.type === 'inspectorSelect') {
			// Handle inspector element selection from the iframe preview
			inspectorElement = e.data.element as InspectorElementInfo
		} else if (e.data.type === 'inspectorClear') {
			// Clear the inspector element when user dismisses the selection
			inspectorElement = undefined
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

	let darkMode: boolean = $state(false)
	$effect(() => {
		runnables && files && saveFrontendDraft()
	})
	$effect(() => {
		iframe?.addEventListener('load', () => {
			iframeLoaded = true
		})
	})
	$effect(() => {
		iframe && iframeLoaded && files && populateFiles()
	})
	$effect(() => {
		iframe && iframeLoaded && runnables && populateRunnables()
	})

	function clearInspectorSelection() {
		inspectorElement = undefined
		iframe?.contentWindow?.postMessage({ type: 'inspectorClear' }, '*')
	}

	function handleSelectFile(path: string) {
		console.log('event Select file:', path)
		selectedRunnable = undefined
		// Inspector is cleared by the $effect watching selection changes
		iframe?.contentWindow?.postMessage(
			{
				type: 'selectFile',
				path: path
			},
			'*'
		)
	}

	function handleClearRunnable() {
		selectedRunnable = undefined
	}

	// Track previous values for change detection
	let prevSelectedRunnable: string | undefined = undefined
	let prevSelectedDocument: string | undefined = undefined

	// Clear inspector and reset exclusion when selection changes
	$effect(() => {
		if (selectedRunnable !== prevSelectedRunnable || selectedDocument !== prevSelectedDocument) {
			// Only clear if we're actually switching to something different
			if (prevSelectedRunnable !== undefined || prevSelectedDocument !== undefined) {
				clearInspectorSelection()
			}
			// Reset exclusion when switching files/runnables
			selectionExcludedFromPrompt = false
			prevSelectedRunnable = selectedRunnable
			prevSelectedDocument = selectedDocument
		}
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

	function handleKeydown(e: KeyboardEvent) {
		// Ctrl/Cmd + Shift + H for manual snapshot
		if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'H') {
			e.preventDefault()
			historyManager.manualSnapshot(files ?? {}, runnables, summary, data)
		}
	}
</script>

<svelte:window onmessage={listener} onkeydown={handleKeydown} />
<DarkModeObserver bind:darkMode />

<RawAppBackgroundRunner
	workspace={$workspaceStore ?? ''}
	editor
	{iframe}
	bind:jobs
	bind:jobsById
	{runnables}
	{path}
/>
<div class="max-h-screen overflow-hidden h-screen min-h-0 flex flex-col">
	<RawAppEditorHeader
		bind:jobs
		bind:jobsById
		bind:savedApp
		bind:summary
		on:restore
		on:savedNewAppPath
		{policy}
		{diffDrawer}
		{newApp}
		{newPath}
		appPath={path}
		{files}
		{data}
		{runnables}
		{getBundle}
		canUndo={historyManager.canUndo}
		canRedo={historyManager.canRedo}
		onUndo={handleUndo}
		onRedo={handleRedo}
	/>

	<Splitpanes id="o2" class="grow min-h-0">
		<Pane bind:size={sidebarPanelSize} maxSize={20} class="h-full overflow-y-auto">
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
					saveFrontendDraft()
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
					saveFrontendDraft()
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
		<Pane>
			<div class="h-full w-full">
				<iframe
					bind:this={iframe}
					title="UI builder"
					style="display: {selectedRunnable == undefined ? 'block' : 'none'}"
					src="/ui_builder/index.html?dark={darkMode}"
					class="w-full h-full"
				></iframe>
				{#if selectedRunnable !== undefined}
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div class="flex h-full w-full">
						<RawAppInlineScriptsPanel
							appPath={path}
							{selectedRunnable}
							bind:runnables
							onSelectionChange={(selection) => {
								console.log('handle selection', selection)

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
					</div>
				{/if}
			</div>

			<!-- <div class="bg-red-400 h-full w-full" /> -->
		</Pane>
	</Splitpanes>
</div>
