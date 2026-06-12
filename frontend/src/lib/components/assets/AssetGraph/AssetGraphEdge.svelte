<script lang="ts">
	import { BaseEdge, getBezierPath, type EdgeProps } from '@xyflow/svelte'
	import { NODE } from '$lib/components/graph/util'

	let {
		sourceX,
		sourceY,
		sourcePosition,
		targetX,
		targetY,
		targetPosition,
		markerEnd,
		style
	}: EdgeProps = $props()

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
	// gutter lane, run it down past the intermediate rows, swing back into
	// the target.
	function gutterPath(): string {
		const side = targetX >= sourceX ? 1 : -1
		// Beyond *both* endpoints so the in/out horizontal runs never
		// degenerate (midX stays a full lane away from each column).
		const midX = side > 0 ? Math.max(sourceX, targetX) + LANE : Math.min(sourceX, targetX) - LANE
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
		if (dy > SKIP_DY && Math.abs(targetX - sourceX) < NEAR_VERTICAL_DX) {
			return gutterPath()
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
