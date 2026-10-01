<script lang="ts">
	import { BaseEdge, getBezierPath, type EdgeProps } from '@xyflow/svelte'
	import { polylineMidpoint, roundedPath, type EdgeRoute } from './assetGraphEdgeRouting'
	import { FlaskConical, Columns3, SquareFunction, BellOff } from 'lucide-svelte'
	import type { ColumnLineage, DataTest } from './parsePipelineAnnotations'

	const CORNER = 10

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
	// The route the canvas computed for this edge (see assetGraphEdgeRouting):
	// spread ports, the height of each horizontal run and, for an edge that
	// skips layers, the lane beside the nodes in between.
	let route = $derived((data as { route?: EdgeRoute } | undefined)?.route)
	let points = $derived.by((): Array<[number, number]> | undefined => {
		if (!route) return undefined
		const sx = sourceX + route.sourceOffset
		const tx = targetX + route.targetOffset
		// The layout reserves each row's height, but a node can render shorter or
		// taller; keep every run clear of the edge's own endpoints.
		const clamp = (y: number) =>
			Math.min(Math.max(y, sourceY + 8), Math.max(sourceY + 8, targetY - 8))
		if (route.lane) {
			const y1 = Math.max(route.lane.y, sourceY + 8)
			const y2 = Math.min(Math.max(route.y, y1), targetY - 8)
			return [
				[sx, sourceY],
				[sx, y1],
				[route.lane.x, y1],
				[route.lane.x, y2],
				[tx, y2],
				[tx, targetY]
			]
		}
		const y = clamp(route.y)
		return [
			[sx, sourceY],
			[sx, y],
			[tx, y],
			[tx, targetY]
		]
	})
	let mid = $derived(points ? polylineMidpoint(points) : undefined)
	let labelX = $derived(mid?.[0] ?? (sourceX + targetX) / 2)
	let labelY = $derived(mid?.[1] ?? (sourceY + targetY) / 2)

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

	let edgePath = $derived.by(() => {
		if (points) return roundedPath(points, CORNER)
		// No route: an edge pointing up (a cycle's feedback edge), which the
		// layered routing has no gap for.
		const [bezier] = getBezierPath({
			sourceX,
			sourceY,
			sourcePosition,
			targetX,
			targetY,
			targetPosition,
			curvature: 0.25
		})
		return bezier
	})
</script>

<BaseEdge
	path={edgePath}
	{markerEnd}
	{style}
	interactionWidth={12}
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
	     read but its auto cascade trigger is suppressed (`// mute`). Icon-only;
	     the tooltip carries the explanation. -->
	<foreignObject x={labelX - 9} y={labelY - 9} width="18" height="18" class="overflow-visible">
		<div
			xmlns="http://www.w3.org/1999/xhtml"
			class="w-full h-full flex items-center justify-center"
			style="pointer-events: none;"
		>
			<div
				class="flex items-center px-1 py-0.5 rounded-sm border shadow-sm leading-none cursor-default bg-surface border-amber-300 dark:border-amber-900/60 text-amber-700 dark:text-amber-300"
				style="pointer-events: all;"
				title={mutedBadgeTitle}
			>
				<BellOff size={10} />
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
