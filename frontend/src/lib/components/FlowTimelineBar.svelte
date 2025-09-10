<script lang="ts">
	import { msToReadableTime, msToReadableTimeShort } from '$lib/utils'
	import { ZoomIn, ZoomOut } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { GlobalIterationBounds } from './graph'
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
		globalIterationBounds?: GlobalIterationBounds
		loadPreviousIterations?: () => void
		onSelectIteration?: (id: string) => void
		idToIterationIndex?: (id: string) => number | undefined
		showInterations?: string[]
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
		globalIterationBounds,
		loadPreviousIterations,
		onSelectIteration,
		idToIterationIndex,
		showInterations
	}: Props = $props()

	function getLength(item: TimelineItem): number {
		if (!item?.started_at) return 0
		return item.duration_ms ?? now - item.started_at
	}

	function isRunning(item: TimelineItem): boolean {
		return item.started_at !== undefined && item.duration_ms === undefined
	}

	const filteredItems = $derived(
		showInterations ? items.filter((item) => showInterations.includes(item.id)) : items
	)

	let selectedItem = $derived(
		selectedIndex && selectedIndex >= 0 ? filteredItems[selectedIndex] : filteredItems[0]
	)
	let startItem = $derived(showInterations ? filteredItems[0] : selectedItem)

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
		showInterations ? calculateTotalExecutionTime() : getLength(selectedItem)
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

	function getOverlapOpacity(item: TimelineItem, allItems: TimelineItem[]): number {
		if (!item.started_at) return 1

		const itemEnd = item.duration_ms ? item.started_at + item.duration_ms : now
		let overlapCount = 0

		for (const otherItem of allItems) {
			if (otherItem.id === item.id || !otherItem.started_at) continue

			const otherEnd = otherItem.duration_ms ? otherItem.started_at + otherItem.duration_ms : now

			// Check if time ranges overlap
			if (item.started_at < otherEnd && otherItem.started_at < itemEnd) {
				overlapCount++
			}
		}

		// Base opacity of 1, reduce by 0.2 for each overlap, minimum 0.3
		return Math.max(0.3, 1 - overlapCount * 0.2)
	}
</script>

{#if min && filteredItems.length > 0}
	<div
		class="flex items-center gap-2 ml-auto min-w-32 max-w-[1000px] h-4 group"
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
		{:else if globalIterationBounds && globalIterationBounds.iteration_from && globalIterationBounds.iteration_from > 0}
			<Tooltip
				class="hover:text-primary hover:bg-surface p-1 -my-1 w-24 rounded-md flex items-center justify-center"
				openDelay={100}
			>
				<button
					class="text-2xs text-primary whitespace-nowrap"
					onclick={(e) => {
						e.stopPropagation()
						loadPreviousIterations?.()
					}}
				>
					load more...
				</button>
				{#snippet text()}
					Load previous iterations
				{/snippet}
			</Tooltip>
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

			{#if showInterations}
				<!-- All iterations with absolute positioning -->
				{#each filteredItems as item, i}
					{#if item.started_at}
						{@const position = calculateItemPosition(item)}
						{@const opacity = getOverlapOpacity(item, filteredItems)}
						<Tooltip
							style="left: {position.left}%; width: {position.width}%"
							class="h-full absolute top-0"
							openDelay={100}
						>
							<!-- svelte-ignore a11y_consider_explicit_label -->
							<div class="relative w-full h-full">
								<button
									class={twMerge(
										'w-full h-full hover:outline outline-1 outline-white -outline-offset-1 rounded-sm block transition-opacity duration-200',
										isRunning(item) ? 'bg-blue-400' : 'bg-blue-500',
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
						<div
							class={twMerge(
								'h-full hover:outline outline-1 outline-white -outline-offset-1 rounded-sm',
								isRunning(selectedItem) ? ' bg-blue-400' : ' bg-blue-500'
							)}
						></div>
						{#snippet text()}
							{msToReadableTime(selectedLen, 1)}
						{/snippet}
					</Tooltip>
				{/if}
			{/if}
		</div>
		{#if selectedLen > 0}
			<span
				class="text-2xs text-tertiary font-mono font-normal w-10 truncate"
				title={msToReadableTime(selectedLen, 1)}>{msToReadableTimeShort(selectedLen, 1)}</span
			>
		{/if}
	</div>
{/if}
