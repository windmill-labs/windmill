<script lang="ts">
	import { untrack } from 'svelte'
	import { Loader2, Workflow } from 'lucide-svelte'
	import { page } from '$app/state'
	import PipelineGraphEditor from './PipelineGraphEditor.svelte'
	import PipelineActivityPanel from './PipelineActivityPanel.svelte'
	import { PipelineEditorState } from './pipelineEditorState.svelte'
	import { useActiveRunnableIds } from './activeRunnables.svelte'
	import { makeLaunch, makeWaitJobTerminal, runDownstreamCascade } from './cascadeRun'
	import { assetProducers, buildDownstreamMap } from './graphTraversal'
	import type { AssetGraphResponse, AssetGraphSelection } from './types'
	import { JobService, OpenAPI, type Preview, type Script } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptySchema, sendUserToast } from '$lib/utils'
	import { inferArgs } from '$lib/infer'

	// A local-dev pipeline preview driven by `wmill pipeline dev`. The CLI watches
	// an `f/<folder>` of `// pipeline` scripts, builds the asset graph from the
	// working tree, and pushes it over a WebSocket; this page renders that graph
	// with the same editor the UI uses and runs the cascade by PREVIEWING the
	// pushed local content (no deploy). Editing happens in the user's own editor;
	// each save live-reloads the graph here.

	type PushedScript = {
		path: string
		content: string
		language: Preview['language']
		tag?: string
	}
	type PipelineBundle = {
		type: 'pipeline'
		folder: string
		graph: AssetGraphResponse
		scripts: PushedScript[]
		temp_script_refs?: Record<string, string>
	}

	const sp = page.url.searchParams
	const token = sp.get('wm_token') ?? undefined
	const workspace = sp.get('workspace') ?? undefined
	const port = sp.get('port') ?? '3201'
	const folder = sp.get('folder') ?? ''
	// Per-session token gating the dev WS — the server rejects connections without
	// it so a stray localhost page can't read the pushed script source.
	const wsToken = sp.get('ws_token') ?? undefined

	$effect.pre(() => {
		if (token) {
			OpenAPI.WITH_CREDENTIALS = true
			OpenAPI.TOKEN = token
		}
	})
	$effect.pre(() => {
		if (workspace) $workspaceStore = workspace
	})

	const EMPTY_GRAPH: AssetGraphResponse = { assets: [], runnables: [], edges: [], triggers: [] }

	let bundle = $state<PipelineBundle | undefined>(undefined)
	let wsState = $state<'connecting' | 'open' | 'closed'>('connecting')

	// Externalized editor state (selection only — this view has no DB drafts; the
	// local files are the source of truth).
	const pe = new PipelineEditorState()
	$effect(() => {
		const f = folder
		untrack(() => {
			if (pe.folder !== f) pe.folder = f
		})
	})

	const displayGraph = $derived((bundle?.graph ?? EMPTY_GRAPH) as AssetGraphResponse)
	const pathPrefix = $derived(`f/${folder}/`)

	// Producers of the selected asset — the local scripts that write it (`w`/`rw`
	// edge, incl. the `// materialize` target). Shared `assetProducers` keeps this
	// in lockstep with the route page so the details pane shows the producer + its
	// runs instead of "No producer for this asset" (which only holds for the
	// deployed graph, absent here).
	const selectionProducers = $derived(assetProducers(displayGraph, pe.selection))

	// Subscriber count of the open script — gates the detail form's "Run +
	// downstream" (only meaningful when the script has a chain to fan out to).
	const openScriptPath = $derived(
		pe.selection?.kind === 'runnable' && pe.selection.runnable_kind === 'script'
			? pe.selection.path
			: undefined
	)
	const selectionDownstreamCount = $derived(
		openScriptPath ? (buildDownstreamMap(displayGraph).get(openScriptPath)?.size ?? 0) : 0
	)

	// path -> pushed local content, for preview-running each node.
	const scriptByPath = $derived(
		new Map<string, PushedScript>((bundle?.scripts ?? []).map((s) => [s.path, s]))
	)

	// Selecting a node must render its working-tree source + Run button, NOT
	// fetch the deployed script (this pipeline is local-only — that fetch 404s).
	// Synthesize a read-only `Script` from the pushed content and infer its args
	// schema (same wasm the script editor uses) so a parameterized node shows its
	// run-form inputs. The details pane only reads path/language/content/schema.
	async function resolveLocalScript(path: string): Promise<Script | undefined> {
		const s = scriptByPath.get(path)
		if (!s) return undefined
		const schema = emptySchema()
		try {
			await inferArgs(s.language as any, s.content ?? '', schema)
		} catch {
			// Inference failure (unsupported lang / parse error) → no inputs, still runnable.
		}
		return {
			hash: '',
			path,
			summary: '',
			description: '',
			content: s.content,
			schema,
			is_template: false,
			extra_perms: {},
			language: s.language as Script['language'],
			kind: 'script',
			created_by: '',
			created_at: '',
			archived: false,
			deleted: false,
			starred: false
		} as unknown as Script
	}

	// Live run state (node badges + event log), shared with the route page's poll.
	const activeRunnables = useActiveRunnableIds(
		() => workspace,
		() => pathPrefix
	)
	let activeRunnable = $state<{ kind: 'script' | 'flow'; path: string } | undefined>(undefined)
	let activeRunnableJobId = $state<string | undefined>(undefined)

	// Drive the selected node's runs pane (AssetRunsPanel): a just-launched
	// preview is `runsPendingJobId` (shown immediately), and bumping
	// `runsRefreshKey` re-fetches the list — mirrors the route page so a local run
	// appears + selects in the pane instead of only updating the node badge.
	let runsPendingJobId = $state<string | undefined>(undefined)
	let runsRefreshKey = $state(0)

	// Activity-panel cross-highlighting: hovering/expanding a run row emphasizes
	// its node(s) on the canvas (same wiring the route page uses).
	let activityHoverPaths = $state<string[]>([])
	let activitySelectPaths = $state<string[]>([])

	// Collapse the details/activity pane to give the graph full width/height —
	// works in both side-by-side and stacked layouts.
	let panelHidden = $state(false)
	$effect(() => {
		pathPrefix
		activeRunnables.setObserving(true)
		return () => activeRunnables.dispose()
	})
	$effect(() => {
		if (!activeRunnableJobId) return
		const ev = activeRunnables.events.find((e) => e.id === activeRunnableJobId)
		if (ev && (ev.status === 'success' || ev.status === 'failure')) {
			activeRunnable = undefined
			activeRunnableJobId = undefined
			// The job reached a terminal state — refetch the runs pane so it shows
			// the completed run + result, and drop the pending placeholder.
			runsPendingJobId = undefined
			runsRefreshKey += 1
		}
	})

	// Connect to the `wmill pipeline dev` WS, auto-reconnecting while mounted —
	// the dev server may bounce (the agent/user restarts it after changing config
	// or the folder), and the page should recover without a manual reload.
	$effect(() => {
		const _port = port
		let socket: WebSocket | undefined
		let retry: ReturnType<typeof setTimeout> | undefined
		let disposed = false

		function scheduleReconnect() {
			if (disposed) return
			wsState = 'closed'
			if (retry) return
			retry = setTimeout(() => {
				retry = undefined
				open()
			}, 1500)
		}
		function open() {
			if (disposed) return
			wsState = 'connecting'
			try {
				const q = wsToken ? `?token=${encodeURIComponent(wsToken)}` : ''
				socket = new WebSocket(`ws://localhost:${_port}/ws${q}`)
			} catch {
				scheduleReconnect()
				return
			}
			socket.addEventListener('open', () => (wsState = 'open'))
			socket.addEventListener('message', (event) => {
				try {
					const data = JSON.parse(event.data)
					if (data?.type === 'pipeline') bundle = data as PipelineBundle
				} catch {
					// ignore malformed frames
				}
			})
			socket.addEventListener('close', scheduleReconnect)
			socket.addEventListener('error', () => socket?.close())
		}
		open()
		return () => {
			disposed = true
			if (retry) clearTimeout(retry)
			socket?.close()
		}
	})

	function resolveLocal(
		path: string
	): { content: string; language: Preview['language']; tag?: string } | undefined {
		const s = scriptByPath.get(path)
		return s ? { content: s.content, language: s.language, tag: s.tag } : undefined
	}

	// Run a single node as a preview of its local content. Never fans out to
	// downstream deployed subscribers (skip-dispatch) — single-node runs stay local.
	async function runNode(
		nodePath: string,
		args: Record<string, any> = {}
	): Promise<string | undefined> {
		const local = resolveLocal(nodePath)
		if (!local || !workspace) {
			sendUserToast(`No local content for ${nodePath}`, true)
			return undefined
		}
		activeRunnables.arm(`script:${nodePath}`)
		try {
			const jobId = await JobService.runScriptPreview({
				workspace,
				requestBody: {
					content: local.content,
					language: local.language,
					path: nodePath,
					args: { ...args, _wmill_skip_asset_dispatch: true },
					...(local.tag ? { tag: local.tag } : {}),
					...(bundle?.temp_script_refs ? { temp_script_refs: bundle.temp_script_refs } : {})
				}
			})
			activeRunnable = { kind: 'script', path: nodePath }
			activeRunnableJobId = jobId
			runsPendingJobId = jobId
			runsRefreshKey += 1
			return jobId
		} catch (e: any) {
			sendUserToast(`Run failed: ${e?.body ?? e?.message ?? e}`, true)
			return undefined
		}
	}

	let cascadeRunningRoot = $state<string | undefined>(undefined)

	// "Run + downstream" — the client orchestrates the closure (preview each node
	// in topological order) since the backend dispatcher only resolves deployed rows.
	// `rootArgs` feeds the root node its inputs (e.g. an uploaded S3Object from the
	// detail-pane form) so a `data_upload` entry can seed the whole chain.
	async function runCascadeFrom(
		rootPath: string,
		rootArgs: Record<string, any> = {}
	): Promise<string | undefined> {
		if (!workspace) return undefined
		if (cascadeRunningRoot) {
			sendUserToast(`A chain run from ${cascadeRunningRoot} is still in progress`, true)
			return undefined
		}
		cascadeRunningRoot = rootPath
		let rootJobId: string | undefined
		const launch = makeLaunch({
			workspace,
			resolveLocal,
			tempScriptRefs: bundle?.temp_script_refs,
			argsFor: (p) => (p === rootPath ? rootArgs : undefined),
			onLaunched: (p, jobId) => {
				activeRunnables.arm(`script:${p}`)
				if (p === rootPath) {
					rootJobId = jobId
					activeRunnable = { kind: 'script', path: p }
					activeRunnableJobId = jobId
					runsPendingJobId = jobId
					runsRefreshKey += 1
				}
			}
		})
		try {
			const res = await runDownstreamCascade({
				graph: displayGraph,
				root: rootPath,
				launch,
				waitTerminal: makeWaitJobTerminal(workspace)
			})
			if (res.cyclic.length > 0) {
				sendUserToast(
					`Skipped ${res.cyclic.length} script(s) on a cycle: ${res.cyclic.join(', ')}`,
					true
				)
			}
			const n = res.statuses.size
			if (res.ok) {
				sendUserToast(`Chain run complete — ${n} script${n === 1 ? '' : 's'} succeeded`)
			} else {
				const failed = [...res.statuses.entries()].filter(([, s]) => s.status === 'failure')
				sendUserToast(`Chain run failed at ${failed.map(([p]) => p).join(', ')}`, true)
			}
		} finally {
			cascadeRunningRoot = undefined
		}
		return rootJobId
	}

	function runProducer(producer: { kind: 'script' | 'flow'; path: string; cascade?: boolean }) {
		if (producer.kind !== 'script') return Promise.resolve(undefined)
		return producer.cascade ? runCascadeFrom(producer.path) : runNode(producer.path)
	}

	function handleCanvasSelect(s: AssetGraphSelection | undefined) {
		pe.selection = s
	}
