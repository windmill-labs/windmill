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

	function getGap(filteredItems: TimelineItem[], i: number): number {
		// The gap between the start of the current item and the end of the previous item
		if (
			i > 0 &&
			filteredItems[i].started_at &&
			filteredItems[i - 1].started_at &&
			filteredItems[i - 1].duration_ms
		) {
			return (
				(filteredItems[i].started_at ?? 0) -
				(filteredItems[i - 1].started_at ?? 0) -
				(filteredItems[i - 1].duration_ms ?? 0)
			)
		}
		return 0
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
			class="flex-1 h-1 bg-gray-300 dark:bg-gray-800 rounded-sm overflow-hidden group-hover:h-full transition-all duration-100"
		>
			{#if waitingLen > 100 && startItem.created_at}
				<div
					style="width: {((startItem.created_at - min) / total) * 100}%"
					class="h-full float-left"
				>
				</div>
				<div
					style="width: {(waitingLen / total) * 100}%"
					class="h-full float-left bg-gray-300 dark:bg-gray-600"
					title={msToReadableTime(waitingLen, 1)}
				>
				</div>
			{:else if startItem?.started_at}
				<div
					style="width: {((startItem.started_at - min) / total) * 100}%"
					class="h-full float-left"
				></div>
			{/if}

			{#if showInterations}
				<!-- All iterations -->
				{#each filteredItems as item, i}
					{#if item.started_at}
						<div
							style="width: {(getGap(filteredItems, i) / total) * 100}%"
							class={twMerge('h-full float-left bg-gray-300 dark:bg-gray-800')}
						></div>
						<Tooltip
							style="width: {(getLength(item) / total) * 100}%"
							class={twMerge('h-full ', isRunning(item) ? 'float-right' : 'float-left')}
							openDelay={100}
						>
							<!-- svelte-ignore a11y_consider_explicit_label -->
							<div class="relative w-full h-full">
								<button
									class={twMerge(
										'w-full h-full hover:outline outline-1 outline-white -outline-offset-1 rounded-sm block',
										isRunning(item) ? 'bg-blue-400' : 'bg-blue-500',
										i > 0 ? 'border-l border-gray-300 dark:border-gray-800 ' : '',
										i === selectedIndex ? 'outline' : ''
									)}
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
							{/snippet}
						</Tooltip>
					{/if}
				{/each}
			{:else}
				<!-- Single item case or inside a loop -->
				{#if selectedItem?.started_at}
					<Tooltip
						class={twMerge('h-full', isRunning(selectedItem) ? 'float-right' : 'float-left')}
						style="width: {(selectedLen / total) * 100}%"
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
