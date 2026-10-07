<script lang="ts">
	import { BaseEdge, getSmoothStepPath, type EdgeProps } from '@xyflow/svelte'
	import type { SimplifiedEdgeData } from './simplifiedLayout'

	let { id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, data }: EdgeProps =
		$props()

	let [path] = $derived(
		getSmoothStepPath({
			sourceX,
			sourceY,
			targetX,
			targetY,
			sourcePosition,
			targetPosition,
			borderRadius: 10,
			centerX: (data as SimplifiedEdgeData | undefined)?.trunkX
		})
	)
</script>

<BaseEdge {id} {path} interactionWidth={0} />
