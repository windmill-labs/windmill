<script lang="ts">
	import { resource } from 'runed'
	import { tick, untrack } from 'svelte'
	import { Loader2, Workflow } from 'lucide-svelte'
	import PipelineGraphEditor from '$lib/components/assets/AssetGraph/PipelineGraphEditor.svelte'
	import PipelineTriggerEditors from '$lib/components/assets/AssetGraph/PipelineTriggerEditors.svelte'
	import { getAiChatManager } from '$lib/components/copilot/chat/aiChatManagerContext'
	import { resolveGraph } from '$lib/components/assets/AssetGraph/resolveGraph'
	import { useActiveRunnableIds } from '$lib/components/assets/AssetGraph/activeRunnables.svelte'
	import type {
		AssetGraphResponse,
		AssetGraphSelection,
		NativeTriggerKind
	} from '$lib/components/assets/AssetGraph/types'
	import { AssetService, JobService, type AssetKind } from '$lib/gen'
	import { sendUserToast } from '$lib/utils'
	import { createPipelineAiHelpers } from '$lib/components/assets/AssetGraph/pipelineAiHelpers'
	import type { SessionRuntime } from './sessionRuntime.svelte'

	let {
		runtime,
		path,
		workspaceId,
		isActiveSession = true
	}: {
		/** Per-session runtime; owns the pipeline editor state so it survives the
		 * editor pane unmounting on hide/show. */
		runtime: SessionRuntime
		/** Folder name the pipeline graph is scoped to (not a workspace item path). */
		path: string
		workspaceId: string
		/** Only the visible session registers the pipeline tools on its manager. */
		isActiveSession?: boolean
	} = $props()

	// The session's scoped chat manager (falls back to the singleton off-session).
	const aiChatManager = getAiChatManager()

	// Externalized editor state — lives on the runtime so the drafts persist across
	// hide/show of the preview pane (the pane unmounts on hide).
	const pe = runtime.pipelineEditorState

	// The reused `pe` is scoped to a folder. A same-folder remount (hide→show)
	// keeps the drafts; a retarget to a different folder resets so stale drafts
	// don't bleed across folders. untrack the writes so this can't self-loop.
	$effect(() => {
		const folder = path
		untrack(() => {
			if (pe.folder !== folder) {
				if (pe.folder !== undefined) {
					pe.reset()
					// A retarget without remount re-scopes the poll, so the release
					// effect could never match the old folder's job — drop the hint.
					activeRunnable = undefined
					activeRunnableJobId = undefined
					// Re-scope the Global pipeline prompt to the new folder (the helper
					// methods already read the reactive path, but the system message
					// string was built for the old one). Only when this session is the
					// active one — its helpers are the registered set; a hidden session
					// reconfigures when it next becomes active.
					if (isActiveSession) aiChatManager.rebuildGlobalSystemMessage()
				}
				pe.folder = folder
			}
		})
	})

	const EMPTY_GRAPH: AssetGraphResponse = { assets: [], runnables: [], edges: [], triggers: [] }
	const EMPTY_PATH_MAP = new Map<string, Array<{ kind: AssetKind; path: string }>>()
	const EMPTY_NATIVE_MAP = new Map<string, Set<any>>()

	const graphRes = resource(
		() => ({ workspace: workspaceId, folder: path }),
		async ({ workspace, folder }) =>
			workspace && folder ? await AssetService.getAssetsGraph({ workspace, folder }) : EMPTY_GRAPH
	)

	// Folder whose graph is actually rendered — `graphRes.current` is stale-
	// while-revalidate on a folder retarget, so the canvas's one-shot initial
	// fit is keyed on the folder captured when a graph lands, not on `path`
	// (same rationale as the pipeline route page).
	let viewportFitFolder = $state('')
	$effect(() => {
		if (graphRes.current) untrack(() => (viewportFitFolder = path))
	})

	// Deployed graph + the in-flight draft overlay (AI-built nodes render as plain
	// dashed unsaved drafts, same as manual drafts). The session
	// skips the route page's folder-wide asset prefetch (empty inferred maps); the
	// open script's live overlays still feed the graph. (resolveGraph's base is the
	// pipeline runnables subset; the 'job' usage_kind of the wire type never appears.)
	let resolvedGraph = $derived.by<AssetGraphResponse>(() =>
		resolveGraph({
			base: (graphRes.current ?? EMPTY_GRAPH) as AssetGraphResponse,
			drafts: pe.drafts,
			liveBodyAssets: pe.liveBodyAssets,
			liveAnnotations: pe.liveAnnotations,
			inferredWritesByPath: EMPTY_PATH_MAP,
			inferredReadsByPath: EMPTY_PATH_MAP,
			annotatedNativeKindsByPath: EMPTY_NATIVE_MAP
		})
	)

	const helpers = createPipelineAiHelpers({
		getFolder: () => path,
		getWorkspace: () => workspaceId,
		getResolvedGraph: () => resolvedGraph,
		getDrafts: () => pe.drafts,
		setDrafts: (next) => (pe.drafts = next),
		newDraftLocalId: pe.newDraftLocalId,
		onForgetPath: pe.forgetPath,
		// Open the staged node in the details pane so its code is visible.
		onProposeNode: (p) => {
			pe.activeDraftPath = p
			pe.selection = undefined
		},
		// An AI test_pipeline_node run lights up the same live badge as the run button.
		onRunStarted: (jobId, p) => {
			activeRunnables.arm(`script:${p}`)
			activeRunnable = { kind: 'script', path: p }
			activeRunnableJobId = jobId
		}
	})

	function handleCanvasSelect(s: AssetGraphSelection | undefined) {
		if (s && s.kind === 'runnable' && s.runnable_kind === 'script' && pe.drafts.has(s.path)) {
			pe.activeDraftPath = s.path
			pe.selection = undefined
		} else {
			pe.activeDraftPath = undefined
			pe.selection = s
		}
	}

	async function afterSaved(savedPath: string) {
		const next = new Map(pe.drafts)
		next.delete(savedPath)
		pe.drafts = next
		if (pe.activeDraftPath === savedPath) {
			pe.selection = { kind: 'runnable', runnable_kind: 'script', path: savedPath }
			pe.activeDraftPath = undefined
		}
		await graphRes.refetch()
	}

	// Native trigger editor drawers — the shared <PipelineTriggerEditors> the route
	// page uses; the canvas drives them imperatively. The draft guards mirror the
	// route page: a trigger row stores a hard `script_path`, so it can only attach
	// to a deployed script.
	let triggerEditors: PipelineTriggerEditors | undefined = $state(undefined)
	let focusDataUploadSignal = $state(0)

	function openMissingTriggerDrawer(kind: NativeTriggerKind, scriptPath: string) {
		if (pe.drafts.has(scriptPath)) {
			sendUserToast(
				`Save the script "${scriptPath}" first — triggers can only be attached to deployed scripts.`,
				true
			)
			return
		}
		triggerEditors?.openNew(kind, scriptPath)
	}

	function openEditTriggerDrawer(kind: NativeTriggerKind, triggerPath: string, scriptPath: string) {
		triggerEditors?.openEdit(kind, triggerPath, scriptPath)
	}

	function deleteAttachedTrigger(kind: NativeTriggerKind, triggerPath: string) {
		triggerEditors?.requestDelete(kind, triggerPath)
	}

	function openWebhookDrawer(scriptPath: string) {
		if (pe.drafts.has(scriptPath)) {
			sendUserToast(
				`Save the script "${scriptPath}" first — webhooks only trigger the deployed version.`,
				true
			)
			return
		}
		triggerEditors?.openWebhook(scriptPath)
	}

	// Data upload has no trigger row — open the target in the details pane and pulse
	// the signal so its auto-generated run form focuses the S3 input.
	function openDataUploadRun(scriptPath: string) {
		if (pe.drafts.has(scriptPath)) {
			pe.activeDraftPath = scriptPath
			pe.selection = undefined
		} else {
			pe.activeDraftPath = undefined
			pe.selection = { kind: 'runnable', runnable_kind: 'script', path: scriptPath }
		}
		void tick().then(() => focusDataUploadSignal++)
	}

	// ── Run dispatch + live run state ──────────────────────────────────────────
	// Reuses the shared folder-scoped job poll (node badges + event log) the route
	// page uses. The session runs ONE node at a time (preview for an unsaved draft,
	// the deployed version otherwise) — it skips the route page's cascade/deploy
	// queue, which the AI-session run UX doesn't need.
	const pathPrefix = $derived(`f/${path}/`)
	const activeRunnables = useActiveRunnableIds(
		() => workspaceId,
		() => pathPrefix
	)
	// Zero-latency "running" hint for a node run launched from the canvas (the poll
	// hasn't seen the job yet). Released when that exact job reaches a terminal
	// status in the poll, so the badge hands off to the poll's status with no flash.
	let activeRunnable = $state<{ kind: 'script' | 'flow'; path: string } | undefined>(undefined)
	let activeRunnableJobId = $state<string | undefined>(undefined)
	$effect(() => {
		pathPrefix // re-scope the poll when the folder changes
		// Only the visible session needs live badges/event-log; hidden warm panes
		// (up to MAX_WARM_EDITORS) shouldn't poll in the background.
		activeRunnables.setObserving(isActiveSession)
		return () => activeRunnables.dispose()
	})
	$effect(() => {
		if (!activeRunnableJobId) return
		const ev = activeRunnables.events.find((e) => e.id === activeRunnableJobId)
		if (ev && (ev.status === 'success' || ev.status === 'failure')) {
			activeRunnable = undefined
			activeRunnableJobId = undefined
		}
	})

	function runProducer(producer: { kind: 'script' | 'flow'; path: string; cascade?: boolean }) {
		// Pipeline nodes are scripts; a flow producer can't be preview/by-path run.
		if (producer.kind !== 'script') return Promise.resolve(undefined)
		return runNode(producer.path, {}, producer.cascade ?? false)
	}

	async function runNode(
		nodePath: string,
		args: Record<string, any> = {},
		cascade = false
	): Promise<string | undefined> {
		const draft = pe.drafts.get(nodePath)
		activeRunnables.arm(`script:${nodePath}`)
		try {
			let jobId: string
			if (draft) {
				// Preview-run the draft content; a preview never dispatches downstream.
				jobId = await JobService.runScriptPreview({
					workspace: workspaceId,
					requestBody: {
						path: nodePath,
						content: draft.script.content,
						language: draft.script.language,
						args
					}
				})
			} else {
				// A single-node run must NOT fan out to downstream deployed subscribers
				// via the backend asset dispatcher unless the user explicitly chose
				// "run + downstream" (cascade) — otherwise clicking one node's Run would
				// fire side-effecting production scripts.
				jobId = await JobService.runScriptByPath({
					workspace: workspaceId,
					path: nodePath,
					requestBody: { ...args, ...(cascade ? {} : { _wmill_skip_asset_dispatch: true }) }
				})
			}
			activeRunnable = { kind: 'script', path: nodePath }
			activeRunnableJobId = jobId
			return jobId
		} catch (e: any) {
			sendUserToast(`Run failed: ${e?.body ?? e?.message ?? e}`, true)
			return undefined
		}
	}

	// Register the pipeline tools on this session's manager while the view is the
	// active one. setPipelineHelpers rebuilds the global tool set to include the
	// pipeline tools and tears them down on cleanup.
	$effect(() => {
		if (!isActiveSession) return
		return aiChatManager.setPipelineHelpers(helpers)
	})
