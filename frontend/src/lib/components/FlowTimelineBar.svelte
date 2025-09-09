<script lang="ts">
	import { msToReadableTime, msToReadableTimeShort } from '$lib/utils'
	import { ZoomIn, ZoomOut } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { GlobalIterationBounds } from './graph'

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
		timelinelWidth: number
		showZoomButtons?: boolean
		onZoom?: () => void
		zoom?: 'in' | 'out'
		globalIterationBounds?: GlobalIterationBounds
		loadPreviousIterations?: () => void
	}

	let {
		total,
		min,
		items,
		selectedIndex,
		now,
		showSingleItem = true,
		timelinelWidth,
		showZoomButtons = false,
		onZoom,
		zoom = 'in',
		globalIterationBounds,
		loadPreviousIterations
	}: Props = $props()

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
	<div
		class="flex items-center gap-2 ml-auto min-w-32 max-w-[1000px] h-4 group"
		style="width: {timelinelWidth}px"
	>
		{#if showZoomButtons}
			<button
				onclick={(e) => {
					e.stopPropagation()
					onZoom?.()
				}}
				class="hover:text-primary hover:bg-surface p-1 -my-1 w-6 rounded-md flex items-center justify-center"
			>
				{#if zoom === 'in'}
					<ZoomOut size={12} />
				{:else}
					<ZoomIn size={12} />
				{/if}
			</button>
		{:else}
			<div class="w-6"></div>
		{/if}
		<div
			class="flex-1 h-1 bg-gray-300 dark:bg-gray-800 rounded-sm overflow-hidden group-hover:h-full transition-all duration-100"
		>
			{#if waitingLen > 100 && selectedItem.created_at}
				<div
					style="width: {((selectedItem.created_at - min) / total) * 100}%"
					class="h-full float-left"
				>
				</div>
				<div
					style="width: {(waitingLen / total) * 100}%"
					class="h-full float-left bg-gray-300 dark:bg-gray-600"
					title={msToReadableTime(waitingLen, 1)}
				>
				</div>
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
							'h-full hover:outline outline-1 outline-white -outline-offset-1 rounded-sm',
							isRunning(selectedItem) ? 'float-right bg-blue-400' : 'float-left bg-blue-500'
						)}
						title={msToReadableTime(selectedLen, 1)}
					></div>
				{/if}
			{:else}
				<!-- All iterations -->

				{#if globalIterationBounds && globalIterationBounds.iteration_from && globalIterationBounds.iteration_from > 0}
					<button
						style="width: 56px; background: linear-gradient(to left, rgb(59 130 246) 0%, oklch(27.8% 0.033 256.848) 100%)"
						class={twMerge(
							'h-full group hover:outline outline-1 outline-white -outline-offset-1 rounded-sm',
							'float-left'
						)}
						onclick={(e) => {
							e.stopPropagation()
							loadPreviousIterations?.()
						}}
					>
						+</button
					>
				{/if}
				{#each items as item, i}
					{#if item.started_at}
						<div
							style="width: {(getGap(items, i) / total) * 100}%"
							class={twMerge('h-full float-left bg-gray-300 dark:bg-gray-800')}
						></div>

						<div
							style="width: {(getLength(item) / total) * 100}%"
							class={twMerge(
								'h-full group hover:outline outline-1 outline-white -outline-offset-1 rounded-sm',
								isRunning(item) ? 'float-right bg-blue-400' : 'float-left bg-blue-500',
								i > 0 ? 'border-l border-gray-300 dark:border-gray-800 ' : ''
							)}
							title={`Ran in ${msToReadableTime(getLength(item), 1)}`}
						></div>
					{/if}
				{/each}
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
