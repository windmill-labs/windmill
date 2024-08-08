<script lang="ts">
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'
	import type { FlowModule } from '$lib/gen/models/FlowModule'
	import {
		getBezierPath,
		BaseEdge,
		type Position,
		EdgeLabelRenderer
		// @ts-ignore
	} from '@xyflow/svelte'

	export let sourceX: number
	export let sourceY: number
	export let sourcePosition: Position
	export let targetX: number
	export let targetY: number
	export let targetPosition: Position
	export let markerEnd: string | undefined = undefined
	export let style: string | undefined = undefined

	export let data: {
		insertable: boolean
		modules: FlowModule[]
	}

	$: [edgePath, labelX, labelY] = getBezierPath({
		sourceX,
		sourceY: targetY - sourceY > 100 ? targetY - 100 : sourceY,
		sourcePosition,
		targetX,
		targetY,
		targetPosition,
		curvature: 0.1
	})

	function getStraightLinePath({ sourceX, sourceY, targetX, targetY }) {
		return `M${sourceX},${sourceY} L${sourceX},${targetY - 100}`
	}

	$: completeEdge =
		targetY - sourceY > 100
			? `${edgePath} ${getStraightLinePath({ sourceX, sourceY, targetX, targetY })}`
			: edgePath
</script>

<BaseEdge path={completeEdge} {markerEnd} {style} />
<EdgeLabelRenderer>
	{#if data?.insertable}
		<div
			class="edgeButtonContainer nodrag nopan"
			style:transform="translate(-50%, -50%) translate({labelX}px,{labelY}px)"
		>
			<InsertModuleButton index={0} modules={data?.modules ?? []} funcDesc="" />
		</div>
	{/if}
</EdgeLabelRenderer>

<style>
	.edgeButtonContainer {
		position: absolute;
		font-size: 12pt;
		/* everything inside EdgeLabelRenderer has no pointer events by default */
		/* if you have an interactive element, set pointer-events: all */
		pointer-events: all;
	}
</style>
