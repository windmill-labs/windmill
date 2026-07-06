<script lang="ts">
	import { BaseEdge, getBezierPath, type EdgeProps } from '@xyflow/svelte'
	import { NODE } from '$lib/components/graph/util'
	import { FlaskConical, Columns3, SquareFunction, BellOff } from 'lucide-svelte'
	import type { ColumnLineage, DataTest } from './parsePipelineAnnotations'

	let {
		sourceX,
		sourceY,
		sourcePosition,
		targetX,
		targetY,
		targetPosition,
		markerEnd,
		style,
		data
	}: EdgeProps = $props()

	// Data-test badge on the producer→asset write-edge: the edge *is* the
	// transformation, so the tests that assert on its output belong here. The
	// producer's last-run status tints it (the script fails if any test fails).
	let tests = $derived((data as { data_tests?: DataTest[] } | undefined)?.data_tests)
	let testsStatus = $derived(
		(data as { testsRunStatus?: 'running' | 'success' | 'failure' } | undefined)?.testsRunStatus
	)
	// Badge position on the link. A detoured edge runs its vertical segment in
	// the gutter lane at `detourX`, so anchor the badge there (not on the
	// straight-line midpoint, which would float off the routed path).
	let detourX = $derived((data as { detourX?: number } | undefined)?.detourX)
	let labelX = $derived(detourX ?? (sourceX + targetX) / 2)
	let labelY = $derived((sourceY + targetY) / 2)

	function fmtTest(t: DataTest): string {
		switch (t.type) {
			case 'unique':
				return `unique(${t.column})`
			case 'not_null':
				return `not_null(${t.column})`
			case 'accepted_values':
				return `accepted_values(${t.column} = ${t.values.join(',')})`
			case 'relationships':
				return `relationships(${t.column} → ${t.to_path}.${t.to_column})`
			case 'custom':
				return `custom(${t.path})`
		}
	}
	let badgeTitle = $derived(
		tests && tests.length > 0
			? `${tests.length} data test${tests.length > 1 ? 's' : ''}${
					testsStatus === 'success'
						? ' · last run passed'
						: testsStatus === 'failure'
							? ' · last run failed'
							: ''
				}:\n${tests.map((t) => `• ${fmtTest(t)}`).join('\n')}`
			: ''
	)

	// Column-lineage badge on the same write-edge: a `// column` declaration
	// maps each output column to its upstream source columns. Count = declared
	// output columns; the title lists each mapping.
	let columnLineage = $derived(
		(data as { column_lineage?: ColumnLineage[] } | undefined)?.column_lineage
	)
	let columnsBadgeTitle = $derived(
		columnLineage && columnLineage.length > 0
			? `${columnLineage.length} column${columnLineage.length > 1 ? 's' : ''} mapped:\n${columnLineage
					.map(
						(c) =>
							`• ${c.column} ← ${c.inputs.map((i) => `${i.from_path}.${i.from_column}`).join(', ')}`
					)
					.join('\n')}`
			: ''
	)
	// Stack the columns badge below the data-test badge when both are present so
	// they don't overlap on the link (each badge is 18px tall; +18 clears it).
	let columnsBadgeY = $derived(tests && tests.length > 0 ? labelY + 18 : labelY)

	// Muted read edge: a ducklake/s3 input the script reads every run but that
	// does NOT cascade — `// mute <asset>` or `// mute all` opted it out of the
	// (default) auto-derived trigger. Auto-wiring is the norm, so we badge the
	// exception (a read with no trigger) rather than every derived edge.
	let isMuted = $derived((data as { muted?: boolean } | undefined)?.muted ?? false)
	const mutedBadgeTitle =
		'Read but not cascaded — `// mute` (or `// mute all`) suppresses the auto trigger, so changes to this asset do not re-run this script.'

	// Macro-edge badge: which of the library's macros the consumer calls (all
	// of them when the whole lib is pulled in via `// use`).
	let macroNames = $derived((data as { macro_names?: string[] } | undefined)?.macro_names)
	let macroViaUse = $derived((data as { via_use?: boolean } | undefined)?.via_use ?? false)
	let macroBadgeTitle = $derived(
		macroNames && macroNames.length > 0
			? `${macroViaUse ? 'uses the whole library' : `calls ${macroNames.length} macro${macroNames.length > 1 ? 's' : ''}`}:\n${macroNames
					.map((n) => `• ${n}()`)
					.join('\n')}`
			: macroViaUse
				? 'uses the whole library'
				: ''
	)

	// An edge that skips at least one full layer (source-bottom → target-top
	// gap larger than gap + node row) while staying near-vertical runs
	// straight under the nodes in between — the child-and-grandchild-of-the-
	// same-node case, where the tidy-tree layout puts all three on one
	// column. Detour those through the gutter beside the column.
	const SKIP_DY = NODE.gap.vertical * 2 + NODE.height
	const NEAR_VERTICAL_DX = NODE.width / 4
	// Lane offset: half a node plus a margin that stays inside the
	// inter-column gutter (gap.horizontal / 2).
	const LANE = NODE.width / 2 + NODE.gap.horizontal / 2
	const CORNER = 14

	// Rounded-orthogonal detour: drop out of the source, swing into the
	// gutter lane at `midX`, run it down past the intermediate rows, swing
	// back into the target.
	function gutterPath(midX: number): string {
		const yTop = sourceY + 24
		const yBot = targetY - 24
		const outDir = midX > sourceX ? 1 : -1
		const inDir = targetX > midX ? 1 : -1
		return [
			`M ${sourceX} ${sourceY}`,
			`L ${sourceX} ${yTop - CORNER}`,
			`Q ${sourceX} ${yTop} ${sourceX + outDir * CORNER} ${yTop}`,
			`L ${midX - outDir * CORNER} ${yTop}`,
			`Q ${midX} ${yTop} ${midX} ${yTop + CORNER}`,
			`L ${midX} ${yBot - CORNER}`,
			`Q ${midX} ${yBot} ${midX + inDir * CORNER} ${yBot}`,
			`L ${targetX - inDir * CORNER} ${yBot}`,
			`Q ${targetX} ${yBot} ${targetX} ${yBot + CORNER}`,
			`L ${targetX} ${targetY}`
		].join(' ')
	}

	// Long edges bend out of the source within the layer gap and descend in
	// the *target* column. The flow editor's pattern (descend at the source,
	// bend at the bottom) assumed the source column below was free — in the
	// banded tree layout it's where the source's own children live, so the
	// straight segment ran under them (e.g. fx_rates → daily_revenue under
	// fx_rates' other subtree). Long edges only target joins here (a single-
	// parent child is always exactly one layer below its parent), and the
	// space above a join is the seam between its parents' bands — clear by
	// construction.
	let edgePath = $derived.by(() => {
		const dy = targetY - sourceY
		// Obstacle-aware detour: the canvas detected this edge's straight run
		// would pass over an unrelated node and chose a clear lane. Route there.
		const detourX = (data as { detourX?: number } | undefined)?.detourX
		if (detourX != undefined) {
			return gutterPath(detourX)
		}
		if (dy > SKIP_DY && Math.abs(targetX - sourceX) < NEAR_VERTICAL_DX) {
			// Near-vertical same-column skip: lane just beside the column, past
			// both endpoints so the horizontal runs never degenerate.
			const side = targetX >= sourceX ? 1 : -1
			return gutterPath(
				side > 0 ? Math.max(sourceX, targetX) + LANE : Math.min(sourceX, targetX) - LANE
			)
		}
		const bendY = sourceY + Math.min(100, NODE.gap.vertical)
		const long = dy > 100
		const [bezier] = getBezierPath({
			sourceX,
			sourceY,
			sourcePosition,
			targetX,
			targetY: long ? bendY : targetY,
			targetPosition,
			curvature: 0.25
		})
		// The bezier ends at (targetX, bendY); continue straight down the
		// target column into the node.
		return long ? `${bezier} L${targetX},${targetY}` : bezier
	})
