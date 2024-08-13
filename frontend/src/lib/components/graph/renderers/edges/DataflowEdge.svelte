<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getBezierPath, BaseEdge, type Position, EdgeLabelRenderer } from '@xyflow/svelte'
	import { getStraightLinePath } from '../utils'

	export let sourceX: number
	export let sourceY: number
	export let sourcePosition: Position
	export let targetX: number
	export let targetY: number
	export let targetPosition: Position
	export let markerEnd: string | undefined = undefined

	export let data: {
		modules: FlowModule[]
		sourceId: string
		targetId: string
	}

	$: [edgePath, labelX, labelY] = getBezierPath({
		sourceX,
		sourceY: targetY - sourceY > 100 ? targetY - 100 : sourceY,
		sourcePosition,
		targetX,
		targetY,
		targetPosition,
		curvature: 0.25
	})
</script>

<EdgeLabelRenderer>
	<div
		class="absolute cursor-pointer nodrag nopan bg-surface-selected p-1 border text-xs"
		style:transform="translate(-50%, -50%) translate({labelX}px,{labelY}px)"
	>
		{data.sourceId} -> {data.targetId}
	</div>
</EdgeLabelRenderer>

<BaseEdge
	path={targetY - sourceY > 100
		? `${edgePath} ${getStraightLinePath({ sourceX, sourceY, targetY })}`
		: edgePath}
	{markerEnd}
	style={`animation:dashdraw 0.5s linear infinite; stroke-dasharray: 5px;`}
/>
