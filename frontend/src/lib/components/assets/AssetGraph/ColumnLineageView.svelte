<script lang="ts">
	// Column-level lineage diagram for a materialized asset: a two-column
	// node-edge view of the producer's `// column <out> <- <src>.<col>`
	// declarations. Upstream source columns on the left, this asset's output
	// columns on the right, bezier connectors between them. Deterministic layout
	// (fixed row height per node) so the SVG edge coordinates line up with the
	// absolutely-positioned HTML boxes without measuring the DOM.
	import type { ColumnLineage } from './parsePipelineAnnotations'
	import { Columns3, ArrowRight } from 'lucide-svelte'

	let { lineage, targetLabel }: { lineage: ColumnLineage[]; targetLabel?: string } = $props()

	const ROW_H = 34
	const COL_W = 150
	const GAP = 80

	type SrcNode = { key: string; path: string; column: string }

	// Distinct upstream source columns, in first-appearance order.
	let sources = $derived.by(() => {
		const seen = new Set<string>()
		const order: SrcNode[] = []
		for (const l of lineage) {
			for (const i of l.inputs) {
				const key = `${i.from_kind}:${i.from_path}:${i.from_column}`
				if (!seen.has(key)) {
					seen.add(key)
					order.push({ key, path: i.from_path, column: i.from_column })
				}
			}
		}
		return order
	})
	let srcIndex = $derived(new Map(sources.map((s, i) => [s.key, i] as const)))

	// One connector per distinct (source column → output column) pair. A single
	// `// column` line may repeat the same upstream ref; that dedupes to one
	// source node, so without this guard the duplicate would mint a second edge
	// with an identical key and break the keyed `{#each}` (each_key_duplicate).
	let edges = $derived.by(() => {
		const seen = new Set<string>()
		const es: { from: number; to: number }[] = []
		lineage.forEach((l, to) => {
			for (const inp of l.inputs) {
				const from = srcIndex.get(`${inp.from_kind}:${inp.from_path}:${inp.from_column}`)
				if (from === undefined) continue
				const key = `${from}->${to}`
				if (seen.has(key)) continue
				seen.add(key)
				es.push({ from, to })
			}
		})
		return es
	})

	let rows = $derived(Math.max(sources.length, lineage.length, 1))
	let height = $derived(rows * ROW_H)
	let width = $derived(COL_W * 2 + GAP)

	function rowY(i: number): number {
		return i * ROW_H + ROW_H / 2
	}
	function edgePath(e: { from: number; to: number }): string {
		const x1 = COL_W
		const x2 = COL_W + GAP
		const y1 = rowY(e.from)
		const y2 = rowY(e.to)
		const mx = (x1 + x2) / 2
		return `M ${x1} ${y1} C ${mx} ${y1}, ${mx} ${y2}, ${x2} ${y2}`
	}
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
	</div>

	<div class="relative overflow-auto" style="width: {width}px; height: {height}px;">
		<!-- Connectors behind the boxes. -->
		<svg
			class="absolute inset-0 pointer-events-none overflow-visible"
			{width}
			{height}
			viewBox={`0 0 ${width} ${height}`}
		>
			{#each edges as e (`${e.from}->${e.to}`)}
				<path
					d={edgePath(e)}
					fill="none"
					class="stroke-blue-400 dark:stroke-blue-500"
					stroke-width="1.5"
				/>
			{/each}
		</svg>

		{#each sources as s, i (s.key)}
			<div
				class="absolute flex flex-col justify-center rounded-md border border-border bg-surface-secondary px-2 leading-tight"
				style="left: 0; top: {i * ROW_H + 3}px; width: {COL_W}px; height: {ROW_H - 6}px;"
				title={`${s.path}.${s.column}`}
			>
				<span class="text-2xs font-mono text-primary truncate">{s.column}</span>
				<span class="text-3xs font-mono text-tertiary truncate">{s.path}</span>
			</div>
		{/each}

		{#each lineage as l, i (`${l.column}:${i}`)}
			<div
				class="absolute flex items-center rounded-md border border-blue-300 dark:border-blue-900/60 bg-blue-50 dark:bg-blue-950/40 px-2"
				style="left: {COL_W + GAP}px; top: {i * ROW_H + 3}px; width: {COL_W}px; height: {ROW_H -
					6}px;"
				title={l.column}
			>
				<span class="text-2xs font-mono text-blue-700 dark:text-blue-300 truncate">{l.column}</span>
			</div>
		{/each}
	</div>
</div>