</script>

<BaseEdge
	path={edgePath}
	{markerEnd}
	{style}
	interactionWidth={0}
	label={undefined}
	labelStyle={undefined}
/>

{#if tests && tests.length > 0}
	<!-- Badge centered on the link midpoint. Edges render in the SVG layer, so
	     the HTML badge is wrapped in a foreignObject (no EdgeLabelRenderer in
	     this @xyflow/svelte version). -->
	<foreignObject x={labelX - 28} y={labelY - 9} width="56" height="18" class="overflow-visible">
		<div
			xmlns="http://www.w3.org/1999/xhtml"
			class="w-full h-full flex items-center justify-center"
			style="pointer-events: none;"
		>
			<div
				class="flex items-center gap-0.5 px-1 py-0.5 rounded-sm border shadow-sm text-3xs leading-none font-mono cursor-default {testsStatus ===
				'failure'
					? 'bg-red-50 dark:bg-red-950/50 border-red-300 dark:border-red-900/60 text-red-700 dark:text-red-300'
					: testsStatus === 'success'
						? 'bg-emerald-50 dark:bg-emerald-950/50 border-emerald-300 dark:border-emerald-900/60 text-emerald-700 dark:text-emerald-300'
						: 'bg-surface border-border text-secondary'}"
				style="pointer-events: all;"
				title={badgeTitle}
			>
				<FlaskConical size={10} />
				<span>×{tests.length}</span>
			</div>
		</div>
	</foreignObject>
{/if}

