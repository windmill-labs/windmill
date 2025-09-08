<script lang="ts">
	import { msToSec } from '$lib/utils'

	interface TimelineItem {
		created_at?: number
		started_at?: number
		duration_ms?: number
		id: string
	}

	interface Props {
		total: number
		min: number | undefined
		items: TimelineItem[]
		selectedIndex: number
		now: number
		showSingleItem?: boolean
	}

	let { total, min, items, selectedIndex, now, showSingleItem = true }: Props = $props()

	function getLength(item: TimelineItem): number {
		if (!item.started_at) return 0
		return item.duration_ms ?? now - item.started_at
	}

	function isRunning(item: TimelineItem): boolean {
		return item.started_at !== undefined && item.duration_ms === undefined
	}

	let selectedItem = $derived(items[selectedIndex])

	// Calculate total execution time for multiple items
	function calculateTotalExecutionTime(): number {
		if (showSingleItem) return 0

		let earliestStart: number | undefined
		let latestEnd = 0

		for (const item of items) {
			if (item.started_at) {
				// Track earliest start
				if (!earliestStart || item.started_at < earliestStart) {
					earliestStart = item.started_at
				}

				// Track latest end
				const itemEnd = item.duration_ms ? item.started_at + item.duration_ms : now
				latestEnd = Math.max(latestEnd, itemEnd)
			}
		}

		return earliestStart ? latestEnd - earliestStart : 0
	}

	let totalExecutionTime = $derived(calculateTotalExecutionTime())
	let selectedLen = $derived(
		showSingleItem ? (selectedItem?.started_at ? getLength(selectedItem) : 0) : totalExecutionTime
	)
</script>

{#if min && items.length > 0}
	<div class="flex items-center gap-2">
		<div class="flex-1 h-1 bg-gray-50 dark:bg-gray-800 rounded-sm overflow-hidden">
			{#if items.length === 1 || showSingleItem}
				<!-- Single item case (non-loop) -->
				{#if selectedItem?.started_at}
					<div
						style="width: {((selectedItem.started_at - min) / total) * 100}%"
						class="h-full float-left"
					></div>
					<div
						style="width: {(selectedLen / total) * 100}%"
						class="h-full float-left {isRunning(selectedItem) ? 'bg-blue-400' : 'bg-blue-500'}"
					></div>
				{/if}
			{:else}
				<!-- Multiple items case (loop) -->
				<div class="flex h-full">
					<!-- Empty space before first item -->
					{#if items[0]?.started_at}
						<div style="width: {((items[0].started_at - min) / total) * 100}%" class="h-full"></div>
					{/if}
					<!-- All iterations -->
					{#each items as item, i}
						{#if item.started_at}
							{#if i > 0}
								<!-- Small gap between iterations -->
								<div class="w-px h-full bg-gray-50 dark:bg-gray-800"></div>
							{/if}
							<div
								style="width: {(getLength(item) / total) * 100}%"
								class="h-full {isRunning(item) ? 'bg-blue-400' : 'bg-blue-500'}"
							></div>
						{/if}
					{/each}
				</div>
			{/if}
		</div>
		{#if selectedLen > 0}
			<span class="text-xs text-tertiary font-mono">{msToSec(selectedLen, 1)}s</span>
		{/if}
	</div>
{/if}
