<script lang="ts">
	import { getBezierPath, BaseEdge, type Position } from '@xyflow/svelte'
	import { twMerge } from 'tailwind-merge'
	import { getGraphContext } from '../../graphContext'
	import { NODE } from '../../util'

	interface Props {
		sourceX: number
		sourceY: number
		sourcePosition: Position
		targetX: number
		targetY: number
		targetPosition: Position
		markerEnd?: string | undefined
		data?: { class?: string; bowWhenAligned?: boolean }
	}

	let {
		sourceX,
		sourceY,
		sourcePosition,
		targetX,
		targetY,
		targetPosition,
		markerEnd = undefined,
		data = {}
	}: Props = $props()

	const { useDataflow } = getGraphContext()

	// An edge that skips over nodes stacked in the same column would be drawn behind them, so it
	// bulges into the gap beside the column: a cubic with both control points at +k peaks at 0.75k.
	const BOW_K = (NODE.width / 2 + NODE.gap.horizontal / 2) / 0.75
	let bowed = $derived(data.bowWhenAligned && Math.abs(targetX - sourceX) < NODE.width / 2)

	let [edgePath] = $derived(
		bowed
			? [
					`M ${sourceX} ${sourceY} C ${sourceX + BOW_K} ${sourceY}, ${targetX + BOW_K} ${targetY}, ${targetX} ${targetY}`
				]
			: getBezierPath({
					sourceX,
					sourceY,
					sourcePosition,
					targetX,
					targetY,
					targetPosition,
					curvature: 0.25
				})
	)
</script>

<BaseEdge
	path={edgePath}
	{markerEnd}
	class={twMerge($useDataflow ? 'hidden' : '', 'pointer-events-none', data.class ?? '')}
	interactionWidth={0}
/>