{#if (macroNames && macroNames.length > 0) || macroViaUse}
	<!-- Macro-edge badge, centered on the link like the data-test badge. -->
	<foreignObject x={labelX - 28} y={labelY - 9} width="56" height="18" class="overflow-visible">
		<div
			xmlns="http://www.w3.org/1999/xhtml"
			class="w-full h-full flex items-center justify-center"
			style="pointer-events: none;"
		>
			<div
				class="flex items-center gap-0.5 px-1 py-0.5 rounded-sm border shadow-sm text-3xs leading-none font-mono cursor-default bg-surface border-violet-300 dark:border-violet-900/60 text-violet-700 dark:text-violet-300"
				style="pointer-events: all;"
				title={macroBadgeTitle}
			>
				<SquareFunction size={10} />
				<span>×{macroNames?.length ?? 0}</span>
			</div>
		</div>
	</foreignObject>
{/if}

{#if isMuted}
	<!-- Muted-read badge, centered on the read link. Bell-off = this input is
	     read but its auto cascade trigger is suppressed (`// mute`). -->
	<foreignObject x={labelX - 30} y={labelY - 9} width="60" height="18" class="overflow-visible">
		<div
			xmlns="http://www.w3.org/1999/xhtml"
			class="w-full h-full flex items-center justify-center"
			style="pointer-events: none;"
		>
			<div
				class="flex items-center gap-0.5 px-1 py-0.5 rounded-sm border shadow-sm text-3xs leading-none font-mono cursor-default bg-surface border-amber-300 dark:border-amber-900/60 text-amber-700 dark:text-amber-300"
				style="pointer-events: all;"
				title={mutedBadgeTitle}
			>
				<BellOff size={10} />
				<span>muted</span>
			</div>
		</div>
	</foreignObject>
{/if}

{#if columnLineage && columnLineage.length > 0}
	<!-- Column-lineage badge, stacked below the data-test badge when both exist. -->
	<foreignObject
		x={labelX - 28}
		y={columnsBadgeY - 9}
		width="56"
		height="18"
		class="overflow-visible"
	>
		<div
			xmlns="http://www.w3.org/1999/xhtml"
			class="w-full h-full flex items-center justify-center"
			style="pointer-events: none;"
		>
			<div
				class="flex items-center gap-0.5 px-1 py-0.5 rounded-sm border shadow-sm text-3xs leading-none font-mono cursor-default bg-surface border-blue-300 dark:border-blue-900/60 text-blue-700 dark:text-blue-300"
				style="pointer-events: all;"
				title={columnsBadgeTitle}
			>
				<Columns3 size={10} />
				<span>×{columnLineage.length}</span>
			</div>
		</div>
	</foreignObject>
{/if}
