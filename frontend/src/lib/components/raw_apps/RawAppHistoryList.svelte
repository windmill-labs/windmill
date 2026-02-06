<script lang="ts">
	import type { HistoryEntry, HistoryBranch } from './RawAppHistoryManager.svelte'
	import { displayDate } from '$lib/utils'
	import { Circle, CircleDot, CircleDotDashed, CircleDashed } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		entries: HistoryEntry[]
		branches: HistoryBranch[]
		selectedId: number | undefined
		onSelect: (id: number) => void
		onSelectCurrent?: () => void
	}

	let { entries, branches, selectedId, onSelect, onSelectCurrent }: Props = $props()

	// Constants for layout
	const LINE_X = 8 // X position of main timeline line (center of dots)
	const BRANCH_OFFSET_X = 45 // How far branches are offset horizontally
	const ENTRY_HEIGHT = 28 // Height of each entry
	const CURRENT_HEIGHT = 32 // Height of current state button

	// Build a map of fork points to their branches for rendering
	const branchesByForkPoint = $derived(
		branches.reduce(
			(acc, branch) => {
				if (!acc[branch.forkPointId]) {
					acc[branch.forkPointId] = []
				}
				acc[branch.forkPointId].push(branch)
				return acc
			},
			{} as Record<number, HistoryBranch[]>
		)
	)

	// Entries in reverse order (newest first) for display
	const reversedEntries = $derived(entries.slice().reverse())

	// Calculate positions for rendering
	const entryPositions = $derived.by(() => {
		const positions: Record<number, { x: number; y: number; index: number }> = {}

		let currentY = CURRENT_HEIGHT + 12 // Start below current button

		reversedEntries.forEach((entry, index) => {
			// For each entry, calculate space needed for branches above it
			const entryBranches = branchesByForkPoint[entry.id] ?? []
			const branchSpaceAbove = entryBranches.reduce((total, branch) => {
				return total + branch.entries.length * ENTRY_HEIGHT
			}, 0)

			// Add space for branches above this entry
			currentY += branchSpaceAbove

			// Position this entry
			positions[entry.id] = { x: LINE_X, y: currentY, index }

			// Move to next entry position
			currentY += ENTRY_HEIGHT
		})

		return positions
	})

	// Calculate total height needed for the container
	const totalHeight = $derived.by(() => {
		const mainTimelineMaxY = Object.values(entryPositions).reduce(
			(max, pos) => Math.max(max, pos.y),
			0
		)
		const branchMaxY = Object.values(branchPositions)
			.flatMap((groups) => Object.values(groups).flatMap((nodes) => nodes.map((node) => node.y)))
			.reduce((max, y) => Math.max(max, y), 0)
		return Math.max(mainTimelineMaxY, branchMaxY, 150) + 20 // Add padding at bottom
	})

	// Calculate branch positions
	const branchPositions = $derived.by(() => {
		const positions: Record<
			number,
			Record<number, Array<{ id: number; x: number; y: number }>>
		> = {}

		Object.entries(branchesByForkPoint).forEach(([forkPointId, forkBranches]) => {
			const forkPoint = entryPositions[Number(forkPointId)]
			if (!forkPoint) return

			positions[Number(forkPointId)] = {}

			forkBranches.forEach((branch) => {
				positions[Number(forkPointId)][branch.id] = []

				// Branch flows from fork point upward (toward newer/current)
				// Start just above the fork point and continue upward
				// Oldest branch entry is closest to fork point, newest is furthest
				let currentY = forkPoint.y - ENTRY_HEIGHT

				branch.entries.forEach((entry) => {
					positions[Number(forkPointId)][branch.id].push({
						id: entry.id,
						x: LINE_X + BRANCH_OFFSET_X,
						y: currentY
					})
					currentY -= ENTRY_HEIGHT // Move upward (decreasing Y)
				})
			})
		})

		return positions
	})

	// Helper to create SVG path for branch connection
	function createBranchPath(startX: number, startY: number, endX: number, endY: number): string {
		// Control points are vertically offset to create vertical tangents
		const controlOffset = Math.abs(startY - endY) * 0.85 // 85% of vertical distance for very intense curve
		// Both tangents point upward (toward smaller Y values)
		// Start control point: above the start point
		// End control point: above the end point (since endY < startY, we add to go further up)
		return `M ${startX} ${startY} C ${startX} ${startY - controlOffset}, ${endX} ${endY + controlOffset}, ${endX} ${endY}`
	}
</script>

