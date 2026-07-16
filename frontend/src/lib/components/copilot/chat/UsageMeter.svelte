<script lang="ts">
	import type { Snippet } from 'svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'

	// Presentational gauge shared by the context-usage and free-tier indicators: a thin
	// bar in a tooltip. The owner computes the fill and supplies the tooltip content.
	let {
		fillPct,
		fillClass,
		ariaLabel,
		tooltip
	}: {
		// 0–100. Undefined means "no known max": the bar is decorative (full) and carries
		// no meter role.
		fillPct?: number
		fillClass: string
		ariaLabel: string
		tooltip: Snippet
	} = $props()
</script>

<Tooltip small placement="top">
	<div
		class="flex items-center h-5"
		aria-label={ariaLabel}
		role={fillPct !== undefined ? 'meter' : undefined}
		aria-valuenow={fillPct}
		aria-valuemin={fillPct !== undefined ? 0 : undefined}
		aria-valuemax={fillPct !== undefined ? 100 : undefined}
	>
		<div class="w-8 h-1.5 rounded-full bg-surface-secondary overflow-hidden">
			<div class="h-full rounded-full transition-all {fillClass}" style="width: {fillPct ?? 100}%"
			></div>
		</div>
	</div>
	{#snippet text()}
		{@render tooltip()}
	{/snippet}
</Tooltip>
