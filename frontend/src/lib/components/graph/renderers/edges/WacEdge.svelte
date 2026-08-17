<script lang="ts">
	import { BaseEdge as XyBaseEdge, EdgeLabel, getBezierPath, type EdgeProps } from '@xyflow/svelte'

	let {
		id,
		sourceX,
		sourceY,
		targetX,
		targetY,
		sourcePosition,
		targetPosition,
		label,
		animated
	}: EdgeProps = $props()

	let [path, labelX, labelY] = $derived(
		getBezierPath({
			sourceX,
			sourceY,
			targetX,
			targetY,
			sourcePosition,
			targetPosition,
			curvature: 0.25
		})
	)
</script>

<XyBaseEdge {id} {path} interactionWidth={0} />

{#if label}
	<EdgeLabel x={labelX} y={labelY}>
		<span class="text-2xs text-tertiary bg-surface px-1 rounded">{label}</span>
	</EdgeLabel>
{/if}

{#if animated}
	<path
		d={path}
		fill="none"
		stroke-dasharray="5 5"
		stroke="var(--color-border)"
		stroke-width="1"
		class="animated-dash"
	/>
{/if}

<style>
	.animated-dash {
		animation: dash 1s linear infinite;
	}
	@keyframes dash {
		to {
			stroke-dashoffset: -10;
		}
	}
</style>
