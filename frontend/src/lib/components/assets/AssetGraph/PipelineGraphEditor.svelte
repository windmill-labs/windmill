<script lang="ts">
	import { onMount, untrack, type Snippet } from 'svelte'
	import { Loader2 } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { DraftService } from '$lib/gen'
	import { decodeState, encodeState } from '$lib/utils'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { extractWrites } from '$lib/components/assets/lib'
	import type { PipelineDraft } from './pipelineAiHelpers'
	import type { ColumnLineageGraph } from './columnLineageGraph'
	import HideButton from '$lib/components/apps/editor/settingsPanel/HideButton.svelte'
	import AssetGraphCanvas from './AssetGraphCanvas.svelte'
	import AssetGraphDetailsPane from './AssetGraphDetailsPane.svelte'
	import PipelineEventLog from './PipelineEventLog.svelte'
	import GlobalReviewButtons from '$lib/components/copilot/chat/GlobalReviewButtons.svelte'
	import type {
		AssetGraphResponse,
		AssetGraphSelection,
		NativeTriggerKind,
		PipelineMode
	} from './types'
	import type { AssetKind, ScriptLang } from '$lib/gen'
	import type { RunnableRunState, PipelineEvent } from './activeRunnables.svelte'
	import type { PipelineOutputKind } from './pipelineTemplates'
	import type { PipelineEditorState } from './pipelineEditorState.svelte'

	type RunProducer = { kind: 'script' | 'flow'; path: string; unsaved?: boolean; cascade?: boolean }

	let {
		editor,
		displayGraph,
		mode,
		workspace,
		folder,
		persistDrafts = false,
		pathPrefix,
		defaultPathSuffix = 'new_pipeline_script',
		panelHidden = false,
		onTogglePanelHidden,
		prefetchingAssets = false,
		hasAiPending = false,
		onAcceptAllProposals,
		onRejectAllProposals,
		hoveredPaths = [],
		selectedRunPaths = [],
		activeRunnable,
		activeRunnableIds,
		runStates,
		eventLogEvents = [],
		runsRefreshKey = 0,
		runsPendingJobId,
		boundPick,
		validStartPaths,
		onStartBoundedRun,
		onPickEnd,
		panToNodeId,
		boundBar,
		onCreateMissingTrigger,
		onEditTrigger,
		onDeleteTrigger,
		onOpenWebhook,
		onOpenDataUpload,
		onSelect,
		onAddScriptForAsset,
		onAddPipelineScript,
		onRunnableMenuRemove,
		onRunProducer,
		idlePane,
		onRequestEdit,
		canRunByPath = false,
		onRunByPath,
		selectionProducers = [],
		selectionColumnGraph,
		schemaCanEvolve = true,
		downstreamSubscribers = 0,
		onStartBoundedRunForOpen,
		canBoundedRunOpenScript = false,
		onRunCompleted,
		onTestStateChange,
		requestRemoveSignal = 0,
		requestRunSignal = 0,
		requestRunCascadeSignal = 0,
		focusUploadSignal = 0,
		onDraftPathChange,
		onClose = () => {},
		onDiscard,
		onDraftSaved,
		onPersistedSaved,
		onScriptRenamed,
		onScriptRemoved
	}: {
		editor: PipelineEditorState
		displayGraph: AssetGraphResponse
		mode: PipelineMode
		workspace: string | undefined
		/** Folder the pipeline is scoped to — drives the autosave draft path. */
		folder: string
		/** Persist drafts as a `data_pipeline` DraftService bundle (route page only;
		 * the session preview leaves this false). FlowBuilder's autosave analogue. */
		persistDrafts?: boolean
		pathPrefix: string
		defaultPathSuffix?: string
		panelHidden?: boolean
		onTogglePanelHidden?: () => void
		prefetchingAssets?: boolean
		hasAiPending?: boolean
		onAcceptAllProposals?: () => void
		onRejectAllProposals?: () => void
		hoveredPaths?: string[]
		selectedRunPaths?: string[]
		activeRunnable?: { kind: 'script' | 'flow'; path: string } | undefined
		activeRunnableIds?: ReadonlySet<string>
		runStates?: ReadonlyMap<string, RunnableRunState>
		eventLogEvents?: PipelineEvent[]
		runsRefreshKey?: any
		runsPendingJobId?: string | undefined
		boundPick?: any
		validStartPaths?: Set<string>
		onStartBoundedRun?: (path: string) => void
		onPickEnd?: (id: string) => void
		panToNodeId?: string | undefined
		boundBar?: Snippet
		onCreateMissingTrigger?: (kind: NativeTriggerKind, scriptPath: string) => void
		onEditTrigger?: (...args: any[]) => void
		onDeleteTrigger?: (...args: any[]) => void
		onOpenWebhook?: (...args: any[]) => void
		onOpenDataUpload?: (...args: any[]) => void
		onSelect: (selection: AssetGraphSelection | undefined) => void
		onAddScriptForAsset?: (
			asset: { kind: AssetKind; path: string },
			language: ScriptLang,
			scriptPath: string,
			outputKind: PipelineOutputKind,
			aiPrompt?: string
		) => void
		onAddPipelineScript?: (
			language: ScriptLang,
			path: string,
			source: { kind: NativeTriggerKind; path: string | undefined },
			outputKind: PipelineOutputKind,
			aiPrompt?: string
		) => void
		onRunnableMenuRemove?: (...args: any[]) => void
		onRunProducer?: (producer: RunProducer) => Promise<string | undefined>
		idlePane?: Snippet
		onRequestEdit?: () => void
		canRunByPath?: boolean
		onRunByPath?: (path: string, args: Record<string, any>) => Promise<string | undefined>
		selectionProducers?: Array<{ kind: 'script' | 'flow'; path: string; unsaved?: boolean }>
		/** Transitive column-lineage trace for a selected ducklake asset (route page). */
		selectionColumnGraph?: ColumnLineageGraph
		schemaCanEvolve?: boolean
		downstreamSubscribers?: number
		onStartBoundedRunForOpen?: (path: string) => void
		canBoundedRunOpenScript?: boolean
		onRunCompleted?: () => void
		onTestStateChange?: (running: boolean) => void
		requestRemoveSignal?: number
		requestRunSignal?: number
		requestRunCascadeSignal?: number
		focusUploadSignal?: number
		onDraftPathChange?: (oldPath: string, newPath: string) => boolean | string
		onClose?: () => void
		onDiscard?: () => void
		onDraftSaved?: (savedPath: string) => void
		onPersistedSaved?: (savedPath: string) => void
		onScriptRenamed?: (oldPath: string, newPath: string) => void
		onScriptRemoved?: (removedPath: string) => void
	} = $props()

	let leftPaneSize = $state(60)
	let rightPaneSize = $state(40)
	let storedRightPaneSize = $state(40)

	let effectiveSelection = $derived<AssetGraphSelection | undefined>(
		editor.activeDraftPath
			? { kind: 'runnable', runnable_kind: 'script', path: editor.activeDraftPath }
			: editor.selection
	)
	let activeDraft = $derived(editor.activeDraft)
	// Edit mode opens the right pane only for a selection/draft; view mode keeps it
	// open permanently (it shows the activity idlePane when nothing is selected,
	// and swaps to the details pane on node select). Matches the route page's
	// original behaviour — NOT "always open in edit mode".
	let detailsPaneOpen = $derived(
		mode === 'edit'
			? (editor.selection != undefined || editor.activeDraftPath != undefined) && !panelHidden
			: !panelHidden
	)
	let idleView = $derived(
		mode !== 'edit' && editor.selection == undefined && editor.activeDraftPath == undefined
	)

	// Wrap the writes (and the rightPaneSize read in the else branch) in untrack so
	// the effect tracks only `detailsPaneOpen` / `storedRightPaneSize` — without it,
	// the Pane `bind:size` feedback loops the effect and pegs the main thread.
	$effect(() => {
		if (detailsPaneOpen) {
			const restore = storedRightPaneSize
			untrack(() => {
				rightPaneSize = restore > 0 ? restore : 40
				leftPaneSize = 100 - (restore > 0 ? restore : 40)
			})
		} else {
			untrack(() => {
				if (rightPaneSize > 0) storedRightPaneSize = rightPaneSize
				leftPaneSize = 100
			})
		}
	})

	// ===================== Draft autosave (persistDrafts only) =====================
	// All of this folder's in-flight drafts live in ONE per-user DB draft (typ
	// `data_pipeline`) keyed at the folder, syncing across devices + surfacing in
	// the global drafts list. localStorage is a synchronous crash mirror, READ only
	// for the one-time migration below; the DB is the source of truth on load.
	// FlowBuilder's autosave analogue — gated by `persistDrafts` so the in-session
	// preview (which doesn't persist) opts out.
	const PIPELINE_DRAFT_KIND = 'data_pipeline' as const
	let pipelineDraftPath = $derived(`f/${folder}/data_pipeline`)
	let storageKey = $derived(`pipeline-${folder}`)
	type PipelineDraftBundle = { drafts: Array<[string, PipelineDraft]>; activeDraftPath?: string }
	let draftsHydrated = $state(false)
	let lastPersistedBundle: string | undefined = undefined

	function restoreBundle(bundle: PipelineDraftBundle) {
		if (Array.isArray(bundle.drafts)) {
			const loaded = new Map<string, PipelineDraft>()
			for (const entry of bundle.drafts) {
				if (entry && typeof entry[0] === 'string' && entry[1]?.script) {
					const d = entry[1] as PipelineDraft
					if (typeof d.localId !== 'string' || d.localId === '')
						d.localId = editor.newDraftLocalId()
					loaded.set(entry[0], d)
				}
			}
			if (loaded.size > 0) editor.drafts = loaded
		}
		if (typeof bundle.activeDraftPath === 'string') editor.activeDraftPath = bundle.activeDraftPath
	}

	function readLocalBundle(): PipelineDraftBundle | undefined {
		if (typeof localStorage === 'undefined') return undefined
		const raw = localStorage.getItem(`pipeline-${folder}`)
		if (!raw) return undefined
		try {
			const s = decodeState(raw)
			if (s && (Array.isArray(s.drafts) || typeof s.activeDraftPath === 'string')) {
				return {
					drafts: Array.isArray(s.drafts) ? s.drafts : [],
					activeDraftPath: typeof s.activeDraftPath === 'string' ? s.activeDraftPath : undefined
				}
			}
		} catch (e) {
			console.warn('failed to read local pipeline state', e)
		}
		return undefined
	}

	async function hydrateDrafts() {
		const ws = workspace
		const path = pipelineDraftPath
		try {
			let bundle: PipelineDraftBundle | undefined
			let serverSavedAt: string | undefined
			if (ws) {
				const row = await DraftService.getOwnDraft({
					workspace: ws,
					kind: PIPELINE_DRAFT_KIND,
					path
				})
				if (row?.value) {
					bundle = row.value as PipelineDraftBundle
					serverSavedAt = row.created_at
					editor.loadedFromDbDraft = true
				}
			}
			let migratedFromLocal = false
			if (!bundle) {
				const local = readLocalBundle()
				if (local) {
					bundle = local
					migratedFromLocal = true
				}
			}
			if (bundle) restoreBundle(bundle)
			UserDraftDbSyncer.recordRemoteSync(
				{ workspace: ws ?? '', itemKind: PIPELINE_DRAFT_KIND, path },
				migratedFromLocal ? undefined : serverSavedAt
			)
			if (!migratedFromLocal) lastPersistedBundle = bundle ? JSON.stringify(bundle) : undefined
		} catch (e) {
			console.warn('failed to load pipeline drafts', e)
		} finally {
			draftsHydrated = true
		}
	}

	onMount(() => {
		if (persistDrafts) void hydrateDrafts()
	})

	$effect(() => {
		if (!persistDrafts) return
		// For the active draft, snapshot the latest live body writes + editor buffer
		// at serialize time so edits since the last pane-transition aren't lost on reload.
		const liveWritesSnapshot =
			editor.liveBodyAssets.scriptPath != undefined &&
			editor.drafts.has(editor.liveBodyAssets.scriptPath)
				? extractWrites(editor.liveBodyAssets.assets)
				: undefined
		const liveWritesPath = editor.liveBodyAssets.scriptPath
		const liveContentPath =
			editor.liveContent.scriptPath != undefined && editor.drafts.has(editor.liveContent.scriptPath)
				? editor.liveContent.scriptPath
				: undefined
		const liveContentValue = editor.liveContent.content
		const serialized = Array.from(editor.drafts.entries()).map(([p, d]) => {
			const outputAssets =
				liveWritesSnapshot != undefined && liveWritesPath === p
					? liveWritesSnapshot.length > 0
						? liveWritesSnapshot
						: undefined
					: d.outputAssets
			const script =
				liveContentPath === p && d.script.content !== liveContentValue
					? { ...d.script, content: liveContentValue }
					: d.script
			if (script === d.script && outputAssets === d.outputAssets)
				return [p, d] as [string, PipelineDraft]
			return [p, { ...d, script, outputAssets }] as [string, PipelineDraft]
		})
		const activePath = editor.activeDraftPath
		const key = storageKey
		const ws = workspace
		const path = pipelineDraftPath
		const hydrated = draftsHydrated
		untrack(() => {
			if (!hydrated) return
			const isEmpty = serialized.length === 0 && !activePath
			const bundle: PipelineDraftBundle | undefined = isEmpty
				? undefined
				: { drafts: serialized, activeDraftPath: activePath }
			try {
				if (typeof localStorage !== 'undefined') {
					if (isEmpty) localStorage.removeItem(key)
					else
						localStorage.setItem(
							key,
							encodeState({ drafts: serialized, activeDraftPath: activePath })
						)
				}
			} catch (e) {
				console.warn('failed to mirror pipeline state', e)
			}
			const serializedBundle = bundle ? JSON.stringify(bundle) : undefined
			if (serializedBundle === lastPersistedBundle) return
			lastPersistedBundle = serializedBundle
			if (!ws) return
			void UserDraftDbSyncer.save({
				workspace: ws,
				itemKind: PIPELINE_DRAFT_KIND,
				path,
				value: bundle ?? null,
				auto: true
			})
		})
	})
