<script lang="ts">
	import { untrack, type Snippet } from 'svelte'
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
	import type {
		AssetGraphResponse,
		AssetGraphSelection,
		NativeTriggerKind,
		PipelineMode
	} from './types'
	import type { AssetKind, Script, ScriptLang } from '$lib/gen'
	import type { RunnableRunState, PipelineEvent } from './activeRunnables.svelte'
	import type { PipelineOutputKind } from './pipelineTemplates'
	import type { PipelineEditorState } from './pipelineEditorState.svelte'
	import type { SchemaContractGraphContext } from './schemaContracts'

	type RunProducer = { kind: 'script' | 'flow'; path: string; unsaved?: boolean; cascade?: boolean }

	let {
		editor,
		displayGraph,
		mode,
		workspace,
		folder,
		viewportFitKey = undefined,
		stackBelow = 680,
		persistDrafts = false,
		pathPrefix,
		defaultPathSuffix = 'new_pipeline_script',
		panelHidden = false,
		onTogglePanelHidden,
		prefetchingAssets = false,
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
		onRunCascadeByPath,
		runFormInitialArgs,
		onRunFormArgsChange,
		readyDataUploadPaths,
		resolveLocalScript,
		localScriptsVersion,
		selectionProducers = [],
		selectionColumnGraph,
		schemaCanEvolve = true,
		selectionForkMaterialization = undefined,
		schemaContractContext = undefined,
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
		/** Folder whose graph is actually *loaded* (not the route param). On an
		 * in-place folder switch the stale graph stays rendered while the new
		 * fetch is in flight, so keying the canvas's one-shot initial fit on
		 * `folder` would consume the new folder's fit on the old graph. Pages
		 * that stale-while-revalidate should pass the folder captured when the
		 * fetch resolved; defaults to `folder`. */
		viewportFitKey?: string
		/** Below this container width (px) the graph/details split stacks
		 * vertically instead of side-by-side — for narrow side panels / AI
		 * session previews. Defaults to 680. */
		stackBelow?: number
		/** Persist drafts as a per-folder `data_pipeline` DraftService bundle (enabled
		 * by both the route page and the in-session preview). FlowBuilder's autosave
		 * analogue. */
		persistDrafts?: boolean
		pathPrefix: string
		defaultPathSuffix?: string
		panelHidden?: boolean
		onTogglePanelHidden?: () => void
		prefetchingAssets?: boolean
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
		// Run the open script AND its downstream closure (dev preview only; the
		// deployed pane leaves this unset — its single run cascades via the backend).
		onRunCascadeByPath?: (path: string, args: Record<string, any>) => Promise<string | undefined>
		// Persisted run-form args for the currently-open data-upload entry, and a
		// callback to persist them as they change — see the page's dataUploadArgs.
		runFormInitialArgs?: Record<string, any>
		onRunFormArgsChange?: (path: string, args: Record<string, any>, isValid: boolean) => void
		// Data-upload entry scripts whose staged upload is ready (green node).
		readyDataUploadPaths?: Set<string>
		/** Local-dev (`/pipeline_dev`): resolve a node to its working-tree content
		 * so the details pane skips the (nonexistent) deployed-script fetch. May
		 * be async (to infer the args schema for the run form). */
		resolveLocalScript?: (path: string) => Script | undefined | Promise<Script | undefined>
		/** Bumped on each `pipeline dev` bundle so the open details pane re-resolves
		 * the selected node's source on live-reload. */
		localScriptsVersion?: unknown
		selectionProducers?: Array<{ kind: 'script' | 'flow'; path: string; unsaved?: boolean }>
		/** Transitive column-lineage trace for a selected ducklake asset (route page). */
		selectionColumnGraph?: ColumnLineageGraph
		schemaCanEvolve?: boolean
		/** Fork workspaces: data-environment state of the selected ducklake asset (route page). */
		selectionForkMaterialization?: 'fork' | 'deferred'
		/** Producer-side facts for the editor's live schema-contract diagnostics
		 * (ignore suppression + scd2 `_current` fallback), from the route page. */
		schemaContractContext?: SchemaContractGraphContext
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
	// 0 = "no user-chosen size yet" so the orientation-aware default (55% stacked /
	// 40% side-by-side) applies on first open; a real drag overwrites it below.
	let storedRightPaneSize = $state(0)

	// Container width drives the split orientation: stack vertically (graph over
	// details) when too narrow for a comfortable side-by-side split.
	let containerWidth = $state(0)
	let stacked = $derived(containerWidth > 0 && containerWidth < stackBelow)

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
		// Stacked (vertical) splits give the details pane more room — the run
		// form + result need height more than the graph needs it.
		const fallback = stacked ? 55 : 40
		if (detailsPaneOpen) {
			const restore = storedRightPaneSize
			untrack(() => {
				rightPaneSize = restore > 0 ? restore : fallback
				leftPaneSize = 100 - (restore > 0 ? restore : fallback)
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
	// FlowBuilder's autosave analogue — gated by `persistDrafts`.
	const PIPELINE_DRAFT_KIND = 'data_pipeline' as const
	let pipelineDraftPath = $derived(`f/${folder}/data_pipeline`)
	let storageKey = $derived(`pipeline-${folder}`)
	type PipelineDraftBundle = { drafts: Array<[string, PipelineDraft]>; activeDraftPath?: string }
	// Hydration is tracked on the editor instance (`editor.hydratedFromDb`), not a
	// component flag: the in-session preview reuses one instance across editor
	// hide/show, and it must hydrate ONCE per instance, not on every remount.
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
		const raw = localStorage.getItem(storageKey)
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
			let fromServer = false
			if (ws) {
				const row = await DraftService.getOwnDraft({
					workspace: ws,
					kind: PIPELINE_DRAFT_KIND,
					path
				})
				if (row?.value) {
					bundle = row.value as PipelineDraftBundle
					serverSavedAt = row.created_at
					fromServer = true
				}
			}
			// A folder retarget during the await (the route-page header switcher, or a
			// session retarget) changed the target and reset hydratedFromDb. Don't
			// apply this now-stale folder's bundle or mark the new folder hydrated —
			// otherwise the stale result blocks the new folder's hydrate and its
			// drafts bleed across folders.
			if (path !== pipelineDraftPath) return
			let migratedFromLocal = false
			if (!bundle) {
				const local = readLocalBundle()
				if (local) {
					bundle = local
					migratedFromLocal = true
				}
			}
			if (fromServer) editor.loadedFromDbDraft = true
			if (bundle) restoreBundle(bundle)
			UserDraftDbSyncer.recordRemoteSync(
				{ workspace: ws ?? '', itemKind: PIPELINE_DRAFT_KIND, path },
				migratedFromLocal ? undefined : serverSavedAt
			)
			if (!migratedFromLocal) lastPersistedBundle = bundle ? JSON.stringify(bundle) : undefined
		} catch (e) {
			console.warn('failed to load pipeline drafts', e)
		} finally {
			if (path === pipelineDraftPath) editor.hydratedFromDb = true
		}
	}

	// Hydrate when persistence is on and this editor instance hasn't been hydrated
	// for the current folder yet. An effect (not onMount) so a folder retarget —
	// which resets `hydratedFromDb` — re-hydrates the new folder's draft bundle.
	$effect(() => {
		if (!persistDrafts || editor.hydratedFromDb) return
		void hydrateDrafts()
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
		const hydrated = editor.hydratedFromDb
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

<div class="h-full w-full" bind:clientWidth={containerWidth}>
	<Splitpanes class="!h-full" horizontal={stacked}>
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
					{readyDataUploadPaths}
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
					showMinimap={!stacked}
					viewportFitKey={viewportFitKey ?? folder}
				/>
				{#if boundBar}{@render boundBar()}{/if}
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
						<HideButton
							hidden={panelHidden}
							direction={stacked ? 'bottom' : 'right'}
							on:click={onTogglePanelHidden}
						/>
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
						{onRunCascadeByPath}
						{runFormInitialArgs}
						{onRunFormArgsChange}
						{resolveLocalScript}
						{localScriptsVersion}
						selection={activeDraft ? undefined : editor.selection}
						selectionProducers={activeDraft ? [] : selectionProducers}
						{selectionColumnGraph}
						{schemaCanEvolve}
						{selectionForkMaterialization}
						{schemaContractContext}
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
</div>
