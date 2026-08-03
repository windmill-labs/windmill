<script lang="ts">
	// The dbt editor's model graph, refreshed on demand.
	//
	// dbt's own DAG, drawn from a manifest dbt itself produced — at deploy, or by
	// a `parse` of the buffer being edited. Never from a `ref()` scan of our own:
	// that yields names without relations (a node is
	// `dbt://<warehouse>/<schema>/<table>`, and schema, database and alias come
	// from configs layered with the profile) and disagrees with dbt about
	// `enabled`, macro-built refs, loops and packages.
	//
	// Explicit, because a refresh costs a worker job: engine provisioning is
	// cached per (version, adapter) and `dbt deps` per lock digest, so a warm
	// refresh is seconds and a cold one is a download.
	import { onDestroy, untrack } from 'svelte'
	import { JobService, OpenAPI, type ScriptModule } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import { Loader2, RefreshCw } from 'lucide-svelte'
	import { displayDate } from '$lib/utils'
	import { base } from '$lib/base'
	import AssetGraphCanvas from '$lib/components/assets/AssetGraph/AssetGraphCanvas.svelte'
	import type { DbtPreviewBuffer } from './previewRows'
	import type {
		AssetGraphNodeData,
		AssetGraphResponse,
		DbtAssetProvenance
	} from '$lib/components/assets/AssetGraph/types'
	import { useDbtRunStatus } from './runStatus.svelte'

	let {
		workspace,
		scriptPath,
		/** The descriptor buffer. It is the script's content, and what a parse of
		 *  it runs with. */
		descriptor,
		/** The project. It travels inline as `_MODULES`, the same way a Test does:
		 *  the bundle IS the project, and dbt cannot parse a subset of it. */
		modules,
		/** The run form's arguments. A descriptor interpolating `{{ day }}` takes
		 *  one, and a parse uses whatever has been filled in — it tolerates the
		 *  rest being absent, as the deploy's own parse does. */
		args,
		/** The script's worker tag and timeout. A parse is a real dbt invocation:
		 *  a project reaching a private network parses only on the worker that can
		 *  reach it, and a slow one must get the same budget a run would. */
		tag,
		timeout,
		/** The deployed version, when there is one. Its graph is what the panel
		 *  shows before anything has been parsed from the buffer. */
		deployedHash,
		/** The build the editor is running, so the models it touches colour while
		 *  it walks the DAG — the reason this is one pane and not a tab beside the
		 *  run. `dbt-core-1x` reports per node live; the other engines settle every
		 *  relation from the result at the end. */
		testJobId,
		testRunning = false,
		testResult,
		/** The node picked on the canvas, owned by the parent: it decides what the
		 *  bottom section shows, and closing that has to clear it. Reported rather
		 *  than bound — the provenance travels with it, since resolving that needs
		 *  the graph response, which is this component's. */
		selection,
		onSelect
	}: {
		workspace: string | undefined
		scriptPath: string
		descriptor: string
		modules: Record<string, ScriptModule> | undefined
		args: Record<string, unknown> | undefined
		tag?: string
		timeout?: number
		deployedHash?: string | number
		testJobId?: string
		testRunning?: boolean
		testResult?: unknown
		selection?: AssetGraphNodeData | undefined
		onSelect?: (
			selection: AssetGraphNodeData | undefined,
			dbt: DbtAssetProvenance | undefined,
			/** The project this node was parsed from, when that was the editor's
			 *  buffer rather than a deployed version — as submitted, not as the
			 *  editor holds it now. Sent with the selection rather than exposed on
			 *  its own so it can never disagree with the SQL the parent shows. */
			buffer: DbtPreviewBuffer | undefined
		) => void
	} = $props()

	/** The last parse of the buffer. Component state on purpose: it describes a
	 *  buffer, and a reload has a different one. */
	let refreshJob = $state<string | undefined>(undefined)
	let refreshing = $state(false)
	// The project the pinned graph was parsed FROM, kept as submitted. Every node
	// on screen describes this snapshot, so anything run against those nodes has
	// to run against it too — reading the live buffer instead would preview one
	// project under another one's SQL from the next keystroke onward.
	let parsedBuffer = $state<DbtPreviewBuffer | undefined>(undefined)
	let refreshError = $state<{ message: string; job?: string } | undefined>(undefined)
	/** A parse still running well past the point one normally takes, so its job is
	 *  reachable while it waits. Not an error — it may still land. */
	let refreshPending = $state<string | undefined>(undefined)

	let raw = $state<AssetGraphResponse | undefined>(undefined)
	let loading = $state(true)
	let failed = $state(false)

	// The graph endpoint is folder-scoped; a script outside `f/` has no folder and
	// falls back to the whole workspace, which the scoping below narrows anyway.
	let folder = $derived(scriptPath.startsWith('f/') ? scriptPath.split('/')[1] : undefined)
	let graphKey = $derived(
		`${workspace ?? ''}|${folder ?? ''}|${deployedHash ?? ''}|${refreshJob ?? ''}`
	)

	// Two loads can overlap — a refresh landing while the deployed one is still in
	// flight — and nothing orders two requests to one endpoint. The older answers
	// with the deployed graph, so landing second it would replace the parse the
	// user just asked for.
	let graphSeq = 0
	let destroyed = false
	onDestroy(() => (destroyed = true))

	async function load() {
		if (!workspace) return
		const seq = ++graphSeq
		const current = () => seq === graphSeq && !destroyed
		try {
			const params = new URLSearchParams({ asset_kinds: 'dbt' })
			if (folder) params.set('folder', folder)
			// Through the parse JOB when there is one: its graph belongs to that job
			// and is reachable no other way. Otherwise the deployed version's, by
			// hash, so an editor open on an older version draws that one.
			const path = refreshJob
				? `/w/${workspace}/jobs/dbt_graph/${refreshJob}`
				: `/w/${workspace}/assets/graph`
			if (!refreshJob && deployedHash != undefined) {
				params.set('dbt_script_hash', String(deployedHash))
			}
			const res = await fetch(`${OpenAPI.BASE ?? ''}${path}?${params}`, {
				credentials: 'include'
			})
			if (!current()) return
			if (!res.ok) {
				failed = true
				return
			}
			const body = (await res.json()) as AssetGraphResponse
			if (!current()) return
			raw = body
			failed = false
		} catch {
			if (current()) failed = true
		} finally {
			if (current()) loading = false
		}
	}

	$effect(() => {
		void graphKey
		loading = true
		failed = false
		raw = undefined
		select(undefined)
		void load()
	})

	/** Run a parse-only job over the buffer and pin the panel to its graph.
	 *
	 *  Its own submission rather than the editor's job loader: a refresh and a
	 *  build are independent, and routing both through one would have each evict
	 *  the other's job. */
	export async function refresh() {
		if (refreshing || !workspace) return
		refreshError = undefined
		refreshPending = undefined
		let id: string | undefined
		// Read once, and before the button is disabled: the editor keeps changing
		// underneath, and everything the resulting graph is used for has to name the
		// project it actually described.
		const submitted: DbtPreviewBuffer = {
			content: descriptor,
			// The whole bundle, always: dbt resolves `ref()` project-wide and cannot
			// parse one file of it.
			modules: modules ? { ...modules } : undefined,
			tag,
			timeout,
			// The form's `vars` come along, and only those: they steer `enabled`,
			// schemas, aliases and materializations, so a parse without them reports
			// a different project than the run would build. `select` and the rest
			// belong to a command that builds. Deep-copied, because the form keeps
			// being edited and this has to stay the parse's own inputs.
			args: JSON.parse(
				JSON.stringify({
					...(args ?? {}),
					command: { label: 'parse', vars: (args?.command as any)?.vars ?? {} }
				})
			)
		}
		refreshing = true
		try {
			id = await JobService.runScriptPreview({
				workspace,
				timeout: submitted.timeout,
				requestBody: {
					path: scriptPath,
					content: submitted.content,
					language: 'dbt',
					tag: submitted.tag,
					modules: submitted.modules,
					args: submitted.args ?? {}
				}
			})
			// Polled until the JOB ends, with no window of its own: a cold worker
			// provisions an engine and runs `dbt deps` before dbt starts, which
			// outlasts any bound worth hard-coding, and giving up early strands a
			// parse that then succeeds where nothing can pin to it.
			//
			// A job's own timeout only starts once a worker takes it, so this does
			// NOT terminate on its own for one nothing serves — `tag` is forwarded
			// precisely so a project can name a worker pool, and a pool with no
			// worker leaves it queued. Hence the notice below rather than a bound:
			// waiting stays correct, and the job stays reachable while it waits.
			//
			// Backed off, because most of that wait is not the parse; and tolerant
			// of a failed poll, because one lost request must not abandon a job
			// that is still running — but only of a few in a row, since an expired
			// session answers the same way forever.
			let delay = 1000
			let failures = 0
			const slowAt = Date.now() + 60_000
			while (!destroyed) {
				await new Promise((r) => setTimeout(r, delay))
				if (destroyed) return
				let done: Awaited<ReturnType<typeof JobService.getCompletedJobResultMaybe>>
				try {
					done = await JobService.getCompletedJobResultMaybe({ workspace, id })
					failures = 0
				} catch (e) {
					if (++failures >= 10) {
						refreshError = {
							message: `Lost track of the parse: ${e instanceof Error ? e.message : String(e)}`,
							job: id
						}
						return
					}
					delay = Math.min(delay * 1.5, 5000)
					continue
				}
				if (!done.completed) {
					// Non-terminal, and the only thing that puts the job on screen
					// while it runs: the header shows a disabled spinner and nothing
					// else, so a parse that never starts would otherwise leave no link,
					// no reason and nothing to do but reload — which discards the pin.
					if (Date.now() > slowAt) refreshPending = id
					delay = Math.min(delay * 1.5, 5000)
					continue
				}
				if (!done.success) {
					// A parse renders `profiles.yml` before it runs, so a project whose
					// warehouse does not resolve fails here exactly as a run would. The
					// job carries the reason; the panel says where to read it.
					refreshError = { message: parseFailure(done.result), job: id }
					return
				}
				refreshJob = id
				parsedBuffer = submitted
				return
			}
		} catch (e) {
			refreshError = { message: e instanceof Error ? e.message : String(e), job: id }
		} finally {
			refreshing = false
			refreshPending = undefined
		}
	}

	/** dbt's own message out of the failed job's result, which is where the
	 *  useful half is — "Could not find profile named 'x'" rather than "the job
	 *  failed". */
	function parseFailure(result: unknown): string {
		const err = (result as { error?: { message?: string } } | undefined)?.error?.message
		if (typeof err !== 'string' || !err.trim()) return 'The parse job failed.'
		// dbt is verbose on failure and the panel is narrow; the head of it names
		// the problem and the job page has the rest.
		const trimmed = err.trim()
		return trimmed.length > 400 ? `${trimmed.slice(0, 400)}…` : trimmed
	}

	// This project's models, and whatever else in the workspace reads them. The
	// second half is why the editor draws from the shared asset graph rather than
	// from a manifest of its own: a native script consuming one of these marts
	// belongs on the same canvas as the model that writes it.
	let graph = $derived.by(() => {
		if (!raw) return undefined
		// Pinned to this editor's own parse, every dbt node in the answer is this
		// project's — that graph is nothing else. Unpinned the answer is
		// FOLDER-wide, so the script's own usage rows are what say which relations
		// are its; without that a new project draws a neighbouring one's models
		// under a "never parsed" label.
		const ids =
			refreshJob && raw.dbt_snapshot_job === refreshJob
				? new Set(raw.assets.filter((a) => a.dbt).map((a) => `${a.kind}:${a.path}`))
				: new Set(
						raw.edges
							.filter((e) => e.runnable_path === scriptPath)
							.map((e) => `${e.asset_kind}:${e.asset_path}`)
					)
		const assets = raw.assets.filter((a) => a.dbt && ids.has(`${a.kind}:${a.path}`))
		if (assets.length === 0) return undefined
		const edges = raw.edges.filter(
			(e) => ids.has(`${e.asset_kind}:${e.asset_path}`) && e.runnable_path !== scriptPath
		)
		const readers = new Set(edges.map((e) => `${e.runnable_kind}:${e.runnable_path}`))
		return {
			...raw,
			assets,
			// The script's own node is dropped: the whole graph already IS this
			// project, so a node for it only adds a hub every model hangs off.
			runnables: raw.runnables.filter((r) => readers.has(`${r.usage_kind}:${r.path}`)),
			edges,
			triggers: [],
			dbt_edges: (raw.dbt_edges ?? []).filter(
				(e) => ids.has(`dbt:${e.from_asset_path}`) && ids.has(`dbt:${e.to_asset_path}`)
			)
		} as AssetGraphResponse
	})

	// The build the editor is running, drawn onto the same graph. One pane rather
	// than a tab beside it: watching the models light up IS what you want while a
	// build runs, and a tab makes that a choice between the two.
	// Deliberately NOT `$state`: the effect below increments it, and `+= 1` reads
	// it — a reactive read there would make the effect depend on its own write and
	// loop. It exists only to invalidate responses issued for an older job.
	let runGen = 0
	const runStatus = useDbtRunStatus({
		workspace: () => workspace,
		jobId: () => testJobId,
		running: () => testRunning,
		result: () => testResult,
		graph: () => graph,
		generation: () => runGen,
		destroyed: () => destroyed
	})
	// A new job's colours are not the previous one's, and a finished run settles
	// itself from its own result.
	$effect(() => {
		void testJobId
		runGen += 1
		runStatus.reset()
	})
	$effect(() => {
		if (!testRunning || !testJobId) return
		void runStatus.load()
		const t = setInterval(() => void runStatus.load(), 2000)
		return () => clearInterval(t)
	})
	// Once, on the way out: the last events can land after the run ends, and a
	// run that produced no result at all is coloured from these alone.
	$effect(() => {
		if (testRunning || !testJobId) return
		void runStatus.load()
	})

	// Whether what is drawn is the buffer's graph. Not "a refresh has run": a pin
	// that did not resolve falls back to the deployed graph, and everything keyed
	// on this — the label, and which project a row preview runs — must follow the
	// graph that actually came back.
	let editorParsed = $derived(refreshJob != undefined && raw?.dbt_snapshot_job === refreshJob)

	// `untrack`, because the effect that reloads the graph clears the selection
	// through here: reading the graph to describe a selection would subscribe that
	// effect to the very state its own fetch writes, and it would reload forever.
	function select(sel: AssetGraphNodeData | undefined) {
		untrack(() =>
			onSelect?.(
				sel,
				sel?.kind === 'asset'
					? graph?.assets.find((a) => a.kind === sel.asset_kind && a.path === sel.path)?.dbt
					: undefined,
				editorParsed ? parsedBuffer : undefined
			)
		)
	}

	let modelCount = $derived(
		graph?.assets.filter((a) => a.dbt && a.dbt.resource_type !== 'source').length ?? 0
	)

	// Which graph is on screen, said plainly. The buffer's and the deploy's are
	// drawn identically, so leaving it unlabelled would move the ambiguity the
	// explicit refresh removes straight into the editor.
	let provenance = $derived.by(() => {
		if (editorParsed) {
			// Time alone: a refresh is something you did minutes ago in this
			// session, so the date is noise and the seconds are worse than noise.
			const at = raw?.dbt_graph_ingested_at
			return at
				? `parsed from the editor at ${displayDate(at, false, false)}`
				: 'parsed from the editor'
		}
		// Nothing has come back yet, so there is no provenance to state. Said
		// plainly rather than left to the branches below, which would read the
		// absent graph as a parse that failed to load.
		if (loading) return 'loading…'
		// A refresh landed but its graph is not what came back — the pin did not
		// resolve to it. Saying which graph IS on screen beats guessing why.
		if (refreshJob) return 'last parse could not be loaded — showing the deployed graph'
		if (deployedHash != undefined) return 'as of last deploy'
		return 'never parsed'
	})

