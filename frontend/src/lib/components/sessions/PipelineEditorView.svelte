<script lang="ts">
	import { resource } from 'runed'
	import { Loader2, Workflow } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import AssetGraphCanvas from '$lib/components/assets/AssetGraph/AssetGraphCanvas.svelte'
	import AssetGraphDetailsPane from '$lib/components/assets/AssetGraph/AssetGraphDetailsPane.svelte'
	import GlobalReviewButtons from '$lib/components/copilot/chat/GlobalReviewButtons.svelte'
	import { getAiChatManager } from '$lib/components/copilot/chat/aiChatManagerContext'
	import { resolveGraph } from '$lib/components/assets/AssetGraph/resolveGraph'
	import type {
		AssetGraphResponse,
		AssetGraphSelection
	} from '$lib/components/assets/AssetGraph/types'
	import { AssetService, type AssetKind } from '$lib/gen'
	import { createPipelineAiHelpers } from '$lib/components/assets/AssetGraph/pipelineAiHelpers'
	import { PipelineEditorState } from '$lib/components/assets/AssetGraph/pipelineEditorState.svelte'

	let {
		path,
		workspaceId,
		isActiveSession = true
	}: {
		/** Folder name the pipeline graph is scoped to (not a workspace item path). */
		path: string
		workspaceId: string
		/** Only the visible session registers the pipeline tools on its manager. */
		isActiveSession?: boolean
	} = $props()

	// The session's scoped chat manager (falls back to the singleton off-session).
	const aiChatManager = getAiChatManager()

	// Externalized editor state, shared with the route page's editor.
	const pe = new PipelineEditorState()

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
	// open script's live overlays still feed the graph.
	let resolvedGraph = $derived.by<AssetGraphResponse>(() =>
		resolveGraph({
			base: graphRes.current ?? EMPTY_GRAPH,
			drafts: pe.drafts,
			liveBodyAssets: pe.liveBodyAssets,
			liveAnnotations: pe.liveAnnotations,
			inferredWritesByPath: EMPTY_PATH_MAP,
			inferredReadsByPath: EMPTY_PATH_MAP,
			annotatedNativeKindsByPath: EMPTY_NATIVE_MAP
		})
	)

	let hasAiPending = $derived([...pe.drafts.values()].some((d) => d.aiPending))
	let activeDraft = $derived(pe.activeDraftPath ? pe.drafts.get(pe.activeDraftPath) : undefined)
	let detailsOpen = $derived(
		pe.activeDraftPath != undefined ||
			(pe.selection?.kind === 'runnable' && pe.selection.runnable_kind === 'script')
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
			<Splitpanes class="!h-full">
				<Pane size={detailsOpen ? 60 : 100}>
					<div class="relative h-full">
						<AssetGraphCanvas
							graph={resolvedGraph}
							selection={pe.selection}
							pathPrefix={`f/${path}/`}
							onselect={handleCanvasSelect}
						/>
						{#if hasAiPending}
							<GlobalReviewButtons onAcceptAll={acceptAll} onRejectAll={rejectAll} />
						{/if}
					</div>
				</Pane>
				{#if detailsOpen}
					<Pane size={40} minSize={25}>
						<AssetGraphDetailsPane
							mode="edit"
							workspace={workspaceId}
							selection={activeDraft ? undefined : pe.selection}
							draftScript={activeDraft?.script}
							pathPrefix={`f/${path}/`}
							onAnnotationsChange={pe.handleAnnotationsChange}
							onAssetsChange={pe.handleAssetsChange}
							onContentChange={pe.handleContentChange}
							onDraftPersist={pe.handleDraftPersist}
							onDiscard={() => {
								if (pe.activeDraftPath) pe.discardDraft(pe.activeDraftPath)
							}}
							onDraftSaved={afterSaved}
							onPersistedSaved={afterSaved}
							onScriptRemoved={async (removedPath) => {
								pe.forgetPath(removedPath)
								await graphRes.refetch()
							}}
							onclose={() => {
								pe.selection = undefined
								pe.activeDraftPath = undefined
								pe.clearLiveOverlays()
							}}
						/>
					</Pane>
				{/if}
			</Splitpanes>
		{/if}
	</div>
</div>