</script>

<Splitpanes class="!h-full">
	<Pane bind:size={leftPaneSize}>
		<div class="relative h-full">
			<AssetGraphCanvas
				graph={displayGraph}
				selection={effectiveSelection}
				{hoveredPaths}
				{selectedRunPaths}
				{activeRunnable}
				{activeRunnableIds}
				{runStates}
				{pathPrefix}
				{defaultPathSuffix}
				{onCreateMissingTrigger}
				{onEditTrigger}
				{onDeleteTrigger}
				{onOpenWebhook}
				{onOpenDataUpload}
				onselect={onSelect}
				{onAddScriptForAsset}
				{onAddPipelineScript}
				{onRunnableMenuRemove}
				{onRunProducer}
				{validStartPaths}
				{onStartBoundedRun}
				{boundPick}
				{onPickEnd}
				{panToNodeId}
			/>
			{#if boundBar}{@render boundBar()}{/if}
			{#if hasAiPending && onAcceptAllProposals && onRejectAllProposals}
				<GlobalReviewButtons
					onAcceptAll={onAcceptAllProposals}
					onRejectAll={onRejectAllProposals}
				/>
			{/if}
			{#if mode === 'edit'}
				<PipelineEventLog events={eventLogEvents} />
			{/if}
			{#if prefetchingAssets}
				<div
					class="absolute top-2 left-1/2 -translate-x-1/2 z-20 flex items-center gap-1.5 px-2 py-1 rounded-md bg-surface/95 backdrop-blur-sm border border-gray-200 dark:border-gray-700 text-2xs text-secondary shadow-sm"
					title="Inferring assets for every script in this folder so the graph is complete"
				>
					<Loader2 size={11} class="animate-spin" />
					Parsing assets…
				</div>
			{/if}
			{#if onTogglePanelHidden && (mode !== 'edit' || editor.selection != undefined || editor.activeDraftPath != undefined)}
				<div
					class="absolute top-2 right-2 z-50 rounded-md bg-surface shadow-sm border border-gray-200 dark:border-gray-700"
				>
					<HideButton hidden={panelHidden} direction="right" on:click={onTogglePanelHidden} />
				</div>
			{/if}
		</div></Pane
	>
	{#if detailsPaneOpen && workspace}
		<Pane bind:size={rightPaneSize} minSize={25}>
			{#if idleView && idlePane}
				{@render idlePane()}
			{:else}
				<AssetGraphDetailsPane
					{mode}
					{onRequestEdit}
					{canRunByPath}
					{onRunByPath}
					selection={activeDraft ? undefined : editor.selection}
					selectionProducers={activeDraft ? [] : selectionProducers}
					{selectionColumnGraph}
					{schemaCanEvolve}
					{runsRefreshKey}
					{runsPendingJobId}
					{activeRunnable}
					{downstreamSubscribers}
					onStartBoundedRun={canBoundedRunOpenScript &&
					editor.openScriptPath &&
					onStartBoundedRunForOpen
						? () => onStartBoundedRunForOpen?.(editor.openScriptPath!)
						: undefined}
					{onRunCompleted}
					{onTestStateChange}
					{requestRemoveSignal}
					{requestRunSignal}
					{requestRunCascadeSignal}
					{focusUploadSignal}
					draftScript={activeDraft?.script}
					{pathPrefix}
					{onDraftPathChange}
					{workspace}
					onAnnotationsChange={editor.handleAnnotationsChange}
					onAssetsChange={editor.handleAssetsChange}
					onContentChange={editor.handleContentChange}
					onDraftPersist={editor.handleDraftPersist}
					onclose={onClose}
					onHide={onTogglePanelHidden}
					{onDiscard}
					{onDraftSaved}
					{onPersistedSaved}
					{onScriptRenamed}
					{onScriptRemoved}
				/>
			{/if}
		</Pane>
	{/if}
</Splitpanes>