</script>

<div class="flex flex-col h-full min-h-0">
	<div class="shrink-0 flex items-center gap-2 px-2 py-1 border-b bg-surface-secondary text-2xs">
		<span class="text-secondary truncate" title="Where the models on screen came from">
			{provenance}
		</span>
		{#if modelCount > 0}
			<span class="text-tertiary shrink-0">· {modelCount} models</span>
		{/if}
		<div class="ml-auto shrink-0">
			<Button
				unifiedSize="2xs"
				variant="subtle"
				disabled={refreshing}
				startIcon={{
					icon: refreshing ? Loader2 : RefreshCw,
					classes: refreshing ? 'animate-spin' : undefined
				}}
				on:click={refresh}
				title="Run `dbt parse` over the project as it is here and redraw the graph"
			>
				{refreshing ? 'Parsing' : 'Refresh models'}
			</Button>
		</div>
	</div>

	{#if refreshPending}
		<div class="shrink-0 px-2 py-1.5 border-b text-2xs text-secondary">
			Still parsing. A cold worker provisions the dbt engine before it starts; a project
			pinned to a worker tag nothing serves waits here indefinitely.
			<a
				class="text-blue-500 hover:underline"
				href="{base}/run/{refreshPending}?workspace={workspace}"
				target="_blank"
				rel="noreferrer">Open the parse job</a
			>
		</div>
	{/if}

	{#if refreshError}
		<div class="shrink-0 px-2 py-1.5 border-b text-2xs text-secondary">
			<div class="font-semibold text-primary mb-0.5">The parse failed</div>
			<pre class="whitespace-pre-wrap font-mono text-3xs">{refreshError.message}</pre>
			{#if refreshError.job}
				<a
					class="text-blue-500 hover:underline"
					href="{base}/run/{refreshError.job}?workspace={workspace}"
					target="_blank"
					rel="noreferrer">Open the parse job</a
				>
			{/if}
		</div>
	{/if}

	{#if loading}
		<div class="flex items-center gap-2 text-xs text-secondary p-3">
			<Loader2 class="animate-spin" size={14} /> Loading the model graph
		</div>
	{:else if failed}
		<div class="text-xs text-secondary p-3">Could not load the model graph.</div>
	{:else if !graph}
		<div class="text-xs text-secondary p-3 flex flex-col gap-2">
			{#if refreshJob || deployedHash != undefined}
				<span>
					This project has no models in the asset graph. A project that brings its own
					<span class="font-mono">profiles.yml</span> without naming a
					<span class="font-mono">profile.warehouse</span> has no warehouse identity to key
					<span class="font-mono">dbt://</span> assets on, so its relations cannot be drawn.
				</span>
			{:else}
				<span>
					Nothing has parsed this project yet. <span class="font-mono">Refresh models</span> runs
					<span class="font-mono">dbt parse</span> over the files as they are here and draws what dbt
					reports — deploying does the same.
				</span>
				<span class="text-tertiary">
					It needs a warehouse it can reach: the profile is rendered before dbt runs, so a project
					whose <span class="font-mono">profile.warehouse</span> is not configured under Settings → dbt
					fails the parse the way it would fail a run.
				</span>
			{/if}
		</div>
	{:else}
		<div class="flex-1 min-h-0">
			<AssetGraphCanvas
				{graph}
				{selection}
				assetRunStatus={runStatus.status}
				onselect={select}
				showMinimap={false}
				scrollZoom={false}
			/>
		</div>
	{/if}
</div>
