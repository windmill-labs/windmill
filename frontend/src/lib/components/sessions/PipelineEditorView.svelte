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
	import { AssetService, type AssetKind, type Script } from '$lib/gen'
	import {
		parsePipelineAnnotations,
		type PipelineAnnotations
	} from '$lib/components/assets/AssetGraph/parsePipelineAnnotations'
	import { type AssetWithAltAccessType } from '$lib/components/assets/lib'
	import {
		createPipelineAiHelpers,
		type PipelineDraft
	} from '$lib/components/assets/AssetGraph/pipelineAiHelpers'

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

	let drafts = $state<Map<string, PipelineDraft>>(new Map())
	let selection = $state<AssetGraphSelection | undefined>(undefined)
	let activeDraftPath = $state<string | undefined>(undefined)

	let draftLocalIdCounter = 0
	function newDraftLocalId(): string {
		draftLocalIdCounter += 1
		return `sess-${draftLocalIdCounter}`
	}

	// Live editor overlays for the node open in the details pane, so the canvas
	// updates as the user (or AI) edits — same overlay inputs resolveGraph takes
	// in the full-page editor.
	const EMPTY_ANNOTATIONS: PipelineAnnotations = parsePipelineAnnotations('')
	let liveAnnotations = $state<{
		scriptPath: string | undefined
		annotations: PipelineAnnotations
	}>({ scriptPath: undefined, annotations: EMPTY_ANNOTATIONS })
	let liveBodyAssets = $state<{ scriptPath: string | undefined; assets: AssetWithAltAccessType[] }>(
		{
			scriptPath: undefined,
			assets: []
		}
	)
	let liveContent = $state<{ scriptPath: string | undefined; content: string }>({
		scriptPath: undefined,
		content: ''
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
	// accent-ringed via the shared resolveGraph aiPending threading).
	let resolvedGraph = $derived.by<AssetGraphResponse>(() =>
		resolveGraph({
			base: graphRes.current ?? EMPTY_GRAPH,
			drafts,
			liveBodyAssets,
			liveAnnotations,
			inferredWritesByPath: EMPTY_PATH_MAP,
			inferredReadsByPath: EMPTY_PATH_MAP,
			annotatedNativeKindsByPath: EMPTY_NATIVE_MAP
		})
	)

	let hasAiPending = $derived([...drafts.values()].some((d) => d.aiPending))
	let activeDraft = $derived(activeDraftPath ? drafts.get(activeDraftPath) : undefined)
	let detailsOpen = $derived(
		activeDraftPath != undefined ||
			(selection?.kind === 'runnable' && selection.runnable_kind === 'script')
	)

	const { helpers, acceptAll, rejectAll } = createPipelineAiHelpers({
		getFolder: () => path,
		getWorkspace: () => workspaceId,
		getResolvedGraph: () => resolvedGraph,
		getDrafts: () => drafts,
		setDrafts: (next) => (drafts = next),
		newDraftLocalId,
		onForgetPath: forgetPath,
		// Open the staged node in the details pane so its code is visible.
		onProposeNode: (p) => {
			activeDraftPath = p
			selection = undefined
		}
	})

	function forgetPath(p: string) {
		if (activeDraftPath === p) activeDraftPath = undefined
		if (selection?.kind === 'runnable' && selection.path === p) selection = undefined
		if (liveAnnotations.scriptPath === p)
			liveAnnotations = { scriptPath: undefined, annotations: EMPTY_ANNOTATIONS }
		if (liveBodyAssets.scriptPath === p) liveBodyAssets = { scriptPath: undefined, assets: [] }
		if (liveContent.scriptPath === p) liveContent = { scriptPath: undefined, content: '' }
	}

	function handleCanvasSelect(s: AssetGraphSelection | undefined) {
		if (s && s.kind === 'runnable' && s.runnable_kind === 'script' && drafts.has(s.path)) {
			activeDraftPath = s.path
			selection = undefined
		} else {
			activeDraftPath = undefined
			selection = s
		}
	}

	function handleDraftPersist(
		p: string,
		snapshot: { content: string; writes: { kind: AssetKind; path: string }[]; script?: Script }
	) {
		// Mirror the full-page editor: commit the editor buffer + inferred outputs
		// back into the drafts Map on pane teardown, deferred a microtask so a
		// discard in the same batch doesn't resurrect the entry.
		queueMicrotask(() => {
			const d = drafts.get(p)
			if (!d) {
				if (!snapshot.script) return
				const next = new Map(drafts)
				next.set(p, {
					localId: newDraftLocalId(),
					script: snapshot.script,
					outputAssets: snapshot.writes.length > 0 ? snapshot.writes : undefined
				})
				drafts = next
				return
			}
			const next = new Map(drafts)
			next.set(p, {
				...d,
				script: { ...d.script, content: snapshot.content },
				outputAssets: snapshot.writes.length > 0 ? snapshot.writes : undefined
			})
			drafts = next
		})
	}

	function discardActiveDraft() {
		const p = activeDraftPath
		if (!p || !drafts.has(p)) return
		const next = new Map(drafts)
		next.delete(p)
		drafts = next
		forgetPath(p)
	}

	async function afterSaved(savedPath: string) {
		const next = new Map(drafts)
		next.delete(savedPath)
		drafts = next
		if (activeDraftPath === savedPath) {
			selection = { kind: 'runnable', runnable_kind: 'script', path: savedPath }
			activeDraftPath = undefined
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
							{selection}
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
							selection={activeDraft ? undefined : selection}
							draftScript={activeDraft?.script}
							pathPrefix={`f/${path}/`}
							onAnnotationsChange={(scriptPath, annotations) =>
								(liveAnnotations = { scriptPath, annotations })}
							onAssetsChange={(scriptPath, assets) => (liveBodyAssets = { scriptPath, assets })}
							onContentChange={(scriptPath, content) => (liveContent = { scriptPath, content })}
							onDraftPersist={handleDraftPersist}
							onDiscard={discardActiveDraft}
							onDraftSaved={afterSaved}
							onPersistedSaved={afterSaved}
							onScriptRemoved={async (removedPath) => {
								forgetPath(removedPath)
								await graphRes.refetch()
							}}
							onclose={() => {
								selection = undefined
								activeDraftPath = undefined
								liveAnnotations = { scriptPath: undefined, annotations: EMPTY_ANNOTATIONS }
							}}
						/>
					</Pane>
				{/if}
			</Splitpanes>
		{/if}
	</div>
</div>
