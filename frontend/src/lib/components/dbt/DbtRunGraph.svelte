<script lang="ts">
	// The models this run touches, as the graph, on the page you land on when you
	// click a running job. The live per-model state the worker records is what
	// makes it worth having here: nodes go amber then green as dbt walks the DAG,
	// where the alternative is reading `N of M OK created` out of the log.
	import { onDestroy, untrack } from 'svelte'
	import { OpenAPI, JobService } from '$lib/gen'
	import { nodeSelector, parseDbtRun, splitRelation, splitUniqueId } from './parseDbtRun'
	import { Button } from '$lib/components/common'
	import { ClipboardCopy, Code2, RefreshCw, TableProperties } from 'lucide-svelte'
	import { copyToClipboard } from '$lib/utils'
	import { isWindmillTooBigObject } from '$lib/components/job_args'
	import { previewDbtRows } from './previewRows'
	import { workspaceStore } from '$lib/stores'
	import { appendViewToken } from '$lib/viewToken'
	import AssetGraphCanvas from '$lib/components/assets/AssetGraph/AssetGraphCanvas.svelte'
	import type { AssetGraphResponse } from '$lib/components/assets/AssetGraph/types'
	import { useDbtRunStatus } from './runStatus.svelte'
	import { Loader2 } from 'lucide-svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import type { AssetGraphNodeData } from '$lib/components/assets/AssetGraph/types'

	let {
		scriptPath,
		jobId,
		running = false,
		result,
		scriptHash,
		runArgs,
		canResume = false,
		onResume,
		fill = false
	}: {
		scriptPath: string
		/** The run whose per-model progress to show. dbt records a state per
		 *  relation as it walks the DAG; without this the graph can only show
		 *  what a relation IS, not what this run is doing to it. */
		jobId?: string
		// While the job is in flight the graph is re-fetched, because the per-model
		// materialization rows the worker writes are what move the nodes.
		running?: boolean
		/** The finished job's result. It carries a status per dbt node, which is
		 *  what a completed run is coloured from. */
		result?: unknown
		/** The script version this job ran. The graph is stored per deployed
		 *  version, so passing it renders the project as it was — the models, SQL
		 *  and `ref()` lineage of that deploy, not of today's. */
		scriptHash?: string | number
		/** The arguments this run was invoked with. A preview is a `dbt show` of
		 *  the same project, so it needs them: a descriptor with a required
		 *  `{{ }}` var is refused without them, and an overridden one would
		 *  otherwise query a different relation than the page is showing. */
		runArgs?: Record<string, unknown>
		/** Whether a `dbt retry` by this caller would resume THIS run. One failure
		 *  is saved per script per principal, so a later run of the same script
		 *  takes the state over and the worker refuses a resume aimed here. */
		/** Fill the container instead of standing at a fixed height in a page's
		 *  flow. A pane that IS the graph gives it the room; a page section that
		 *  merely contains one does not. */
		fill?: boolean
		canResume?: boolean
		/** Submits that retry. The caller owns it so the run's own arguments and
		 *  resolved tag come along: worker tags, debounce and concurrency keys are
		 *  interpolated from the submitted arguments at enqueue, long before the
		 *  worker restores the ones being resumed. */
		onResume?: () => Promise<unknown> | void
	} = $props()

	// The graph endpoint is folder-scoped; a script outside `f/` has no folder and
	// falls back to the whole workspace, which the filter below narrows anyway.
	let folder = $derived(scriptPath.startsWith('f/') ? scriptPath.split('/')[1] : undefined)
	// Read inside `load` and listed here so a change of version refetches.
	let graphKey = $derived(`${folder ?? ''}|${scriptHash ?? ''}|${jobId ?? ''}`)

	let raw = $state<AssetGraphResponse | undefined>(undefined)
	let loading = $state(true)
	let failed = $state(false)

	// Two loads of the SAME run overlap — the poll's parting one and the
	// finished-run effect's — and nothing orders two requests to one endpoint. The
	// older answers with the DEPLOYED graph, so landing second it replaces the run's
	// own snapshot with nothing left to fetch again and correct it.
	let graphSeq = 0
	async function load() {
		const ws = $workspaceStore
		if (!ws) return
		const gen = runGen
		const seq = ++graphSeq
		const current = () => gen === runGen && seq === graphSeq
		try {
			const params = new URLSearchParams({ asset_kinds: 'dbt' })
			if (folder) params.set('folder', folder)
			if (scriptHash != undefined) params.set('dbt_script_hash', String(scriptHash))
			// Through the JOB when there is one: `/assets/graph` answers the
			// workspace-wide question and cannot pin to a run.
			const path = jobId != undefined ? `/w/${ws}/jobs/dbt_graph/${jobId}` : `/w/${ws}/assets/graph`
			// `appendViewToken`, because this is a raw `fetch`: the interceptor that
			// adds `X-View-Token` is installed on the generated client only, and a
			// shared run page carries its access in that token.
			const res = await fetch(appendViewToken(`${OpenAPI.BASE ?? ''}${path}?${params}`), {
				credentials: 'include'
			})
			// The page may have moved to another run while this was in flight, and
			// assigning that answer would leave the previous run's graph — or its
			// loading state — on screen with nothing to correct it. A newer request
			// for the SAME run supersedes this one just as completely.
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

	// `asset:<kind>:<path>` -> what this run is doing to that relation: the id shape
	// the canvas builds its nodes with, where a bare `kind:path` looks right and
	// silently never matches. A retry rewrites the same rows, so a failed node
	// returns to `running` and on to its new outcome by itself.
	// Shared with the editor's graph, so "what colour is this model right now" has
	// one answer. `generation` is this page's own: a response that outlives a
	// navigation between runs would colour the next one's models.
	const runStatus = useDbtRunStatus({
		workspace: () => $workspaceStore,
		jobId: () => jobId,
		running: () => running,
		result: () => result,
		graph: () => graph,
		generation: () => runGen,
		destroyed: () => destroyed
	})
	const loadProgress = () => runStatus.load()

	let timer: ReturnType<typeof setInterval> | undefined
	// Bounded so a static descriptor, which never writes a snapshot, stops
	// asking rather than polling for the length of the run.
	let graphTries = 0
	// Deliberately not `$state`: this guards an effect against its own writes, so
	// reading it must not make the effect depend on it.
	let finalLoadFor: string | undefined = undefined
	// SvelteKit reuses this component across runs, so every response that outlives
	// the navigation — graph, progress poll, preview — is checked against the
	// generation it was requested under, or the previous run's answer stays on
	// screen. Not `$state`: the effect below writes it and must not depend on it.
	let runGen = 0
	$effect(() => {
		void scriptPath
		void graphKey
		// SvelteKit reuses this component when navigating between runs, and all
		// three of these describe the PREVIOUS job: a spent retry count stops the
		// next run's snapshot poll before it starts, and stale progress colours
		// its models with another run's statuses.
		graphTries = 0
		finalLoadFor = undefined
		runStatus.reset()
		raw = undefined
		// Both are written by `load` alone and describe the run it answered for,
		// so they are as run-scoped as `raw`: left behind, the gap before the new
		// run's answer falls past `{#if loading}` to the "no models in the asset
		// graph" notice — a claim about the descriptor, on a project that is fine.
		loading = true
		failed = false
		// Previews are keyed by `unique_id` alone, which is the same string for the
		// same model in every run — so without this the next run opens showing the
		// previous one's rows, and `runPreview` treats them as cached and refuses
		// to fetch. The generation drops anything already in flight.
		runGen += 1
		selection = undefined
		previews = {}
		rowsFor = undefined
		// Only while in flight; the effect below owns a FINISHED run, since a
		// snapshot can land after the run ends. On mount both fire in the same tick,
		// so loading here too fetches the whole graph twice. Untracked: depending on
		// `running` would re-run this reset the moment the run finishes.
		if (untrack(() => running)) void load()
		void jobId
		void loadProgress()
	})
	$effect(() => {
		clearInterval(timer)
		// Read synchronously so navigating between two RUNNING jobs re-arms this:
		// everything below is read inside timer callbacks, so `running` alone would
		// leave the second run polling on the first one's cycle — already spent.
		void graphKey
		// Only while in flight, and no faster than dbt finishes a model: this is a
		// poll against the same rows the pipeline page reads, not a subscription.
		if (running) {
			// The GRAPH is refetched only until this run's own snapshot appears: a
			// dynamic descriptor re-ingests once before the build, after a clone and
			// a provision, so a fixed delay either fires too early to ever show the
			// run's models or re-sends the whole graph for the length of the run.
			timer = setInterval(() => void loadProgress(), 2000)
			// Backed off, because for a whole class of runs neither stop below is
			// reached and this walks to its cap: `dbt_snapshot_job` never matches a
			// static descriptor, and the polled status stays empty on engines emitting no node
			// events. Each try re-sends every model's SQL.
			let graphDelay = 3000
			let graphTimer: ReturnType<typeof setTimeout> | undefined
			const pollGraph = () => {
				// Two ways to be done, and the second is the common one. The
				// ingest happens BEFORE the build, so once any model reports
				// progress it has already run: a snapshot that is not here by
				// then is a snapshot this run never writes.
				if (raw?.dbt_snapshot_job === jobId || runStatus.status.size > 0 || graphTries >= 12) {
					// One last load on the way out. Progress proves the ingest
					// happened; it does not prove the previous tick SAW it, and
					// dbt's compile window is usually wider than one tick.
					if (raw?.dbt_snapshot_job !== jobId) void load()
					return
				}
				graphTries += 1
				void load()
				graphDelay = Math.min(graphDelay * 1.6, 30000)
				graphTimer = setTimeout(pollGraph, graphDelay)
			}
			graphTimer = setTimeout(pollGraph, graphDelay)
			return () => {
				clearTimeout(graphTimer)
				clearInterval(timer)
			}
		}
		// One unconditional load once the run has finished: the poll above gives up
		// after a bounded number of tries, which provisioning plus `dbt deps` can
		// outlast on a cold worker. Without this a snapshot written late is never
		// displayed and the page keeps the deployed fallback for good.
		else if (finalLoadFor !== (jobId ?? '')) {
			// Once per finished job, not per response: `load()` assigns `raw`, which
			// recomputes the settled status, which re-enters this effect and refetches the
			// whole graph for as long as the page is open. A plain variable, so
			// reading it adds no dependency.
			finalLoadFor = jobId ?? ''
			void load()
			// The settled status colours a finished run with no request. The poll is the
			// fallback for one that produced no `run_results.json` at all —
			// cancelled or killed — whose relations the worker settles in the
			// table instead.
			if (!untrack(() => runStatus.isSettled)) void loadProgress()
		}
		return () => clearInterval(timer)
	})
	// A version pinned with no graph is usually a deploy that has not finished
	// ingesting yet, so look again a few times before the empty state stands.
	// Self-scheduling rather than effect-driven: a retry that finds the same empty
	// answer changes no state an effect reads, so it would never re-arm. Bounded
	// and backed off — a project with no warehouse identity never fills in, and
	// must not poll for the life of the page.
	$effect(() => {
		void graphKey
		if (running || scriptHash == undefined || jobId != undefined) return
		let tries = 0
		let timer: ReturnType<typeof setTimeout> | undefined
		const again = () => {
			if (destroyed || untrack(() => graph) != undefined || tries >= 5) return
			tries += 1
			void load().then(() => {
				timer = setTimeout(again, 2000 * tries)
			})
		}
		timer = setTimeout(again, 2000)
		return () => clearTimeout(timer)
	})

	// Read by the preview poll, which outlives the component otherwise: its own
	// loop is what has to stop, since there is no interval to clear.
	let destroyed = false
	onDestroy(() => {
		destroyed = true
		clearInterval(timer)
	})

	// Parsed once: five derivations below read it, and `result` is the whole
	// `run_results.json` the job returned.
	let run = $derived(parseDbtRun(result))

	// A failed run's obvious next action is "Run again", which prefills the
	// arguments it just ran — rebuilding the whole project. `dbt retry` redoes the
	// failed and skipped nodes alone, and nothing on the page says so.
	let resumable = $derived(
		canResume && !running && ((run?.totals?.error ?? 0) > 0 || (run?.totals?.skipped ?? 0) > 0)
	)
	let resuming = $state(false)

	async function resume() {
		if (resuming) return
		resuming = true
		try {
			await onResume?.()
		} finally {
			resuming = false
		}
	}

	// A test is not a relation, so a run selecting tests alone has an empty graph
	// and a healthy one — where the empty state otherwise blames the descriptor's
	// warehouse identity. Both kinds: dbt spells a unit test `unit_test.<pkg>.…`.
	let ranTestsOnly = $derived(
		!!run?.nodes?.length &&
			run.nodes.every((n) => ['test', 'unit_test'].includes(splitUniqueId(n.unique_id).kind))
	)

	// A run page is about one script, so the graph is the relations it reads and
	// writes — a folder's other projects are noise here, and so is a node for the
	// script itself: on the pipeline page that node distinguishes one project
	// from another, but here the whole graph already IS that project.
	let scoped = $derived.by(() => {
		if (!raw) return undefined
		// Asked for a specific version, the response IS that version's project.
		// Scoping by usage edges would undo that: those rows are path-keyed and
		// describe the current deploy, so a model this version had and a later one
		// dropped has no edge to be found by.
		const assetIds = scriptHash
			? new Set(raw.assets.filter((a) => a.dbt).map((a) => `${a.kind}:${a.path}`))
			: new Set(
					raw.edges
						.filter((e) => e.runnable_path === scriptPath)
						.map((e) => `${e.asset_kind}:${e.asset_path}`)
				)
		if (assetIds.size === 0) return undefined
		return {
			...raw,
			runnables: [],
			assets: raw.assets.filter((a) => assetIds.has(`${a.kind}:${a.path}`)),
			edges: [],
			triggers: [],
			// `ref()` lineage between two of this project's own models is the shape
			// of the dbt DAG, so it is the one edge set worth keeping whole.
			dbt_edges: (raw.dbt_edges ?? []).filter(
				(e) => assetIds.has(`dbt:${e.from_asset_path}`) && assetIds.has(`dbt:${e.to_asset_path}`)
			)
		} as AssetGraphResponse
	})

	// A FINISHED run is narrowed to what it named: the response is the VERSION's
	// graph, so a model this run never built would appear as though it had. Sources
	// stay — dbt lists none in `run_results.json`, but the run read them. (No
	// `resolveGraph`: it merges drafts and editor buffers a run page has neither of.)
	let historical = $derived.by(() => {
		if (running || !scoped) return undefined
		if (!run?.nodes?.length) return undefined
		const ran = new Set(run.nodes.map((n) => n.unique_id))
		// dbt ids are `<resource_type>.<package>.<name>`, so the packages this run
		// named are the projects it could speak for.
		const pkg = (id: string) => id.split('.')[1]
		const ranPackages = new Set(run.nodes.map((n) => pkg(n.unique_id)))
		const keep = (a: (typeof scoped.assets)[number]) =>
			a.dbt == undefined ||
			a.dbt.resource_type === 'source' ||
			ran.has(a.dbt.unique_id) ||
			// Provenance keeps one winner per relation, so a relation two projects
			// write may carry the OTHER project's model — and this run cannot be
			// judged against an id that was never its own. Dropping it would make the
			// run look like it built fewer models than it did.
			!ranPackages.has(pkg(a.dbt.unique_id))
		const assets = scoped.assets.filter(keep)
		if (assets.length === scoped.assets.length) return undefined
		const ids = new Set(assets.map((a) => `${a.kind}:${a.path}`))
		return {
			...scoped,
			assets,
			dbt_edges: (scoped.dbt_edges ?? []).filter(
				(e) => ids.has(`dbt:${e.from_asset_path}`) && ids.has(`dbt:${e.to_asset_path}`)
			)
		} as AssetGraphResponse
	})

	let graph = $derived(historical ?? scoped)

	// Models whose id survived but whose RELATION changed — an alias, schema or
	// database edit. The graph on screen is the version's as it stands now, so a
	// node can name a relation this run did not write, and saying so beats
	// asserting the run materialized a table that did not exist then.
	let relationDrift = $derived.by(() => {
		if (running || !graph) return 0
		if (!run?.nodes?.length) return 0
		// Schema AND name: the move most likely being reported, a profile repointed
		// at another schema, leaves every model's name where it was. An asset path is
		// `<resource>/<schema>/<name>`, whose schema segment is `<database>.<schema>`
		// only when the model overrode the target's database.
		const byId = new Map(
			graph.assets
				.filter((a) => a.dbt?.unique_id)
				.map((a) => [a.dbt!.unique_id, a.path.split('/').slice(-2)])
		)
		return run.nodes.filter((n) => {
			if (!n.relation_name) return false
			const now = byId.get(n.unique_id)
			if (now == undefined || now.length < 2) return false
			const [nowSchema, nowName] = now
			const parts = splitRelation(n.relation_name)
			const name = parts.at(-1) ?? ''
			const schema = parts.at(-2) ?? ''
			const qualified = parts.length > 2 ? `${parts.at(-3)}.${schema}` : schema
			// Either spelling counts as unmoved: an unqualified segment names the
			// target's own database, so demanding the qualified form reports a move on
			// every node. Lower-cased because `relation_name` is the warehouse's own —
			// Snowflake's is upper, and comparing raw looks like a rename.
			const here = nowSchema.toLowerCase()
			return (
				nowName.toLowerCase() !== name.toLowerCase() ||
				(here !== schema.toLowerCase() && here !== qualified.toLowerCase())
			)
		}).length
	})

	// Models this run built that the graph on screen no longer has under the same
	// id — renamed or deleted since, or dropped from a version graph rewritten
	// after this run. They cannot be drawn, so the count is stated rather than
	// silently missing.
	let goneSinceRun = $derived.by(() => {
		if (running || !scoped) return 0
		if (!run?.nodes?.length) return 0
		const known = new Set(
			scoped.assets.map((a) => a.dbt?.unique_id).filter((u): u is string => u != undefined)
		)
		// Only relations whose own project still lacks the id count as gone: an id
		// missing because another project won the provenance is not a deletion.
		const knownPackages = new Set(
			[...known].map((u) => u.split('.')[1]).filter((p) => p != undefined)
		)
		return run.nodes.filter(
			(n) =>
				!known.has(n.unique_id) &&
				knownPackages.has(n.unique_id.split('.')[1]) &&
				/^(model|seed|snapshot)\./.test(n.unique_id)
		).length
	})

	// A finished run's own output, joined on dbt's `unique_id` — what both sides
	// carry, where the relation name would redo the worker's path derivation. The
	// per-relation state table holds ONE row per relation, whichever job wrote it
	// last, so reading THAT would show an old run a later one's models.
	let assetRunStatus = $derived(runStatus.status)

	// The transform behind the selected relation. dbt's own DAG node is the model
	// — the SQL and the table it writes are one thing — so a graph of relations
	// alone leaves out what a reader came to see.
	let selection = $state<AssetGraphNodeData | undefined>(undefined)
	let selectedDbt = $derived.by(() => {
		const sel = selection
		if (sel?.kind !== 'asset') return undefined
		const dbt = graph?.assets.find((a) => a.kind === sel.asset_kind && a.path === sel.path)?.dbt
		if (!dbt) return undefined
		// Provenance keeps one winner per relation workspace-wide, so a relation two
		// projects materialize carries one project's node, whose SQL is not this
		// run's. Sources are exempt: a run executes none, so none appear in
		// `run_results.json`.
		if (
			dbt.resource_type !== 'source' &&
			run?.nodes?.length &&
			!run.nodes.some((n) => n.unique_id === dbt.unique_id)
		) {
			return undefined
		}
		return dbt
	})

	// `dbt show` against the selected model, run as an ordinary job — same
	// authorization, isolation and cancellation as any other. Explicit rather
	// than on-select: each preview costs a worker slot and a few seconds of
	// engine start-up, so it must be something the reader asked for.
	type Preview =
		| { rows: Record<string, unknown>[]; tookMs: number; node?: string }
		| { error: string }
		| { pending: true }
	// Cached per model id, so flipping between two nodes does not re-run a job
	// that costs a worker slot and an engine start-up. A preview also keeps
	// running when the selection moves — the result lands in the cache and is
	// there when the reader comes back.
	let previews = $state<Record<string, Preview>>({})
	// Read under the same key `runPreview` writes: the model AND the arguments it
	// ran with, so an edited var shows its own rows rather than the last ones.
	let preview = $derived(
		selectedDbt ? previews[`${selectedDbt.unique_id}|${JSON.stringify(runArgs ?? {})}`] : undefined
	)
	let previewing = $derived(preview != undefined && 'pending' in preview)
	// Which model's rows are on display. Keyed by id rather than a boolean so
	// moving the selection goes back to SQL without anything having to reset it.
	let rowsFor = $state<string | undefined>(undefined)
	let showRows = $derived(
		selectedDbt != undefined && rowsFor?.startsWith(`${selectedDbt.unique_id}|`) === true
	)

	// A column can hold an array or a JSON object, whose default stringification
	// is `[object Object]` — which tells the reader nothing about the value.
	function cellText(v: unknown): string {
		if (v == undefined) return ''
		return typeof v === 'object' ? JSON.stringify(v) : String(v)
	}

	async function runPreview() {
		const ws = $workspaceStore
		const dbt = selectedDbt
		if (!ws || !dbt) return
		// Keyed on the ARGUMENTS too, not the model alone: on the script page the
		// form above feeds this, so a var edited between two clicks previews a
		// different query — and a cache that ignored it would return the old rows
		// as though nothing had changed.
		const key = `${dbt.unique_id}|${JSON.stringify(runArgs ?? {})}`
		// Displaying the rows and running the job are separate: coming back from
		// the SQL to a result already in hand must not spend another worker slot.
		rowsFor = key
		// A cached FAILURE must not be cached: it would leave the button dead for
		// that model until reload. Only a result or an in-flight run blocks a
		// re-run.
		const cached = previews[key]
		if (cached != undefined && !('error' in cached)) return
		const gen = runGen
		previews = { ...previews, [key]: { pending: true } }
		// The run's own arguments, so a required `{{ }}` var resolves and an
		// overridden one points at the relation on screen. The result's copy wins:
		// a `dbt retry` job's own arguments name the run it resumed, while what it
		// ran with is the restored ones published here.
		//
		// Fetched when they were too big to inline: while the run is still going
		// there is no result to fall back on, so the placeholder would submit a
		// preview with none of the descriptor's required vars.
		let submitted: Record<string, unknown> = runArgs ?? {}
		if (jobId && isWindmillTooBigObject(submitted)) {
			try {
				submitted = ((await JobService.getJobArgs({ workspace: ws, id: jobId })) ?? {}) as Record<
					string,
					unknown
				>
			} catch {
				// Leave the placeholder: the preview may still resolve, and its own
				// failure says more than refusing to try.
			}
			if (gen !== runGen || destroyed) return
		}
		const ran = { ...submitted, ...(run?.invocation_args ?? {}) } as Record<string, any>
		const { command: _cmd, ...placeholders } = ran
		const next = await previewDbtRows({
			workspace: ws,
			scriptPath,
			// By HASH whenever the graph is pinned, so the rows are that version's.
			scriptHash,
			// Scoped to the node's package, not the bare name: a package can ship a
			// model whose name the project also uses. Derived from the node's id
			// and NEVER from `key`, which carries the run arguments that make the
			// cache entry distinct and would reach dbt as part of the selector.
			model: nodeSelector(dbt.unique_id),
			vars: ran.command?.vars ?? {},
			args: placeholders,
			stillWanted: () => gen === runGen && !destroyed
		})
		if (!next || gen !== runGen) return
		previews = { ...previews, [key]: next }
	}

	// Selected a relation this run built, but its stored provenance belongs to
	// another project that writes the same table. Saying so beats rendering
	// nothing, which reads as a dead click.
	let selectedIsForeign = $derived.by(() => {
		const sel = selection
		if (sel?.kind !== 'asset' || selectedDbt) return false
		const dbt = graph?.assets.find((a) => a.kind === sel.asset_kind && a.path === sel.path)?.dbt
		return dbt != undefined
	})

	// Fully qualified, as dbt reported it for THIS run, and verbatim — quoting
	// included. There is no table browser to link to, so this is the name to paste
	// into a SQL client, and stripping the quotes turns `"wh"."analytics.v2"."Order
	// Items"` into four parts of a name that resolves to nothing.
	let selectedRelation = $derived.by(() => {
		const sel = selection
		if (sel?.kind !== 'asset' || !selectedDbt) return undefined
		return run?.nodes?.find((n) => n.unique_id === selectedDbt!.unique_id)?.relation_name
	})
</script>

{#snippet sqlPane()}
	{#if selectedIsForeign}
		<div class="border-t px-2 py-1.5 text-2xs text-secondary">
			Another dbt project in this workspace also materializes this relation, and the graph keeps one
			project's model per relation — so the SQL shown here would not be this run's. Open that
			project's own run to see it.
		</div>
	{:else if selectedDbt?.raw_code}
		<div class="border-t flex flex-col min-h-0 max-h-72">
			<div
				class="shrink-0 flex items-center gap-2 px-2 py-1 text-2xs border-b bg-surface-secondary text-secondary"
			>
				<span class="font-mono truncate"
					>{selectedDbt.original_file_path ?? selectedDbt.unique_id}</span
				>
				{#if selectedDbt.materialized}
					<span class="shrink-0 opacity-70">{selectedDbt.materialized}</span>
				{/if}
				<!-- `dbt show` is a SELECT against the node's own relation, and the
				     worker intersects the selector with `resource_type:model`, so
				     offering it on a seed or a snapshot only ever produces a failed
				     job. Their SQL still shows: a snapshot has a body worth reading. -->
				{#if selectedDbt.resource_type === 'model'}
					{#if showRows && preview && !('error' in preview)}
						<Button
							unifiedSize="2xs"
							variant="subtle"
							startIcon={{ icon: Code2 }}
							on:click={() => (rowsFor = undefined)}
							title="Show the model's SQL"
						>
							SQL
						</Button>
					{:else}
						<Button
							unifiedSize="2xs"
							variant="subtle"
							startIcon={{
								icon: previewing ? Loader2 : TableProperties,
								classes: previewing ? 'animate-spin' : undefined
							}}
							disabled={previewing}
							on:click={runPreview}
							title="Run `dbt show` against this model and display the rows"
						>
							{previewing ? 'Previewing…' : 'Preview rows'}
						</Button>
					{/if}
				{/if}
				{#if selectedRelation}
					<Button
						unifiedSize="2xs"
						variant="subtle"
						startIcon={{ icon: ClipboardCopy }}
						on:click={() => copyToClipboard(selectedRelation)}
						title="Copy the fully-qualified relation name"
					>
						<span class="font-mono truncate max-w-56">{selectedRelation}</span>
					</Button>
				{/if}
				<!-- The copy taken at deploy, so editing it here would change nothing the
				     next run reads. The file itself is a module of the script — editable
				     in the script editor, or in the project and pushed. -->
				<span class="ml-auto shrink-0 opacity-70" title={selectedDbt.original_file_path}>
					read-only · edit in the script
				</span>
			</div>
			<div class="flex-1 min-h-0 overflow-auto">
				{#if showRows && preview}
					{#if 'error' in preview}
						<div class="p-2 text-2xs text-secondary">{preview.error}</div>
						<HighlightCode language="sql" code={selectedDbt.raw_code} />
					{:else if 'pending' in preview}
						<div class="flex items-center gap-2 p-2 text-2xs text-secondary">
							<Loader2 size={12} class="animate-spin" />
							Running `dbt show` — this is a job, so it waits on a worker and the engine.
						</div>
					{:else}
						{@const cols = Object.keys(preview.rows[0] ?? {})}
						{#if cols.length === 0}
							<div class="p-2 text-2xs text-secondary"> The model returned no rows. </div>
						{:else}
							<table class="w-full text-2xs font-mono">
								<thead class="sticky top-0 bg-surface-secondary text-secondary">
									<tr>
										{#each cols as c (c)}
											<th class="text-left font-semibold px-2 py-1 border-b">{c}</th>
										{/each}
									</tr>
								</thead>
								<tbody>
									{#each preview.rows as row, i (i)}
										<tr class="border-b border-surface-selected">
											{#each cols as c (c)}
												<td class="px-2 py-0.5 truncate max-w-56">{cellText(row[c])}</td>
											{/each}
										</tr>
									{/each}
								</tbody>
							</table>
							<div class="px-2 py-1 text-3xs text-tertiary">
								{preview.rows.length} rows in {(preview.tookMs / 1000).toFixed(1)}s{preview.node
									? ` · ${preview.node}`
									: ''}
							</div>
						{/if}
					{/if}
				{:else}
					<HighlightCode language="sql" code={selectedDbt.raw_code} />
				{/if}
			</div>
		</div>
	{/if}
{/snippet}

{#if resumable}
	<div class="mb-2 px-2 py-1 text-2xs text-secondary border rounded bg-surface-secondary">
		{(run?.totals?.error ?? 0) > 0 ? `${run?.totals?.error} failed` : ''}{(run?.totals?.error ??
			0) > 0 && (run?.totals?.skipped ?? 0) > 0
			? ', '
			: ''}{(run?.totals?.skipped ?? 0) > 0 ? `${run?.totals?.skipped} skipped` : ''}. Rebuild only
		those with <span class="font-mono">dbt retry</span>, instead of the whole project.
		<span class="inline-block ml-1 align-middle">
			<Button
				unifiedSize="2xs"
				variant="subtle"
				disabled={resuming}
				startIcon={{
					icon: resuming ? Loader2 : RefreshCw,
					classes: resuming ? 'animate-spin' : undefined
				}}
				on:click={resume}>{resuming ? 'Resuming' : 'Resume this run'}</Button
			>
		</span>
	</div>
{/if}

{#if loading}
	<div class="flex items-center gap-2 text-xs text-secondary p-3">
		<Loader2 class="animate-spin" size={14} /> Loading the model graph
	</div>
{:else if failed}
	<div class="text-xs text-secondary p-3">Could not load the model graph.</div>
{:else if !graph}
	<div class="text-xs text-secondary p-3">
		{#if ranTestsOnly}
			This run selected tests alone, so it built no models. A dbt test is an assertion rather than a
			relation, so it has no node here — the models it asserts against belong to the runs that build
			them. Its results are in the table below.
		{:else}
			<!-- States the fact and lists what can produce it, rather than asserting one
			     cause. The common one is timing: a version's graph is written by its
			     DEPLOY, so the seconds between deploying and that job finishing look
			     exactly like a project with no warehouse identity. -->
			No models stored for this version of the project.
			{#if scriptHash}
				A version's graph is written by its deploy, so this is what a deploy still in
				flight looks like — it fills in when that job lands.
			{/if}
			A project that brings its own <span class="font-mono">profiles.yml</span> without naming
			a <span class="font-mono">profile.warehouse</span> also has no warehouse identity to key
			models on, and stores none.
		{/if}
	</div>
{:else}
	<div
		class="overflow-hidden flex flex-col {fill ? 'h-full' : 'border rounded'}"
	>
		{#if relationDrift > 0}
			<div class="shrink-0 px-2 py-1 text-2xs text-secondary border-b bg-surface-secondary">
				{relationDrift}
				{relationDrift === 1 ? 'model has' : 'models have'} been renamed or moved since this run —
				{relationDrift === 1 ? 'its node shows' : 'their nodes show'} today's relation, not the one this
				run wrote.
			</div>
		{/if}
		{#if goneSinceRun > 0}
			<div class="shrink-0 px-2 py-1 text-2xs text-secondary border-b bg-surface-secondary">
				{goneSinceRun}
				{goneSinceRun === 1 ? 'model' : 'models'} this run built {goneSinceRun === 1 ? 'is' : 'are'}
				no longer in the project — renamed or removed since, so
				{goneSinceRun === 1 ? 'it is' : 'they are'} not drawn.
			</div>
		{/if}
		<div class={fill ? 'flex-1 min-h-0' : 'h-80'}>
			<AssetGraphCanvas
				{graph}
				{selection}
				{assetRunStatus}
				onselect={(s) => (selection = s)}
				showMinimap={false}
				scrollZoom={false}
			/>
		</div>
		{@render sqlPane()}
	</div>
{/if}