</script>

<div class="flex flex-col h-full w-full bg-surface">
	<div
		class="flex items-center gap-2 px-3 py-1.5 border-b border-light text-xs text-secondary shrink-0"
	>
		<Workflow size={14} />
		<span class="font-mono text-emphasis truncate">f/{folder}</span>
		<span class="text-tertiary">· local pipeline dev</span>
		<span class="ml-auto flex items-center gap-1.5">
			<span
				class="inline-block w-2 h-2 rounded-full {wsState === 'open'
					? 'bg-green-500'
					: wsState === 'connecting'
						? 'bg-yellow-500'
						: 'bg-red-500'}"
			></span>
			<span class="text-tertiary"
				>{wsState === 'open'
					? 'watching'
					: wsState === 'connecting'
						? 'connecting…'
						: 'disconnected'}</span
			>
		</span>
	</div>
	<div class="flex-1 min-h-0">
		{#if !bundle}
			<div class="h-full flex items-center justify-center gap-2 text-tertiary">
				<Loader2 size={18} class="animate-spin" />
				<span>Connecting to <code>wmill pipeline dev</code>…</span>
			</div>
		{:else if displayGraph.runnables.length === 0}
			<div class="h-full flex items-center justify-center text-sm text-tertiary px-4 text-center">
				No <code class="mx-1">// pipeline</code> scripts found in f/{folder}. Mark a script with a
				bare
				<code class="mx-1">// pipeline</code> comment.
			</div>
		{:else}
			<PipelineGraphEditor
				editor={pe}
				{folder}
				{displayGraph}
				mode="view"
				{workspace}
				{pathPrefix}
				{activeRunnable}
				{runsPendingJobId}
				{runsRefreshKey}
				activeRunnableIds={activeRunnables.ids}
				runStates={activeRunnables.states}
				eventLogEvents={activeRunnables.events}
				hoveredPaths={activityHoverPaths}
				selectedRunPaths={activitySelectPaths}
				{panelHidden}
				onTogglePanelHidden={() => (panelHidden = !panelHidden)}
				onRunProducer={runProducer}
				onRunByPath={(path, args) => runNode(path, args)}
				onRunCascadeByPath={(path, args) => runCascadeFrom(path, args)}
				downstreamSubscribers={selectionDownstreamCount}
				{resolveLocalScript}
				localScriptsVersion={bundle}
				{selectionProducers}
				canRunByPath
				onRunCompleted={() => {
					activeRunnable = undefined
					activeRunnableJobId = undefined
				}}
				onSelect={handleCanvasSelect}
				onClose={() => (pe.selection = undefined)}
			>
				{#snippet idlePane()}
					<PipelineActivityPanel
						events={activeRunnables.events}
						liveOnly
						onHoverRun={(p) => (activityHoverPaths = p ?? [])}
						onSelectRun={(p) => (activitySelectPaths = p ?? [])}
					/>
				{/snippet}
			</PipelineGraphEditor>
		{/if}
	</div>
</div>
