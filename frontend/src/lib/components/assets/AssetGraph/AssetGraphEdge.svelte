<script lang="ts">
	import { BaseEdge, getBezierPath, type EdgeProps } from '@xyflow/svelte'
	import { getStraightLinePath } from '$lib/components/graph/renderers/utils'

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

	type Pt = { x: number; y: number }

	function unit(from: Pt, to: Pt): Pt {
		const dx = to.x - from.x
		const dy = to.y - from.y
		const l = Math.hypot(dx, dy) || 1
		return { x: dx / l, y: dy / l }
	}
	function dist(a: Pt, b: Pt): number {
		return Math.hypot(b.x - a.x, b.y - a.y)
	}
	// Polyline through `p` with corners rounded by up to `r` — keeps the
	// node-avoiding Sugiyama route while matching the soft look of the
	// bezier edges used elsewhere.
	function roundedPath(p: Pt[], r: number): string {
		if (p.length < 2) return ''
		if (p.length === 2) return `M ${p[0].x},${p[0].y} L ${p[1].x},${p[1].y}`
		let d = `M ${p[0].x},${p[0].y}`
		for (let i = 1; i < p.length - 1; i++) {
			const prev = p[i - 1]
			const cur = p[i]
			const next = p[i + 1]
			const d1 = Math.min(r, dist(prev, cur) / 2)
			const d2 = Math.min(r, dist(cur, next) / 2)
			const a = unit(cur, prev)
			const b = unit(cur, next)
			d += ` L ${cur.x + a.x * d1},${cur.y + a.y * d1}`
			d += ` Q ${cur.x},${cur.y} ${cur.x + b.x * d2},${cur.y + b.y * d2}`
		}
		const last = p[p.length - 1]
		d += ` L ${last.x},${last.y}`
		return d
	}

	let edgePath = $derived.by(() => {
		const route = (data as { routePoints?: Pt[] } | undefined)?.routePoints
		// Use the layout's routed waypoints when there are real bends to
		// follow: keep xyflow's actual handle coords as the endpoints and
		// thread the interior (dummy-vertex) bends so the edge goes around
		// nodes instead of under them.
		if (route && route.length >= 3) {
			const interior = route.slice(1, -1)
			return roundedPath([{ x: sourceX, y: sourceY }, ...interior, { x: targetX, y: targetY }], 14)
		}
		// Fallback (adjacent layers / draft-overlay edges with no route):
		// mirror BaseEdge.svelte — clamp the source-Y bend on long vertical
		// jumps, then append a straight segment.
		const [bezier] = getBezierPath({
			sourceX,
			sourceY: targetY - sourceY > 100 ? targetY - 100 : sourceY,
			sourcePosition,
			targetX,
			targetY,
			targetPosition,
			curvature: 0.25
		})
		return targetY - sourceY > 100
			? `${bezier} ${getStraightLinePath({ sourceX, sourceY, targetY })}`
			: bezier
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
