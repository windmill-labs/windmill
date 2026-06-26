<script lang="ts">
	import { resource } from 'runed'
	import { Loader2, Workflow } from 'lucide-svelte'
	import AssetGraphCanvas from '$lib/components/assets/AssetGraph/AssetGraphCanvas.svelte'
	import GlobalReviewButtons from '$lib/components/copilot/chat/GlobalReviewButtons.svelte'
	import { getAiChatManager } from '$lib/components/copilot/chat/aiChatManagerContext'
	import { resolveGraph } from '$lib/components/assets/AssetGraph/resolveGraph'
	import type {
		AssetGraphResponse,
		AssetGraphSelection
	} from '$lib/components/assets/AssetGraph/types'
	import {
		AssetService,
		JobService,
		ScriptService,
		type AssetKind,
		type Script,
		type ScriptLang
	} from '$lib/gen'
	import {
		assetUri,
		autoOutputAsset,
		type PipelineOutputKind
	} from '$lib/components/assets/AssetGraph/pipelineTemplates'
	import { parsePipelineAnnotations } from '$lib/components/assets/AssetGraph/parsePipelineAnnotations'
	import { extractWrites, type AssetWithAltAccessType } from '$lib/components/assets/lib'
	import { inferAssets } from '$lib/infer'
	import { emptySchema, sendUserToast } from '$lib/utils'
	import type {
		PipelineAIChatHelpers,
		PipelineContext,
		PipelineNodeSummary
	} from '$lib/components/copilot/chat/pipeline/core'

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

	type Draft = {
		script: Script
		outputAsset?: { kind: AssetKind; path: string }
		outputAssets?: Array<{ kind: AssetKind; path: string }>
		aiPending?: boolean
	}

	let drafts = $state<Map<string, Draft>>(new Map())
	let selection = $state<AssetGraphSelection | undefined>(undefined)

	// Pre-proposal snapshots so Reject restores the exact prior state (a previous
	// draft, or absence). Non-reactive — pure revert bookkeeping. See the route
	// editor's identical mechanism in /pipeline/[folder]/+page.svelte.
	const aiSnapshots = new Map<string, Draft | undefined>()

	const EMPTY_GRAPH: AssetGraphResponse = { assets: [], runnables: [], edges: [], triggers: [] }
	const EMPTY_LIVE_ASSETS = {
		scriptPath: undefined as string | undefined,
		assets: [] as AssetWithAltAccessType[]
	}
	const EMPTY_LIVE_ANNOTATIONS = {
		scriptPath: undefined as string | undefined,
		annotations: parsePipelineAnnotations('')
	}
	const EMPTY_PATH_MAP = new Map<string, Array<{ kind: AssetKind; path: string }>>()
	const EMPTY_NATIVE_MAP = new Map<string, Set<never>>()

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
			liveBodyAssets: EMPTY_LIVE_ASSETS,
			liveAnnotations: EMPTY_LIVE_ANNOTATIONS,
			inferredWritesByPath: EMPTY_PATH_MAP,
			inferredReadsByPath: EMPTY_PATH_MAP,
			annotatedNativeKindsByPath: EMPTY_NATIVE_MAP as Map<string, Set<any>>
		})
	)

	let hasAiPending = $derived([...drafts.values()].some((d) => d.aiPending))

	function makeScript(language: ScriptLang, scriptPath: string, content: string): Script {
		return {
			hash: '',
			path: scriptPath,
			summary: '',
			description: '',
			content,
			schema: emptySchema(),
			is_template: false,
			extra_perms: {},
			language,
			kind: 'script',
			created_by: '',
			created_at: new Date().toISOString(),
			archived: false,
			deleted: false,
			starred: false
		} as unknown as Script
	}

	function snapshotPath(p: string) {
		if (!aiSnapshots.has(p)) aiSnapshots.set(p, drafts.get(p))
	}

	async function inferOutputAssets(
		language: ScriptLang,
		content: string
	): Promise<Array<{ kind: AssetKind; path: string }>> {
		try {
			const inferred = await inferAssets(language, content)
			if (inferred?.status === 'error') return []
			return extractWrites((inferred?.assets ?? []) as AssetWithAltAccessType[])
		} catch {
			return []
		}
	}

	function buildPipelineContext(): PipelineContext {
		const graph = resolvedGraph
		const nodes: PipelineNodeSummary[] = graph.runnables
			.filter((r) => r.usage_kind === 'script')
			.map((r) => {
				const draft = drafts.get(r.path)
				const writes = graph.edges
					.filter(
						(e) =>
							e.runnable_kind === 'script' &&
							e.runnable_path === r.path &&
							(e.access_type === 'w' || e.access_type === 'rw')
					)
					.map((e) => assetUri({ kind: e.asset_kind, path: e.asset_path }))
				const reads = graph.edges
					.filter(
						(e) =>
							e.runnable_kind === 'script' &&
							e.runnable_path === r.path &&
							(e.access_type === 'r' || e.access_type === 'rw')
					)
					.map((e) => assetUri({ kind: e.asset_kind, path: e.asset_path }))
				const triggers = graph.triggers
					.filter((t) => t.runnable_kind === 'script' && t.runnable_path === r.path)
					.map((t) =>
						t.trigger_kind === 'asset'
							? assetUri({ kind: t.asset_kind, path: t.asset_path })
							: t.trigger_kind
					)
				return {
					path: r.path,
					language: draft?.script.language,
					unsaved: r.unsaved ?? false,
					aiPending: draft?.aiPending ?? false,
					summary: draft?.script.summary || undefined,
					writes: [...new Set(writes)],
					reads: [...new Set(reads)],
					triggers: [...new Set(triggers)]
				}
			})
		return {
			folder: path,
			mode: 'edit',
			nodes,
			assets: graph.assets.map((a) => assetUri({ kind: a.kind, path: a.path })),
			pendingProposals: nodes.filter((n) => n.aiPending).length
		}
	}

	function revertPath(p: string) {
		const snap = aiSnapshots.get(p)
		const next = new Map(drafts)
		if (snap === undefined) next.delete(p)
		else next.set(p, snap)
		drafts = next
		aiSnapshots.delete(p)
	}

	const helpers: PipelineAIChatHelpers = {
		getPipelineContext: buildPipelineContext,
		getNodeBody: async (p) => {
			const draft = drafts.get(p)
			if (draft) return { language: draft.script.language, content: draft.script.content }
			try {
				const deployed = await ScriptService.getScriptByPath({ workspace: workspaceId, path: p })
				return { language: deployed.language, content: deployed.content }
			} catch {
				return undefined
			}
		},
		proposeNode: async ({ path: nodePath, language, content, outputKind }) => {
			if (drafts.has(nodePath) && !drafts.get(nodePath)?.aiPending) {
				throw new Error(
					`A draft already exists at '${nodePath}'. Use edit_pipeline_node to change it instead.`
				)
			}
			snapshotPath(nodePath)
			const outputAssets = await inferOutputAssets(language, content)
			const seededOutput =
				outputAssets[0] ?? (outputKind ? autoOutputAsset(outputKind, path, language) : undefined)
			const prev = drafts.get(nodePath)
			const next = new Map(drafts)
			next.set(nodePath, {
				script: makeScript(language, nodePath, content),
				outputAsset: seededOutput,
				outputAssets: outputAssets.length > 0 ? outputAssets : undefined,
				aiPending: true
			})
			void prev
			drafts = next
			selection = { kind: 'runnable', runnable_kind: 'script', path: nodePath }
			return { path: nodePath }
		},
		editNode: async (p, content) => {
			const existing = drafts.get(p)
			let language: ScriptLang
			if (existing) {
				language = existing.script.language
			} else {
				const deployed = await ScriptService.getScriptByPath({ workspace: workspaceId, path: p })
				language = deployed.language
			}
			snapshotPath(p)
			const outputAssets = await inferOutputAssets(language, content)
			const next = new Map(drafts)
			next.set(p, {
				script: makeScript(language, p, content),
				outputAsset: outputAssets[0] ?? existing?.outputAsset,
				outputAssets: outputAssets.length > 0 ? outputAssets : existing?.outputAssets,
				aiPending: true
			})
			drafts = next
			selection = { kind: 'runnable', runnable_kind: 'script', path: p }
		},
		removeProposedNode: async (p) => {
			if (!drafts.get(p)?.aiPending) {
				throw new Error(`'${p}' is not an AI-pending node, so it cannot be removed here.`)
			}
			revertPath(p)
		},
		hasPendingProposals: () => [...drafts.values()].some((d) => d.aiPending),
		acceptAllProposals: acceptAll,
		rejectAllProposals: rejectAll,
		testNode: async (p, args) => {
			const draft = drafts.get(p)
			try {
				if (draft) {
					return await JobService.runScriptPreview({
						workspace: workspaceId,
						requestBody: {
							path: p,
							content: draft.script.content,
							language: draft.script.language,
							args: args ?? {}
						}
					})
				}
				return await JobService.runScriptByPath({
					workspace: workspaceId,
					path: p,
					requestBody: { ...(args ?? {}) }
				})
			} catch (e: any) {
				sendUserToast(`Run failed: ${e?.body ?? e?.message ?? e}`, true)
				return undefined
			}
		}
	}

	function acceptAll() {
		const next = new Map(drafts)
		for (const [p, d] of next) if (d.aiPending) next.set(p, { ...d, aiPending: false })
		drafts = next
		aiSnapshots.clear()
	}

	function rejectAll() {
		for (const p of [...aiSnapshots.keys()]) revertPath(p)
		aiSnapshots.clear()
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
	<div class="relative flex-1 min-h-0">
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
			<AssetGraphCanvas
				graph={resolvedGraph}
				{selection}
				pathPrefix={`f/${path}/`}
				onselect={(s) => (selection = s)}
			/>
			{#if hasAiPending}
				<GlobalReviewButtons onAcceptAll={acceptAll} onRejectAll={rejectAll} />
			{/if}
		{/if}
	</div>
</div>
