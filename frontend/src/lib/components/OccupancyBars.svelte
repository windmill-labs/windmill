<script lang="ts">
	import MeltTooltip from '$lib/components/meltComponents/Tooltip.svelte'

	interface Props {
		rate_15s?: number
		rate_5m?: number
		rate_30m?: number
		rate_ever?: number
	}

	let { rate_15s, rate_5m, rate_30m, rate_ever }: Props = $props()

	function displayOccupancyRate(occupancy_rate: number | undefined) {
		if (occupancy_rate == undefined) {
			return '--'
		}
		return Math.ceil(occupancy_rate * 100) + '%'
	}

	const rates = $derived([
		{ value: rate_15s, label: '15s' },
		{ value: rate_5m, label: '5m' },
		{ value: rate_30m, label: '30m' },
		{ value: rate_ever, label: 'ever' }
	])
</script>

<div class="flex gap-1 items-end py-1">
	{#each rates as rate}
		<MeltTooltip>
			<div class="relative w-4 h-8 bg-surface-secondary rounded-sm border shadow-sm">
				{#if rate.value !== undefined && rate.value > 0}
					{@const heightPercent = Math.min(rate.value * 100, 100)}
					{@const minHeight = heightPercent > 0 && heightPercent < 3 ? 1 : heightPercent}
					<div
						class="absolute bottom-0 left-0 right-0 bg-surface-accent-primary rounded-sm transition-all duration-200"
						style="height: {minHeight < 3 ? `${minHeight}px` : `${heightPercent}%`}"
					></div>
				{/if}
			</div>
			{#snippet text()}
				{rate.label}: {rate.value ? displayOccupancyRate(rate.value) : '--'}
			{/snippet}
		</MeltTooltip>
	{/each}
</div>
