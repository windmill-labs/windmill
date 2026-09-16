<script lang="ts">
	// The column trace for one selected asset, plus the three things that are not
	// the diagram: the lineage still being fetched, a request that failed, and an
	// answer the API cut short. All three exist because a dbt project's half
	// arrives in a request of its own, so an empty or partial trace has several
	// causes that draw identically — a project that never asked for the analysis
	// pass, an answer that has not landed, one that never will, and one that ends
	// where a bound put it.
	import type { AssetKind } from '$lib/gen'
	import { Loader2 } from 'lucide-svelte'
	import ColumnLineageTrace from './ColumnLineageTrace.svelte'
	import { assetColumnNodes, type ColumnLineageGraph } from './columnLineageGraph'

	let {
		graph,
		assetKind,
		assetPath,
		targetLabel,
		loading = false,
		truncated = false,
		failed = false
	}: {
		graph?: ColumnLineageGraph
		assetKind: AssetKind
		assetPath: string
		targetLabel?: string
		loading?: boolean
		truncated?: boolean
		/** The lineage request failed. Said rather than left blank: a project
		 *  that never asked for the analysis pass draws nothing either, and the
		 *  two must not look alike. */
		failed?: boolean
	} = $props()

	// The selected asset's own column nodes, which is what decides whether there
	// is a trace to draw at all: a producer that declares no column lineage — or a
	// dbt project that never asked for the analysis pass — has none.
	let nodes = $derived(graph ? assetColumnNodes(graph, assetKind, assetPath) : [])
</script>

<!-- Beside a drawn trace as well as instead of one. A selection whose producers
     declare column lineage has nodes from the graph the canvas already carries,
     so a failed dbt request leaves a trace that renders and is missing a half —
     which is the reading this line exists to prevent. -->
{#snippet failure()}
	<div class="px-3 py-1.5 text-2xs text-tertiary">
		Part of this column lineage could not be loaded, so the trace may be incomplete.
	</div>
{/snippet}

{#if loading && nodes.length === 0}
	<div class="border-b shrink-0 flex items-center gap-2 px-3 py-1.5 text-2xs text-secondary">
		<Loader2 size={12} class="animate-spin" />
		Loading column lineage
	</div>
{:else if nodes.length === 0}
	{#if failed}
		<div class="border-b shrink-0">{@render failure()}</div>
	{/if}
{:else if graph}
	<div class="border-b shrink-0 overflow-auto max-h-64">
		<ColumnLineageTrace {graph} {assetKind} {assetPath} {targetLabel} />
		{#if failed}
			{@render failure()}
		{/if}
		{#if truncated}
			<div class="px-3 pb-1.5 text-2xs text-tertiary">
				Showing the part of the trace nearest this relation. The lineage reaches further than one
				view can draw.
			</div>
		{/if}
	</div>
{/if}