</script>

<div class="flex flex-col h-full w-full bg-surface">
	<div
		class="flex items-center gap-2 px-3 py-1.5 border-b border-light text-xs text-secondary shrink-0"
	>
		<Workflow size={14} />
		<span class="font-mono text-emphasis truncate">f/{path}</span>
		<span class="text-tertiary">· data pipeline</span>
	</div>
	<div class="flex-1 min-h-0">
		<!-- Only block on the deployed-graph fetch when there's nothing to show yet.
		     When the runtime already holds drafts (e.g. returning to a session whose
		     editor pane was LRU-unmounted, so `graphRes` re-fetches from scratch),
		     render the editor immediately so the drafts stay visible — `resolveGraph`
		     overlays them on an empty base and the deployed nodes fill in when the
		     fetch resolves. -->
		{#if graphRes.loading && !graphRes.current && pe.drafts.size === 0}
			<div class="h-full flex items-center justify-center gap-2 text-tertiary">
				<Loader2 size={18} class="animate-spin" />
				<span>Loading pipeline…</span>
			</div>
		{:else if graphRes.error && pe.drafts.size === 0}
			<div class="h-full flex items-center justify-center text-red-500 text-sm px-4 text-center">
				Failed to load pipeline: {graphRes.error.message}
			</div>
		{:else}
			<!-- The session preview shares the route page's editor body and persists
			     drafts to the same per-folder DB draft, so AI drafts survive a reload /
			     session switch (and an LRU-evicted runtime). It wires the shared run +
			     trigger affordances (handlers above) but omits the route page's
			     cascade / bounded-run / add-script-from-asset callbacks, so those stay
			     hidden. -->
			<PipelineGraphEditor
				editor={pe}
				folder={path}
				viewportFitKey={viewportFitFolder}
				persistDrafts={true}
				displayGraph={resolvedGraph}
				mode="edit"
				workspace={workspaceId}
				{pathPrefix}
				onCreateMissingTrigger={openMissingTriggerDrawer}
				onEditTrigger={openEditTriggerDrawer}
				onDeleteTrigger={deleteAttachedTrigger}
				onOpenWebhook={openWebhookDrawer}
				onOpenDataUpload={openDataUploadRun}
				focusUploadSignal={focusDataUploadSignal}
				{activeRunnable}
				activeRunnableIds={activeRunnables.ids}
				runStates={activeRunnables.states}
				eventLogEvents={activeRunnables.events}
				onRunProducer={runProducer}
				onRunByPath={(path, args) => runNode(path, args)}
				canRunByPath
				onTestStateChange={(running) => {
					const openPath = pe.openScriptPath
					if (running && openPath) {
						activeRunnable = { kind: 'script', path: openPath }
						activeRunnables.arm(`script:${openPath}`)
						activeRunnableJobId = undefined
					} else if (!running && activeRunnable?.path === openPath) {
						// Only clear the hint for the script the pane just finished — a
						// canvas per-node run of a different script keeps its own hint.
						activeRunnable = undefined
						activeRunnableJobId = undefined
					}
				}}
				onRunCompleted={() => {
					activeRunnable = undefined
					activeRunnableJobId = undefined
				}}
				onSelect={handleCanvasSelect}
				onDraftSaved={afterSaved}
				onPersistedSaved={afterSaved}
				onScriptRemoved={async (removedPath) => {
					pe.forgetPath(removedPath)
					await graphRes.refetch()
				}}
				onScriptRenamed={async (oldPath, newPath) => {
					// Repoint the selection so the canvas follows the renamed node instead
					// of staying on the now-gone old path until an unrelated refetch.
					if (pe.selection?.kind === 'runnable' && pe.selection.path === oldPath) {
						pe.selection = { ...pe.selection, path: newPath }
					}
					await graphRes.refetch()
				}}
				onDiscard={() => {
					if (pe.activeDraftPath) pe.discardDraft(pe.activeDraftPath)
				}}
				onClose={() => {
					pe.selection = undefined
					pe.activeDraftPath = undefined
					pe.clearLiveOverlays()
				}}
			/>
		{/if}
	</div>
</div>

<!-- Native trigger editor drawers (schedule/kafka/webhook/…), shared with the
     route page; opened imperatively from the canvas via the handlers above.
     NOTE: these operate on the global `$workspaceStore`, not the `workspaceId`
     prop. That's correct only because activating a session syncs the store to
     the session's workspace (SessionPicker), and triggers are only opened from
     the visible/active session — so for a forked-workspace session the store is
     already pointed at the right workspace by the time a drawer opens. -->
<PipelineTriggerEditors
	bind:this={triggerEditors}
	mountTriggerEditors
	onUpdate={() => graphRes.refetch()}
/>
