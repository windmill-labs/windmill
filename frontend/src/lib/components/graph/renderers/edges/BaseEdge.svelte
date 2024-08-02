<script lang="ts">
	import { getBezierPath, BaseEdge, type EdgeProps } from '@xyflow/svelte'

	type $$Props = EdgeProps

	export let sourceX: $$Props['sourceX']
	export let sourceY: $$Props['sourceY']
	export let sourcePosition: $$Props['sourcePosition']
	export let targetX: $$Props['targetX']
	export let targetY: $$Props['targetY']
	export let targetPosition: $$Props['targetPosition']
	export let markerEnd: $$Props['markerEnd'] = undefined
	export let style: $$Props['style'] = undefined

	$: [edgePath] = getBezierPath({
		sourceX,
		sourceY,
		sourcePosition,
		targetX,
		targetY,
		targetPosition
	})
</script>

<BaseEdge path={edgePath} {markerEnd} {style} />
