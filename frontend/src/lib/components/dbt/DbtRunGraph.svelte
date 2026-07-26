<script lang="ts">
	// The models this run touches, as the graph, on the page you land on when you
	// click a running job. The live per-model state the worker records is what
	// makes it worth having here: nodes go amber then green as dbt walks the DAG,
	// where the alternative is reading `N of M OK created` out of the log.
	import { onDestroy } from 'svelte'
	import { OpenAPI } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import AssetGraphCanvas from '$lib/components/assets/AssetGraph/AssetGraphCanvas.svelte'
	import type { AssetGraphResponse } from '$lib/components/assets/AssetGraph/types'
	import { Loader2 } from 'lucide-svelte'

	let {
		scriptPath,
		running = false
	}: {
		scriptPath: string
		// While the job is in flight the graph is re-fetched, because the per-model
		// materialization rows the worker writes are what move the nodes.
		running?: boolean
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

	let timer: ReturnType<typeof setInterval> | undefined
	$effect(() => {
		void scriptPath
		void load()
	})
	$effect(() => {
		clearInterval(timer)
		// Only while in flight, and no faster than dbt finishes a model: this is a
		// poll against the same rows the pipeline page reads, not a subscription.
		if (running) timer = setInterval(load, 2000)
		return () => clearInterval(timer)
	})
	onDestroy(() => clearInterval(timer))

	// A run page is about one script, so the graph is its runnable plus the
	// relations it reads and writes — a folder's other projects are noise here.
	let scoped = $derived.by(() => {
		if (!raw) return undefined
		const edges = raw.edges.filter((e) => e.runnable_path === scriptPath)
		if (edges.length === 0) return undefined
		const assetIds = new Set(edges.map((e) => `${e.asset_kind}:${e.asset_path}`))
		return {
			...raw,
			runnables: raw.runnables.filter((r) => r.path === scriptPath),
			assets: raw.assets.filter((a) => assetIds.has(`${a.kind}:${a.path}`)),
			edges,
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
</script>

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
	<div class="h-80 border rounded overflow-hidden">
		<AssetGraphCanvas {graph} showMinimap={false} scrollZoom={false} />
	</div>
{/if}