{#snippet timelineButton(
	label: string,
	timestamp: string | null,
	isSelected: boolean,
	onClick: () => void,
	position: { x: number; y: number },
	type: 'current' | 'main' | 'branch' = 'main'
)}
	{@const colors = {
		current: {
			selected: 'text-accent',
			default: 'text-primary',
			textSelected: 'text-accent',
			bgSelected: 'bg-surface-accent-selected'
		},
		main: {
			selected: 'text-accent',
			default: 'text-primary',
			textSelected: 'text-accent',
			bgSelected: 'bg-surface-accent-selected'
		},
		branch: {
			selected: 'text-amber-500 dark:text-amber-400',
			default: 'text-secondary',
			textSelected: 'text-amber-600 dark:text-amber-400',
			bgSelected: 'bg-amber-50 dark:bg-amber-900/20'
		}
	}}
	{@const color = colors[type]}

	<button
		onclick={onClick}
		aria-label={timestamp ? `Snapshot from ${displayDate(timestamp, true, false)}` : label}
		aria-current={isSelected ? 'true' : 'false'}
		class={'absolute flex items-center gap-1 transition-all duration-200 animate-fadeIn'}
		style="left: {position.x - 6}px; top: {position.y - 6}px;"
	>
		{#if isSelected}
			{#if type === 'current'}
				<CircleDotDashed size={12} class="bg-surface {color.selected}" />
			{:else}
				<CircleDot size={12} class="bg-surface {color.selected}" />
			{/if}
		{:else if type === 'current'}
			<CircleDashed size={12} class="bg-surface {color.default}" />
		{:else}
			<Circle size={12} class="bg-surface {color.default}" />
		{/if}
		<span
			class={twMerge(
				'text-2xs truncate px-2 py-1 rounded-md',
				type === 'current' ? 'font-semibold text-emphasis' : 'font-normal',
				isSelected ? color.textSelected : 'text-tertiary',
				isSelected ? color.bgSelected : 'hover:bg-surface-hover'
			)}
		>
			{timestamp ? displayDate(timestamp, true, false) : label}
		</span>
	</button>
{/snippet}

{#if entries.length === 0}
	<div class="text-tertiary py-2 text-center text-2xs">
		No snapshots yet. Auto-saved every 5 min.
	</div>
{:else}
	<div
		class="relative w-full"
		style="height: {totalHeight}px; transition: height 0.3s cubic-bezier(0.4, 0, 0.2, 1);"
		role="list"
		aria-label="History timeline"
	>
		<!-- SVG for lines and connections -->
		<svg class="absolute inset-0 pointer-events-none" style="width: 100%; height: 100%;">
			<!-- Always show the main line if there are entries -->
			{#if reversedEntries.length > 0}
				{@const mainTimelineMaxY = Math.max(...Object.values(entryPositions).map((pos) => pos.y))}
				{@const branchMaxY = Object.values(branchPositions)
					.flatMap((groups) =>
						Object.values(groups).flatMap((nodes) => nodes.map((node) => node.y))
					)
					.reduce((max, y) => Math.max(max, y), 0)}
				{@const maxY = Math.max(mainTimelineMaxY, branchMaxY)}
				<line
					x1={LINE_X}
					y1={8}
					x2={LINE_X}
					y2={maxY + 8}
					stroke="currentColor"
					stroke-width="1"
					opacity="1"
					class="text-primary"
					style="transition: y2 0.3s cubic-bezier(0.4, 0, 0.2, 1)"
				/>
			{/if}

			<!-- Branch connection lines -->
			{#each Object.entries(branchPositions) as [forkPointId, branchGroups] (forkPointId)}
				{@const forkPoint = entryPositions[Number(forkPointId)]}
				{#if forkPoint}
					{#each Object.entries(branchGroups) as [branchId, branchNodes] (branchId)}
						{#if branchNodes.length > 0}
							<!-- Connection from fork point to first branch entry -->
							<path
								d={createBranchPath(forkPoint.x, forkPoint.y, branchNodes[0].x, branchNodes[0].y)}
								stroke="currentColor"
								stroke-width="1.5"
								fill="none"
								stroke-dasharray="4 2"
								opacity="0.6"
								class="text-secondary transition-all duration-300"
							/>

							<!-- Vertical line connecting branch entries within this branch -->
							{#if branchNodes.length > 1}
								{@const firstNode = branchNodes[0]}
								{@const lastNode = branchNodes[branchNodes.length - 1]}
								<line
									x1={firstNode.x}
									y1={firstNode.y}
									x2={lastNode.x}
									y2={lastNode.y}
									stroke="currentColor"
									stroke-width="1.5"
									stroke-dasharray="4 2"
									opacity="0.6"
									class="text-secondary"
									style="transition: y1 0.3s cubic-bezier(0.4, 0, 0.2, 1), y2 0.3s cubic-bezier(0.4, 0, 0.2, 1)"
								/>
							{/if}
						{/if}
					{/each}
				{/if}
			{/each}
		</svg>

		<!-- Interactive elements -->
		<div class="relative">
			<!-- Current Working State -->
			{@render timelineButton(
				'Current',
				null,
				selectedId === undefined,
				() => onSelectCurrent?.(),
				{ x: LINE_X, y: 6 },
				'current'
			)}

			<!-- Main timeline entries -->
			{#each reversedEntries as entry (entry.id)}
				{@const pos = entryPositions[entry.id]}
				{@const isSelected = selectedId === entry.id}
				{@const entryBranches = branchesByForkPoint[entry.id] ?? []}

				{#if pos}
					<!-- Main entry button -->
					{@render timelineButton(
						'',
						entry.timestamp.toISOString(),
						isSelected,
						() => onSelect(entry.id),
						pos,
						'main'
					)}

					<!-- Branch entries -->
					{#if entryBranches.length > 0}
						{#each Object.values(branchPositions[entry.id] ?? {}) as branchNodes}
							{#each branchNodes as branchNode (branchNode.id)}
								{@const branchEntry = branches
									.flatMap((b) => b.entries)
									.find((e) => e.id === branchNode.id)}
								{@const isBranchSelected = selectedId === branchNode.id}
								{#if branchEntry}
									{@render timelineButton(
										'',
										branchEntry.timestamp.toISOString(),
										isBranchSelected,
										() => onSelect(branchNode.id),
										{ x: branchNode.x, y: branchNode.y - 4 },
										'branch'
									)}
								{/if}
							{/each}
						{/each}
					{/if}
				{/if}
			{/each}
		</div>
	</div>
{/if}

<style>
	@keyframes fadeIn {
		from {
			opacity: 0;
			transform: translateY(-10px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.animate-fadeIn {
		animation: fadeIn 0.3s cubic-bezier(0.4, 0, 0.2, 1) forwards;
	}
</style>
