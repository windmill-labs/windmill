<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import RawAppInlineScriptsPanel from './RawAppInlineScriptsPanel.svelte'
	import type { JobById } from '../apps/types'
	import RawAppEditorHeader from './RawAppEditorHeader.svelte'
	import { type Policy } from '$lib/gen'
	import DiffDrawer from '../DiffDrawer.svelte'
	import { encodeState } from '$lib/utils'

	// import { addWmillClient } from './utils'
	import RawAppBackgroundRunner from './RawAppBackgroundRunner.svelte'
	import { workspaceStore } from '$lib/stores'
	import { genWmillTs, type Runnable } from './utils'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import RawAppSidebar from './RawAppSidebar.svelte'
	import type { Modules } from './RawAppModules.svelte'
	import { isRunnableByName, isRunnableByPath } from '../apps/inputType'
	import { aiChatManager, AIMode } from '../copilot/chat/AIChatManager.svelte'
	import { onMount } from 'svelte'
	import type { LintResult } from '../copilot/chat/app/core'
	import { rawAppLintStore } from './lintStore'
	import { RawAppHistoryManager } from './RawAppHistoryManager.svelte'
	import RawAppHistorySidebar from './RawAppHistorySidebar.svelte'
	import { sendUserToast, displayDate } from '$lib/utils'
	import Button from '../common/button/Button.svelte'
	import { Eye, X, RotateCcw } from 'lucide-svelte'

	interface Props {
		initFiles: Record<string, string>
		initRunnables: Record<string, Runnable>
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
		initFiles,
		initRunnables,
		newApp,
		policy,
		summary = $bindable(''),
		path,
		newPath = undefined,
		savedApp = $bindable(undefined),
		diffDrawer = undefined
	}: Props = $props()
	export const version: number | undefined = undefined

	let runnables = $state(initRunnables)

	let initRunnablesContent = Object.fromEntries(
		Object.entries(initRunnables).map(([key, runnable]) => {
			if (isRunnableByName(runnable)) {
				return [key, runnable?.inlineScript?.content ?? '']
			}
			return [key, '']
		})
	)

	let files: Record<string, string> | undefined = $state(initFiles)

	// Initialize history manager
	const historyManager = new RawAppHistoryManager({
		maxEntries: 50,
		autoSnapshotInterval: 5 * 60 * 1000 // 5 minutes
	})
	historyManager.addSnapshot()
	let historyPaneOpen = $state(false)
	let historyPreviewId = $state<number | undefined>(undefined)

	let draftTimeout: number | undefined = undefined
	function saveFrontendDraft() {
		draftTimeout && clearTimeout(draftTimeout)
		draftTimeout = setTimeout(() => {
			try {
				localStorage.setItem(
					path != '' ? `rawapp-${path}` : 'rawapp',
					encodeState({
						files,
						runnables: runnables
					})
				)
			} catch (err) {
				console.error(err)
			}
		}, 500)
	}

	let iframe: HTMLIFrameElement | undefined = $state(undefined)

	let sidebarPanelSize = $state(10)

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

	function populateFiles() {
		setFilesInIframe(initFiles)
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

		// Start auto-snapshot
		historyManager.startAutoSnapshot(() => ({
			files: files ?? {},
			runnables,
			summary
		}))

		return () => {
			rawAppLintStore.disable()
			historyManager.destroy()
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
				console.log('result', frontendFiles)
				return frontendFiles
			},
			setFrontendFile: (path, content): LintResult => {
				console.log('setting frontend file', path, content)
				if (!files) {
					files = {}
				}
				files[path] = content
				setFilesInIframe(files)
				selectedDocument = path
				handleSelectFile(path)
				console.log('files after setting', files)
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
				if (selectedRunnable) {
					console.log('selectedRunnable', selectedRunnable)
					return {
						type: 'backend',
						content: selectedRunnable ?? ''
					}
				}
				if (selectedDocument) {
					console.log('selectedDocument', selectedDocument)
					return {
						type: 'frontend',
						content: selectedDocument ?? ''
					}
				}
				console.log('no selection')
				return {
					type: 'none',
					content: ''
				}
			},
			snapshot: () => {
				return (
					historyManager.manualSnapshot(files ?? {}, runnables, summary)?.id ??
					historyManager.getId()
				)
			},
			revertToSnapshot: (id: number) => {
				console.log('reverting to snapshot', id)
				handlePreviewSelect(id)
				historyPaneOpen = true
			}
		})
	})
	let selectedRunnable: string | undefined = $state(undefined)
	let selectedDocument: string | undefined = $state(undefined)

	let modules = $state({}) as Modules
	function listener(e: MessageEvent) {
		if (e.data.type === 'setFiles') {
			files = e.data.files
		} else if (e.data.type === 'getBundle') {
			getBundleResolve?.(e.data.bundle)
		} else if (e.data.type === 'updateModules') {
			modules = e.data.modules
		} else if (e.data.type === 'setActiveDocument') {
			selectedDocument = e.data.path
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
		iframe && iframeLoaded && initFiles && populateFiles()
	})
	$effect(() => {
		iframe && iframeLoaded && runnables && populateRunnables()
	})

	function handleSelectFile(path: string) {
		console.log('event Select file:', path)
		selectedRunnable = undefined
		iframe?.contentWindow?.postMessage(
			{
				type: 'selectFile',
				path: path
			},
			'*'
		)
	}

	function handleRestoreSnapshot(id: number) {
		const entry = historyManager.getEntryById(id)
		if (!entry) {
			sendUserToast('Failed to restore snapshot: entry not found', true)
			return
		}

		// Validate snapshot integrity
		if (!entry.files || typeof entry.files !== 'object') {
			sendUserToast('Failed to restore snapshot: invalid files data', true)
			return
		}
		if (!entry.runnables || typeof entry.runnables !== 'object') {
			sendUserToast('Failed to restore snapshot: invalid runnables data', true)
			return
		}

		try {
			// Auto-save current state before restoring
			historyManager.manualSnapshot(files ?? {}, runnables, summary)

			// Restore from snapshot
			files = entry.files
			runnables = entry.runnables
			summary = entry.summary

			setFilesInIframe(files)
			populateRunnables()

			// Clear preview mode after restore
			historyPreviewId = undefined
			historyManager.clearPreview()

			sendUserToast('Snapshot restored successfully')
		} catch (error) {
			console.error('Failed to restore snapshot:', error)
			sendUserToast('Failed to restore snapshot: ' + (error as Error).message, true)
		}
	}

	function handleUndo() {
		const entry = historyManager.undo()
		if (entry) {
			try {
				files = entry.files
				runnables = entry.runnables
				summary = entry.summary

				setFilesInIframe(entry.files)
				populateRunnables()

				sendUserToast('Undo successful')
			} catch (error) {
				console.error('Failed to undo:', error)
				sendUserToast('Failed to undo: ' + (error as Error).message, true)
			}
		}
	}

	function handleRedo() {
		const entry = historyManager.redo()
		if (entry) {
			try {
				files = entry.files
				runnables = entry.runnables
				summary = entry.summary

				setFilesInIframe(entry.files)
				populateRunnables()

				sendUserToast('Redo successful')
			} catch (error) {
				console.error('Failed to redo:', error)
				sendUserToast('Failed to redo: ' + (error as Error).message, true)
			}
		}
	}

	function handlePreviewSelect(id: number) {
		const entry = historyManager.getEntryById(id)
		if (entry) {
			historyPreviewId = entry.id
			historyManager.setPreview(id)

			// Apply preview state to editor
			files = entry.files
			runnables = entry.runnables
			summary = entry.summary

			setFilesInIframe(entry.files)
			populateRunnables()
		}
	}

	function handleClearPreview() {
		historyPreviewId = undefined
		historyManager.clearPreview()

		// Restore to latest state (last entry in history)
		const latestEntry = historyManager.getEntry(historyManager.entryCount - 1)
		if (latestEntry) {
			files = latestEntry.files
			runnables = latestEntry.runnables
			summary = latestEntry.summary

			setFilesInIframe(latestEntry.files)
			populateRunnables()
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		// Ctrl/Cmd + Shift + H for manual snapshot
		if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'H') {
			e.preventDefault()
			historyManager.manualSnapshot(files ?? {}, runnables, summary)
			sendUserToast('Snapshot created')
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
		{runnables}
		{getBundle}
		bind:historyPaneOpen
		hasHistoryEntries={historyManager.hasEntries}
		canUndo={historyManager.canUndo}
		canRedo={historyManager.canRedo}
		onUndo={handleUndo}
		onRedo={handleRedo}
	/>

	<Splitpanes id="o2" class="grow">
		{#if historyPaneOpen}
			<Pane size={20} minSize={15} maxSize={30}>
				<RawAppHistorySidebar
					{historyManager}
					selectedId={historyPreviewId}
					onSelect={handlePreviewSelect}
					onManualSnapshot={() => {
						historyManager.manualSnapshot(files ?? {}, runnables, summary)
						sendUserToast('Snapshot created')
					}}
				/>
			</Pane>
		{/if}
		<Pane bind:size={sidebarPanelSize} maxSize={20}>
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
				{runnables}
				{modules}
			></RawAppSidebar>
		</Pane>
		<Pane>
			<!-- Preview Mode Banner -->
			{#if historyManager.isPreviewMode && historyPreviewId !== undefined}
				{@const previewEntry = historyManager.getEntryById(historyPreviewId)}
				{#if previewEntry}
					<div
						class="bg-amber-50 dark:bg-amber-900/20 border-b border-amber-200 dark:border-amber-700 px-4 py-3"
					>
						<div class="flex items-center justify-between gap-4">
							<div class="flex items-center gap-2 text-amber-800 dark:text-amber-300">
								<Eye size={16} class="flex-shrink-0" />
								<div class="flex flex-col gap-0.5">
									<span class="font-medium text-sm">Preview Mode - Read Only</span>
									<span class="text-xs opacity-80">
										Viewing snapshot from {displayDate(previewEntry.timestamp.toISOString())}
									</span>
								</div>
							</div>
							<div class="flex gap-2">
								<Button
									size="xs"
									color="light"
									startIcon={{ icon: X }}
									on:click={handleClearPreview}
								>
									Exit Preview
								</Button>
								<Button
									size="xs"
									color="dark"
									startIcon={{ icon: RotateCcw }}
									on:click={() =>
										historyPreviewId !== undefined && handleRestoreSnapshot(historyPreviewId)}
								>
									Restore This Version
								</Button>
							</div>
						</div>
					</div>
				{/if}
			{/if}

			<div
				class="h-full w-full"
				style="height: {historyManager.isPreviewMode ? 'calc(100% - 60px)' : '100%'}"
			>
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
							{initRunnablesContent}
							{runnables}
						/>
					</div>
				{/if}
			</div>

			<!-- <div class="bg-red-400 h-full w-full" /> -->
		</Pane>
	</Splitpanes>
</div>
