<script lang="ts">
	import { getBezierPath, BaseEdge, type Position } from '@xyflow/svelte'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		sourceX: number
		sourceY: number
		sourcePosition: Position
		targetX: number
		targetY: number
		targetPosition: Position
		markerEnd?: string | undefined
		data?: { class?: string }
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

	const { useDataflow } = getContext<{
		useDataflow: Writable<boolean | undefined>
	}>('FlowGraphContext')

	let [edgePath] = $derived(
		getBezierPath({
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
	class={twMerge($useDataflow ? 'hidden' : '', data.class ?? '')}
/>
