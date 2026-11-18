<script lang="ts">
	// Transitive column-lineage explorer for a selected asset. Renders the
	// connected neighborhood of the asset's columns — sources on the left,
	// derived columns progressively right, laid out by hop depth — as a
	// node-edge diagram. Clicking any column traces its full upstream +
	// downstream impact set across the whole pipeline (not just one hop) and
	// dims everything else, turning the static diagram into impact analysis
	// ("what feeds, or is fed by, this column?"). Deterministic depth layout so
	// the SVG edges line up with the absolutely-positioned boxes without
	// measuring the DOM.
	import type { AssetKind } from '$lib/gen'
	import {
		assetColumnNodes,
		computeDepths,
		connectedComponent,
		traceColumn,
		type ColumnLineageGraph,
		type ColumnNodeId
	} from './columnLineageGraph'
	import { Columns3, ArrowRight, X } from 'lucide-svelte'

	let {
		graph,
		assetKind,
		assetPath,
		targetLabel
	}: {
		graph: ColumnLineageGraph
		assetKind: AssetKind
		assetPath: string
		targetLabel?: string
	} = $props()

	const ROW_H = 34
	const COL_W = 150
	const GAP = 80

	// The selected asset's own column nodes — highlighted as the focus, and the
	// seed of the rendered neighborhood.
	let seeds = $derived(assetColumnNodes(graph, assetKind, assetPath))
	let seedSet = $derived(new Set(seeds))
	let component = $derived(connectedComponent(seeds, graph))
	let depths = $derived(computeDepths(component, graph))
	let maxDepth = $derived(component.size ? Math.max(0, ...depths.values()) : 0)

	// Group nodes into vertical lanes by hop depth (sources at depth 0, left).
	let lanes = $derived.by(() => {
		const byDepth: ColumnNodeId[][] = []
		for (const id of [...component].sort()) {
			const d = depths.get(id) ?? 0
			;(byDepth[d] ??= []).push(id)
		}
		return byDepth
	})
	let pos = $derived.by(() => {
		const m = new Map<ColumnNodeId, { x: number; y: number }>()
		lanes.forEach((lane, depth) =>
			lane.forEach((id, i) => m.set(id, { x: depth * (COL_W + GAP), y: i * ROW_H }))
		)
		return m
	})
	let width = $derived((maxDepth + 1) * COL_W + maxDepth * GAP)
	let height = $derived(Math.max(1, ...lanes.map((l) => l?.length ?? 0)) * ROW_H)

	// One connector per (source → derived) edge within the neighborhood.
	let edges = $derived.by(() => {
		const es: { from: ColumnNodeId; to: ColumnNodeId }[] = []
		for (const out of component)
			for (const src of graph.up.get(out) ?? [])
				if (component.has(src)) es.push({ from: src, to: out })
		return es
	})

	let selected = $state<ColumnNodeId | undefined>(undefined)
	// The active trace: the selected column's full transitive impact set. Reset
	// if the selection no longer exists in the current graph (asset switched).
	let traced = $derived(
		selected && graph.nodes.has(selected) ? traceColumn(selected, graph) : undefined
	)

	function nodeCenterY(id: ColumnNodeId): number {
		return (pos.get(id)?.y ?? 0) + (ROW_H - 6) / 2 + 3
	}
	function edgePath(e: { from: ColumnNodeId; to: ColumnNodeId }): string {
		const x1 = (pos.get(e.from)?.x ?? 0) + COL_W
		const x2 = pos.get(e.to)?.x ?? 0
		const y1 = nodeCenterY(e.from)
		const y2 = nodeCenterY(e.to)
		const mx = (x1 + x2) / 2
		return `M ${x1} ${y1} C ${mx} ${y1}, ${mx} ${y2}, ${x2} ${y2}`
	}

	const dimmed = (id: ColumnNodeId) => traced !== undefined && !traced.has(id)
	const onFocus = (id: ColumnNodeId) => traced !== undefined && traced.has(id)
</script>

<div class="flex flex-col gap-2 p-3">
	<div class="flex items-center gap-1.5 text-2xs uppercase tracking-wide text-tertiary">
		<Columns3 size={12} />
		<span>Column lineage</span>
		{#if targetLabel}
			<ArrowRight size={10} class="text-tertiary" />
			<span class="font-mono normal-case tracking-normal text-secondary truncate"
				>{targetLabel}</span
			>
		{/if}
		<span class="flex-1"></span>
		{#if selected}
			<button
				type="button"
				class="flex items-center gap-0.5 normal-case tracking-normal text-2xs text-secondary hover:text-primary"
				onclick={() => (selected = undefined)}
			>
				<X size={11} /> clear trace
			</button>
		{:else if component.size > 1}
			<span class="normal-case tracking-normal text-3xs text-tertiary">click a column to trace</span
			>
		{/if}
	</div>

	{#if component.size === 0}
		<span class="text-2xs text-tertiary">No column lineage for this asset.</span>
	{:else}
		<div class="relative overflow-auto" style="width: {width}px; height: {height}px;">
			<svg
				class="absolute inset-0 pointer-events-none overflow-visible"
				{width}
				{height}
				viewBox={`0 0 ${width} ${height}`}
			>
				{#each edges as e (`${e.from}->${e.to}`)}
					{@const hot = traced !== undefined && traced.has(e.from) && traced.has(e.to)}
					{@const cold = traced !== undefined && !hot}
					<path
						d={edgePath(e)}
						fill="none"
						class={hot ? 'stroke-blue-500' : 'stroke-blue-400 dark:stroke-blue-500'}
						stroke-width={hot ? 2 : 1.5}
						opacity={cold ? 0.15 : 1}
					/>
				{/each}
			</svg>

			{#each [...component] as id (id)}
				{@const n = graph.nodes.get(id)}
				{@const p = pos.get(id)}
				{#if n && p}
					{@const isSeed = seedSet.has(id)}
					<button
						type="button"
						class="absolute flex flex-col justify-center rounded-md border px-2 leading-tight text-left transition-opacity
							{isSeed
							? 'border-blue-400 dark:border-blue-600 bg-blue-50 dark:bg-blue-950/40'
							: 'border-border bg-surface-secondary'}
							{onFocus(id) ? 'ring-1 ring-blue-500' : ''}"
						style="left: {p.x}px; top: {p.y + 3}px; width: {COL_W}px; height: {ROW_H -
							6}px; opacity: {dimmed(id) ? 0.3 : 1};"
						title={`${n.path}.${n.column}`}
						onclick={() => (selected = selected === id ? undefined : id)}
					>
						<span
							class="text-2xs font-mono truncate {isSeed
								? 'text-blue-700 dark:text-blue-300'
								: 'text-primary'}">{n.column}</span
						>
						<span class="text-3xs font-mono text-tertiary truncate">{n.path}</span>
					</button>
				{/if}
			{/each}
		</div>
	{/if}
</div>
