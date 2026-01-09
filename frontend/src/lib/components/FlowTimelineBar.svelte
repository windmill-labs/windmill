<script lang="ts">
	import { msToReadableTime, msToReadableTimeShort } from '$lib/utils'
	import { ZoomIn, ZoomOut } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { Tooltip } from './meltComponents'

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
		selectedIndex?: number
		now: number
		timelinelWidth: number
		showZoomButtons?: boolean
		onZoom?: () => void
		zoom?: 'in' | 'out'
		onSelectIteration?: (id: string) => void
		idToIterationIndex?: (id: string) => number | undefined
		showIterations?: string[]
		isJobFailure?: (id: string) => boolean
	}

	let {
		total,
		min,
		items,
		selectedIndex,
		now,
		timelinelWidth,
		showZoomButtons = false,
		onZoom,
		zoom = 'in',
		onSelectIteration,
		idToIterationIndex,
		showIterations,
		isJobFailure
	}: Props = $props()

	function getLength(item: TimelineItem): number {
		if (!item?.started_at) return 0
		return item.duration_ms ?? now - item.started_at
	}

	function isRunning(item: TimelineItem): boolean {
		return item.started_at !== undefined && item.duration_ms === undefined
	}

	const filteredItems = $derived(
		showIterations ? items.filter((item) => showIterations.includes(item.id)) : items
	)

	let selectedItem = $derived(
		selectedIndex && selectedIndex >= 0 ? filteredItems[selectedIndex] : filteredItems[0]
	)
	let startItem = $derived(showIterations ? filteredItems[0] : selectedItem)

	// Calculate total execution time for multiple filteredItems
	function calculateTotalExecutionTime(): number {
		let earliestStart: number | undefined
		let latestEnd = 0

		for (const item of filteredItems) {
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

	let selectedLen = $derived(
		// If selectedIteration is set, it means we are in a loop and we are selecting an iteration
		showIterations ? calculateTotalExecutionTime() : getLength(selectedItem)
	)

	const waitingLen = $derived(
		startItem?.created_at
			? startItem.started_at
				? startItem.started_at - startItem.created_at
				: startItem.duration_ms
					? 0
					: now - startItem.created_at
			: 0
	)

	function calculateItemPosition(item: TimelineItem): { left: number; width: number } {
		if (!item.started_at || !min) return { left: 0, width: 0 }

		const startOffset = item.started_at - min!
		const duration = getLength(item)
		const leftPercent = (startOffset / total) * 100
		const widthPercent = (duration / total) * 100

		return { left: leftPercent, width: widthPercent }
	}

	// More efficient version using sweep line algorithm for computing all overlaps at once
	function computeAllOverlaps(items: TimelineItem[]): Record<string, number> {
		const overlapCounts = new Map<string, number>()

		// Create events for start and end times
		interface Event {
			time: number
			type: 'start' | 'end'
			itemId: string
		}

		const events: Event[] = []

		for (const item of items) {
			if (!item.started_at) continue

			const endTime = item.duration_ms ? item.started_at + item.duration_ms : now
			events.push({ time: item.started_at, type: 'start', itemId: item.id })
			events.push({ time: endTime, type: 'end', itemId: item.id })
			overlapCounts.set(item.id, 0)
		}

		// Sort events by time, with end events before start events at the same time
		events.sort((a, b) => {
			if (a.time !== b.time) return a.time - b.time
			return a.type === 'end' ? -1 : 1
		})

		// Sweep through events
		const activeItems = new Set<string>()

		for (const event of events) {
			if (event.type === 'start') {
				// Count current active items as overlaps for this item
				overlapCounts.set(event.itemId, activeItems.size)

				// Update overlap counts for all currently active items
				for (const activeId of activeItems) {
					overlapCounts.set(activeId, overlapCounts.get(activeId)! + 1)
				}

				activeItems.add(event.itemId)
			} else {
				activeItems.delete(event.itemId)
			}
		}
		return Object.fromEntries(overlapCounts.entries())
	}

	// At component level, compute once when items change
	const allOverlaps = $derived(computeAllOverlaps(filteredItems))
	const maximumOverlaps = $derived(Math.max(...Object.values(allOverlaps)))
	const opacity = $derived(Math.max(0.02, 1 / maximumOverlaps))
	// Then in your template, use:
	// allOverlaps.get(item.id) ?? 0
</script>

{#if min && filteredItems.length > 0 && startItem?.started_at}
	<div
		class="flex items-center gap-2 ml-auto min-w-96 max-w-[1000px] h-4 group"
		style="width: {timelinelWidth}px"
	>
		{#if showZoomButtons}
			<div class="w-24 flex items-center justify-end">
				<button
					onclick={(e) => {
						e.stopPropagation()
						onZoom?.()
					}}
					class="hover:text-primary hover:bg-surface p-1 -my-1 rounded-md"
				>
					{#if zoom === 'in'}
						<ZoomOut size={12} />
					{:else}
						<ZoomIn size={12} />
					{/if}
				</button>
			</div>
		{:else}
			<div class="w-24"></div>
		{/if}
		<div
			class="flex-1 h-1 bg-gray-300 dark:bg-gray-800 rounded-sm overflow-hidden group-hover:h-full transition-all duration-100 relative"
		>
			{#if waitingLen > 100 && startItem.created_at}
				<div
					style="width: {((startItem.created_at - min) / total) * 100}%"
					class="h-full absolute left-0 top-0"
				>
				</div>
				<div
					style="left: {((startItem.created_at - min) / total) * 100}%; width: {(waitingLen /
						total) *
						100}%"
					class="h-full absolute top-0 bg-gray-300 dark:bg-gray-600"
					title={msToReadableTime(waitingLen, 1)}
				>
				</div>
			{:else if startItem?.started_at}
				<div
					style="width: {((startItem.started_at - min) / total) * 100}%"
					class="h-full absolute left-0 top-0"
				></div>
			{/if}

			{#if showIterations}
				<!-- All iterations with absolute positioning -->
				{#each filteredItems as item, i}
					{#if item.started_at}
						{@const position = calculateItemPosition(item)}
						<!-- {overlapCount}
						{opacity} -->
						<Tooltip
							style="left: {position.left}%; width: {position.width}%"
							class="h-full absolute top-0"
							openDelay={100}
						>
							<!-- svelte-ignore a11y_consider_explicit_label -->
							<div class="relative w-full h-full">
								<button
									class={twMerge(
										'w-full h-full hover:outline outline-1 outline-blue-800 dark:outline-blue-300 -outline-offset-1 rounded-sm block transition-opacity duration-200',
										isRunning(item)
											? 'bg-blue-400'
											: isJobFailure?.(item.id)
												? ' bg-red-500'
												: ' bg-blue-500',
										i > 0 ? 'border-l border-gray-300 dark:border-gray-800 ' : '',
										i === selectedIndex ? 'outline' : ''
									)}
									style="opacity: {opacity}"
									onclick={(e) => {
										e.stopPropagation()
										onSelectIteration?.(item.id)
									}}
								>
								</button>
							</div>
							{#snippet text()}
								{`#${(idToIterationIndex?.(item.id) ?? 0) + 1}`}
								<br />
								{msToReadableTime(getLength(item), 1)}
								{#if opacity < 1}
									<br />
									<span class="text-xs opacity-75">Overlapping</span>
								{/if}
							{/snippet}
						</Tooltip>
					{/if}
				{/each}
			{:else}
				<!-- Single item case or inside a loop -->
				{#if selectedItem?.started_at}
					{@const position = calculateItemPosition(selectedItem)}
					<Tooltip
						class="h-full absolute top-0"
						style="left: {position.left}%; width: {position.width}%"
						openDelay={100}
					>
						<!-- svelte-ignore a11y_consider_explicit_label -->
						<button
							class={twMerge(
								'block w-full h-full hover:outline outline-1 outline-white -outline-offset-1 rounded-sm',
								isRunning(selectedItem)
									? 'bg-blue-400'
									: isJobFailure?.(selectedItem.id)
										? ' bg-red-500'
										: ' bg-blue-500'
							)}
							onclick={(e) => {
								e.stopPropagation()
							}}
						></button>
						{#snippet text()}
							{msToReadableTime(selectedLen, 1)}
						{/snippet}
					</Tooltip>
				{/if}
			{/if}
		</div>
		{#if selectedLen > 0}
			<span
				class="text-2xs text-primary font-mono font-normal w-10 truncate"
				title={msToReadableTime(selectedLen, 1)}>{msToReadableTimeShort(selectedLen, 1)}</span
			>
		{/if}
	</div>
{/if}
