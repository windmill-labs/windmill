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
		style
	}: EdgeProps = $props()

	// Mirror BaseEdge.svelte (flow editor): clamp the source-Y bend so long
	// vertical jumps stay visually consistent, then append a straight segment.
	let edgePath = $derived.by(() => {
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
