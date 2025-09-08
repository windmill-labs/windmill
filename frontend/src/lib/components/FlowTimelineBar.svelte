<script lang="ts">
	import { msToSec } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'

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

	let selectedItem = $derived(showSingleItem ? items[selectedIndex] : items[0])

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

	const waitingLen = $derived(
		selectedItem?.created_at
			? selectedItem.started_at
				? selectedItem.started_at - selectedItem.created_at
				: selectedItem.duration_ms
					? 0
					: now - selectedItem.created_at
			: 0
	)

	$inspect('dbg selectedItem', selectedItem, showSingleItem)

	function getGap(items: TimelineItem[], i: number): number {
		// The gap between the start of the current item and the end of the previous item
		if (i > 0 && items[i].started_at && items[i - 1].started_at && items[i - 1].duration_ms) {
			return (
				(items[i].started_at ?? 0) -
				(items[i - 1].started_at ?? 0) -
				(items[i - 1].duration_ms ?? 0)
			)
		}
		return 0
	}
</script>

{#if min && items.length > 0}
	<div class="flex items-center gap-2">
		<div class="flex-1 h-1 bg-gray-50 dark:bg-gray-800 rounded-sm overflow-hidden">
			{#if waitingLen > 100 && selectedItem.created_at}
				<div
					style="width: {((selectedItem.created_at - min) / total) * 100}%"
					class="h-full float-left"
				></div>
				<div
					style="width: {(waitingLen / total) * 100}%"
					class="h-full float-left bg-gray-300 dark:bg-gray-600"
				></div>
			{:else if selectedItem?.started_at}
				<div
					style="width: {((selectedItem.started_at - min) / total) * 100}%"
					class="h-full float-left"
				></div>
			{/if}

			{#if items.length === 1 || showSingleItem}
				<!-- Single item case (non-loop) -->
				{#if selectedItem?.started_at}
					<div
						style="width: {(selectedLen / total) * 100}%"
						class={twMerge(
							'h-full ',
							isRunning(selectedItem) ? 'float-right bg-blue-400' : 'float-left bg-blue-500'
						)}
					></div>
				{/if}
			{:else}
				<!-- All iterations -->
				{#each items as item, i}
					{#if item.started_at}
						<div
							style="width: {(getGap(items, i) / total) * 100}%"
							class={twMerge('h-full float-left bg-gray-50 dark:bg-gray-800')}
						></div>
						<div
							style="width: {(getLength(item) / total) * 100}%"
							class={twMerge(
								'h-full ',
								isRunning(item) ? 'float-right bg-blue-400' : 'float-left bg-blue-500',
								i > 0 ? 'border-l border-gray-50 dark:border-gray-800 ' : ''
							)}
						></div>
					{/if}
				{/each}
			{/if}
		</div>
		{#if selectedLen > 0}
			<span class="text-xs text-tertiary font-mono">{msToSec(selectedLen, 1)}s</span>
		{/if}
	</div>
{/if}
