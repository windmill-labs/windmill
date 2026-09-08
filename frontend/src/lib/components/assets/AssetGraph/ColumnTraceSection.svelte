<script lang="ts">
	// The column trace for one selected asset, plus the two things that are not
	// the diagram: the lineage still being fetched, and an answer the API cut
	// short. Both exist because a dbt project's half arrives in a request of its
	// own — a project without the analysis pass and one whose answer has not
	// landed yet are the same empty graph otherwise, and a trace that stops at a
	// bound reads exactly like one that ends.
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

{#if loading && nodes.length === 0}
	<div class="border-b shrink-0 flex items-center gap-2 px-3 py-1.5 text-2xs text-secondary">
		<Loader2 size={12} class="animate-spin" />
		Loading column lineage
	</div>
{:else if failed && nodes.length === 0}
	<div class="border-b shrink-0 px-3 py-1.5 text-2xs text-tertiary">
		Column lineage could not be loaded. Refresh to try again.
	</div>
{:else if graph && nodes.length > 0}
	<div class="border-b shrink-0 overflow-auto max-h-64">
		<ColumnLineageTrace {graph} {assetKind} {assetPath} {targetLabel} />
		{#if truncated}
			<div class="px-3 pb-1.5 text-2xs text-tertiary">
				Showing the part of the trace nearest this relation. The lineage reaches further than one
				view can draw.
			</div>
		{/if}
	</div>
{/if}
