<script lang="ts">
	// The models this run touches, as the graph, on the page you land on when you
	// click a running job. The live per-model state the worker records is what
	// makes it worth having here: nodes go amber then green as dbt walks the DAG,
	// where the alternative is reading `N of M OK created` out of the log.
	import { onDestroy } from 'svelte'
	import { OpenAPI, JobService } from '$lib/gen'
	import { parseDbtRun, relationOutcome, splitRelation, splitUniqueId } from './parseDbtRun'
	import { Button } from '$lib/components/common'
	import { ClipboardCopy, Code2, TableProperties } from 'lucide-svelte'
	import { copyToClipboard } from '$lib/utils'
	import { workspaceStore } from '$lib/stores'
	import AssetGraphCanvas from '$lib/components/assets/AssetGraph/AssetGraphCanvas.svelte'
	import type { AssetGraphResponse, AssetRunState } from '$lib/components/assets/AssetGraph/types'
	import { Loader2 } from 'lucide-svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import type { AssetGraphNodeData } from '$lib/components/assets/AssetGraph/types'

	let {
		scriptPath,
		jobId,
		running = false,
		result,
		scriptHash
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
		 *  what a completed run is coloured from — see `settled`. */
		result?: unknown
		/** The script version this job ran. The graph is stored per deployed
		 *  version, so passing it renders the project as it was — the models, SQL
		 *  and `ref()` lineage of that deploy, not of today's. */
		scriptHash?: string | number
	} = $props()

	// The graph endpoint is folder-scoped; a script outside `f/` has no folder and
	// falls back to the whole workspace, which the filter below narrows anyway.
	let folder = $derived(scriptPath.startsWith('f/') ? scriptPath.split('/')[1] : undefined)
	// Read inside `load` and listed here so a change of version refetches.
	let graphKey = $derived(`${folder ?? ''}|${scriptHash ?? ''}|${jobId ?? ''}`)

	let raw = $state<AssetGraphResponse | undefined>(undefined)
	let loading = $state(true)
	let failed = $state(false)

	async function load() {
		const ws = $workspaceStore
		if (!ws) return
		try {
			const params = new URLSearchParams({ asset_kinds: 'table' })
			if (folder) params.set('folder', folder)
			if (scriptHash != undefined) params.set('dbt_script_hash', String(scriptHash))
			// Passed unconditionally: only a dynamic descriptor leaves a per-run
			// snapshot, and without one the pinned version's graph answers.
			if (jobId != undefined) params.set('dbt_job_id', jobId)
			const res = await fetch(`${OpenAPI.BASE ?? ''}/w/${ws}/assets/graph?${params}`, {
				credentials: 'include'
			})
			if (!res.ok) {
				failed = true
				return
			}
			raw = (await res.json()) as AssetGraphResponse
			failed = false
		} catch {
			failed = true
		} finally {
			loading = false
		}
	}

	// `asset:<kind>:<path>` -> what this run is doing to that relation. That is
	// the id shape the canvas builds its nodes with; a bare `kind:path` looks
	// right and silently never matches. Polled while the job runs; a retry
	// rewrites the same rows, so a failed node returns to `running` and on to its
	// new outcome by itself.
	let polled = $state<Map<string, AssetRunState>>(new Map())

	async function loadProgress() {
		const ws = $workspaceStore
		if (!ws || !jobId) return
		try {
			const rows = await JobService.getRunProgress({ workspace: ws, id: jobId })
			const next = new Map<string, AssetRunState>()
			for (const r of rows) {
				next.set(`asset:${r.asset_kind}:${r.asset_path}`, {
					status: r.status,
					rowCount: r.row_count
				})
			}
			polled = next
		} catch {
			// A progress hiccup must not blank the graph.
		}
	}

	let timer: ReturnType<typeof setInterval> | undefined
	$effect(() => {
		void scriptPath
		void graphKey
		void load()
		void jobId
		void loadProgress()
	})
	$effect(() => {
		clearInterval(timer)
		// Only while in flight, and no faster than dbt finishes a model: this is a
		// poll against the same rows the pipeline page reads, not a subscription.
		if (running)
			timer = setInterval(() => {
				// The graph itself only changes on a deploy; the per-relation state
				// is what moves during a run, so it is polled faster and the graph
				// refetch rides along to pick up a per-run re-ingest.
				void loadProgress()
				void load()
			}, 2000)
		// A finished run is coloured from `settled`, which needs no request. The
		// poll is the fallback for a run that never produced one — cancelled or
		// killed, where dbt wrote no `run_results.json` — whose relations the
		// worker settles in the table instead.
		else if (!settled) void loadProgress()
		return () => clearInterval(timer)
	})
	onDestroy(() => clearInterval(timer))

	// Parsed once: five derivations below read it, and `result` is the whole
	// `run_results.json` the job returned.
	let run = $derived(parseDbtRun(result))

	// A run page is about one script, so the graph is the relations it reads and
	// writes — a folder's other projects are noise here, and so is a node for the
	// script itself: on the pipeline page that node distinguishes one project
	// from another, but here the whole graph already IS that project.
	let scoped = $derived.by(() => {
		if (!raw) return undefined
		// Asked for a specific version, the response IS that version's project —
		// every dbt-owned relation in it belongs to this script. Scoping by usage
		// edges would undo that: those rows are path-keyed and describe the
		// current deploy, so a model this version had and a later one dropped has
		// no edge to be found by.
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
				(e) =>
					assetIds.has(`table:${e.from_asset_path}`) && assetIds.has(`table:${e.to_asset_path}`)
			)
		} as AssetGraphResponse
	})

	// No `resolveGraph`: that merges drafts and live editor buffers into the
	// persisted graph, and a run page has neither. The response is the graph.
	//
	// For a FINISHED run the node set is narrowed to what that run actually
	// named. `/assets/graph` is the current deploy, so without this a model added
	// after the run appears in its graph as though it had been there, and the
	// older the run the more the picture drifts. Sources are kept regardless:
	// dbt never lists them in `run_results.json` because it does not build them,
	// but they are the upstream the run read.
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
			// both write may carry the OTHER project's model. This run cannot be
			// judged against an id that was never its own — keeping it shows the
			// relation the run did write, where dropping it makes the run look like
			// it built fewer models than it did.
			!ranPackages.has(pkg(a.dbt.unique_id))
		const assets = scoped.assets.filter(keep)
		if (assets.length === scoped.assets.length) return undefined
		const ids = new Set(assets.map((a) => `${a.kind}:${a.path}`))
		return {
			...scoped,
			assets,
			dbt_edges: (scoped.dbt_edges ?? []).filter(
				(e) => ids.has(`table:${e.from_asset_path}`) && ids.has(`table:${e.to_asset_path}`)
			)
		} as AssetGraphResponse
	})

	let graph = $derived(historical ?? scoped)

	// Models whose id survived but whose RELATION changed since — an alias,
	// schema or database edit. The graph is the current deploy, so the node is
	// today's relation while the run wrote a different one; saying so beats the
	// page asserting this run materialized a table that did not exist then.
	// Detecting it needs no snapshot: the result carries the relation each node
	// actually wrote.
	let relationDrift = $derived.by(() => {
		if (running || !graph) return 0
		if (!run?.nodes?.length) return 0
		const byId = new Map(
			graph.assets
				.filter((a) => a.dbt?.unique_id)
				.map((a) => [a.dbt!.unique_id, a.path.split('/').at(-1)])
		)
		return run.nodes.filter((n) => {
			if (!n.relation_name) return false
			const now = byId.get(n.unique_id)
			if (now == undefined) return false
			// Asset paths are canonicalized to lower case, while `relation_name` is
			// the warehouse's own spelling — Snowflake returns it upper. Comparing
			// them raw makes every model of every Snowflake run look renamed.
			return now.toLowerCase() !== (splitRelation(n.relation_name).at(-1) ?? '').toLowerCase()
		}).length
	})

	// Models this run built that the project no longer has under the same id —
	// renamed or deleted since. They cannot be drawn (the graph is the current
	// deploy) so the count is stated rather than silently missing.
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

	// A finished run's own output, joined to the graph on dbt's `unique_id`.
	//
	// This is what makes an old run still render correctly. The alternative,
	// reading the per-relation state table, cannot: that table keeps ONE row per
	// relation stamped with whichever job wrote it last, so a later run silently
	// takes the earlier one's models away. The result is the run's own, is stored
	// with the job, and is deleted with it.
	//
	// `unique_id` is the join because it is what both sides already carry —
	// matching on the warehouse relation name would mean redoing the worker's
	// path derivation here.
	let settled = $derived.by(() => {
		if (running) return undefined
		if (!run?.nodes?.length || !graph) return undefined
		const assetByNode = new Map<string, string>()
		for (const a of graph.assets) {
			if (a.dbt?.unique_id) assetByNode.set(a.dbt.unique_id, `asset:${a.kind}:${a.path}`)
		}
		const out = new Map<string, AssetRunState>()
		for (const n of run.nodes) {
			const id = assetByNode.get(n.unique_id)
			const outcome = id && relationOutcome(n.status)
			// A test or an analysis matches no relation, and a skipped node says
			// nothing about one; both are left uncoloured rather than guessed at.
			if (id && outcome) out.set(id, { status: outcome, rowCount: n.rows_affected })
		}
		return out.size > 0 ? out : undefined
	})

	let assetRunStatus = $derived(settled ?? polled)

	// The transform behind the selected relation. dbt's own DAG node is the model
	// — the SQL and the table it writes are one thing — so a graph of relations
	// alone leaves out what a reader came to see.
	let selection = $state<AssetGraphNodeData | undefined>(undefined)
	let selectedDbt = $derived.by(() => {
		const sel = selection
		if (sel?.kind !== 'asset') return undefined
		const dbt = graph?.assets.find((a) => a.kind === sel.asset_kind && a.path === sel.path)?.dbt
		if (!dbt) return undefined
		// Provenance is one winner per relation across the whole workspace, so a
		// relation two projects both materialize carries only one project's node.
		// Showing that project's SQL under this run would be a different model's
		// source; when this run names the node, it is this run's.
		if (run?.nodes?.length && !run.nodes.some((n) => n.unique_id === dbt.unique_id)) {
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
	let preview = $derived(selectedDbt ? previews[selectedDbt.unique_id] : undefined)
	let previewing = $derived(preview != undefined && 'pending' in preview)
	// Which model's rows are on display. Keyed by id rather than a boolean so
	// moving the selection goes back to SQL without anything having to reset it.
	let rowsFor = $state<string | undefined>(undefined)
	let showRows = $derived(selectedDbt != undefined && rowsFor === selectedDbt.unique_id)

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
		const key = dbt.unique_id
		// Displaying the rows and running the job are separate: coming back from
		// the SQL to a result already in hand must not spend another worker slot.
		rowsFor = key
		// A cached FAILURE must not be cached: it would leave the button dead for
		// that model until reload. Only a result or an in-flight run blocks a
		// re-run.
		const cached = previews[key]
		if (cached != undefined && !('error' in cached)) return
		const startedAt = Date.now()
		previews = { ...previews, [key]: { pending: true } }
		try {
			const requestBody = {
				dbt_command: 'show',
				select: [splitUniqueId(key).name],
				limit: 25
			}
			// By HASH whenever the graph is pinned: the SQL on screen is that
			// version's, and running the deployed one would show today's rows
			// under it — or fail outright for a model since removed.
			const id = scriptHash
				? await JobService.runScriptByHash({
						workspace: ws,
						hash: String(scriptHash),
						requestBody
					})
				: await JobService.runScriptByPath({ workspace: ws, path: scriptPath, requestBody })
			// Polled rather than awaited: a preview is a job, and its engine may
			// need provisioning on a cold worker.
			for (let i = 0; i < 90; i++) {
				await new Promise((r) => setTimeout(r, 1000))
				const done = await JobService.getCompletedJobResultMaybe({ workspace: ws, id })
				if (!done.completed) continue
				const res = done.result as { node?: string; show?: Record<string, unknown>[] } | undefined
				const next: Preview =
					done.success && res?.show
						? { rows: res.show, tookMs: Date.now() - startedAt, node: res.node }
						: { error: 'The preview job failed — open it from Runs for the detail.' }
				previews = { ...previews, [key]: next }
				return
			}
			previews = {
				...previews,
				[key]: { error: 'The preview is still running; open it from Runs.' }
			}
		} catch (e) {
			previews = {
				...previews,
				[key]: { error: e instanceof Error ? e.message : String(e) }
			}
		}
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

	// The relation this model writes, fully qualified, as dbt reported it for THIS
	// run. There is no table browser to link to, so the next best affordance is
	// the exact name to paste into a SQL client — quote-aware, because a period
	// inside a quoted identifier is part of the name, not a separator.
	let selectedRelation = $derived.by(() => {
		const sel = selection
		if (sel?.kind !== 'asset' || !selectedDbt) return undefined
		const rel = run?.nodes?.find(
			(n) => n.unique_id === selectedDbt!.unique_id
		)?.relation_name
		return rel ? splitRelation(rel).join('.') : undefined
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
				{#if selectedDbt.resource_type !== 'source'}
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
				<span class="ml-auto shrink-0 opacity-70">read-only · edit locally</span>
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

{#if loading}
	<div class="flex items-center gap-2 text-xs text-secondary p-3">
		<Loader2 class="animate-spin" size={14} /> Loading the model graph
	</div>
{:else if failed}
	<div class="text-xs text-secondary p-3">Could not load the model graph.</div>
{:else if !graph}
	<div class="text-xs text-secondary p-3">
		This dbt script has no models in the asset graph. A project that brings its own
		<span class="font-mono">profiles.yml</span> without naming a
		<span class="font-mono">profile.resource</span> has no warehouse identity to key them on.
	</div>
{:else}
	<div class="border rounded overflow-hidden flex flex-col">
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
		<div class="h-80">
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
