<script lang="ts">
	// The models this run touches, as the graph, on the page you land on when you
	// click a running job. The live per-model state the worker records is what
	// makes it worth having here: nodes go amber then green as dbt walks the DAG,
	// where the alternative is reading `N of M OK created` out of the log.
	import { onDestroy } from 'svelte'
	import { OpenAPI, JobService } from '$lib/gen'
	import { parseDbtRun, relationOutcome, splitRelation } from './parseDbtRun'
	import { Button } from '$lib/components/common'
	import { ClipboardCopy } from 'lucide-svelte'
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
		result
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
	} = $props()

	// The graph endpoint is folder-scoped; a script outside `f/` has no folder and
	// falls back to the whole workspace, which the filter below narrows anyway.
	let folder = $derived(scriptPath.startsWith('f/') ? scriptPath.split('/')[1] : undefined)

	let raw = $state<AssetGraphResponse | undefined>(undefined)
	let loading = $state(true)
	let failed = $state(false)

	async function load() {
		const ws = $workspaceStore
		if (!ws) return
		try {
			const params = new URLSearchParams({ asset_kinds: 'table' })
			if (folder) params.set('folder', folder)
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

	// A run page is about one script, so the graph is the relations it reads and
	// writes — a folder's other projects are noise here, and so is a node for the
	// script itself: on the pipeline page that node distinguishes one project
	// from another, but here the whole graph already IS that project.
	let scoped = $derived.by(() => {
		if (!raw) return undefined
		const edges = raw.edges.filter((e) => e.runnable_path === scriptPath)
		if (edges.length === 0) return undefined
		const assetIds = new Set(edges.map((e) => `${e.asset_kind}:${e.asset_path}`))
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
	let graph = $derived(scoped)

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
		const run = parseDbtRun(result)
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
		const run = parseDbtRun(result)
		if (run?.nodes?.length && !run.nodes.some((n) => n.unique_id === dbt.unique_id)) {
			return undefined
		}
		return dbt
	})

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
		const rel = parseDbtRun(result)?.nodes?.find(
			(n) => n.unique_id === selectedDbt!.unique_id
		)?.relation_name
		return rel ? splitRelation(rel).join('.') : undefined
	})
</script>

{#snippet sqlPane()}
	{#if selectedIsForeign}
		<div class="border-t px-2 py-1.5 text-2xs text-secondary">
			Another dbt project in this workspace also materializes this relation, and the graph keeps
			one project's model per relation — so the SQL shown here would not be this run's. Open that
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
				<HighlightCode language="sql" code={selectedDbt.raw_code} />
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
