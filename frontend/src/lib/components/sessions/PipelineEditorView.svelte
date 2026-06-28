<script lang="ts">
	import { resource } from 'runed'
	import { untrack } from 'svelte'
	import { Loader2, Workflow } from 'lucide-svelte'
	import PipelineGraphEditor from '$lib/components/assets/AssetGraph/PipelineGraphEditor.svelte'
	import { getAiChatManager } from '$lib/components/copilot/chat/aiChatManagerContext'
	import { resolveGraph } from '$lib/components/assets/AssetGraph/resolveGraph'
	import type {
		AssetGraphResponse,
		AssetGraphSelection
	} from '$lib/components/assets/AssetGraph/types'
	import { AssetService, type AssetKind } from '$lib/gen'
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
				if (pe.folder !== undefined) pe.reset()
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

	// Deployed graph + the in-flight draft overlay (AI proposals render dashed +
	// accent-ringed via the shared resolveGraph aiPending threading). The session
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

	const { helpers, acceptAll, rejectAll } = createPipelineAiHelpers({
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
		{#if graphRes.loading && !graphRes.current}
			<div class="h-full flex items-center justify-center gap-2 text-tertiary">
				<Loader2 size={18} class="animate-spin" />
				<span>Loading pipeline…</span>
			</div>
		{:else if graphRes.error}
			<div class="h-full flex items-center justify-center text-red-500 text-sm px-4 text-center">
				Failed to load pipeline: {graphRes.error.message}
			</div>
		{:else}
			<!-- The session preview shares the route page's editor body. It opts out of
			     persistence (persistDrafts=false) and the run/cascade/trigger/bounded
			     affordances (their callbacks are omitted, so the canvas/pane hide them);
			     building + the diff/approval review still work via the AI helpers. -->
			<PipelineGraphEditor
				editor={pe}
				folder={path}
				persistDrafts={false}
				displayGraph={resolvedGraph}
				mode="edit"
				workspace={workspaceId}
				pathPrefix={`f/${path}/`}
				hasAiPending={pe.hasAiPending}
				onAcceptAllProposals={acceptAll}
				onRejectAllProposals={rejectAll}
				onSelect={handleCanvasSelect}
				onDraftSaved={afterSaved}
				onPersistedSaved={afterSaved}
				onScriptRemoved={async (removedPath) => {
					pe.forgetPath(removedPath)
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
