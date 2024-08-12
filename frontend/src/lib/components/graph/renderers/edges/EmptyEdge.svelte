<script lang="ts">
	import { getBezierPath, BaseEdge, type Position } from '@xyflow/svelte'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'

	export let sourceX: number
	export let sourceY: number
	export let sourcePosition: Position
	export let targetX: number
	export let targetY: number
	export let targetPosition: Position
	export let markerEnd: string | undefined = undefined

	const { useDataflow } = getContext<{
		useDataflow: Writable<boolean | undefined>
	}>('FlowGraphContext')

	$: [edgePath] = getBezierPath({
		sourceX,
		sourceY,
		sourcePosition,
		targetX,
		targetY,
		targetPosition
	})
</script>

<BaseEdge path={edgePath} {markerEnd} class={$useDataflow ? 'hidden' : ''} />
