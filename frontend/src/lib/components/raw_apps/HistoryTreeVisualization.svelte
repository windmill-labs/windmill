<script lang="ts">
	import type { HistoryEntry, HistoryBranch } from './RawAppHistoryManager.svelte'
	import { classNames, displayDate } from '$lib/utils'
	import { GitBranch, CircleDot } from 'lucide-svelte'
	import { fade, fly } from 'svelte/transition'
	import { cubicOut } from 'svelte/easing'

	interface Props {
		entries: HistoryEntry[]
		branches: HistoryBranch[]
		selectedId: number | undefined
		onSelect: (id: number) => void
		onSelectCurrent?: () => void
	}

	let { entries, branches, selectedId, onSelect, onSelectCurrent }: Props = $props()

	// Constants for layout
	const LINE_X = 20 // X position of main timeline line
	const BRANCH_OFFSET_X = 40 // How far branches are offset horizontally
	const ENTRY_HEIGHT = 32 // Height of each entry
	const CURRENT_HEIGHT = 40 // Height of current state button

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

	// Calculate positions for rendering
	const entryPositions = $derived.by(() => {
		const positions: Record<number, { x: number; y: number; index: number }> = {}
		let currentY = CURRENT_HEIGHT + 16 // Start below current button

		entries.forEach((entry, index) => {
			positions[entry.id] = { x: LINE_X, y: currentY, index }
			currentY += ENTRY_HEIGHT

			// Add space for branches if they exist
			const entryBranches = branchesByForkPoint[entry.id] ?? []
			entryBranches.forEach((branch) => {
				branch.entries.forEach(() => {
					currentY += ENTRY_HEIGHT
				})
				currentY += 16 // Extra space after branch
			})
		})

		return positions
	})

	// Calculate branch positions
	const branchPositions = $derived.by(() => {
		const positions: Record<
			number,
			Array<{ id: number; x: number; y: number; branchId: number }>
		> = {}

		Object.entries(branchesByForkPoint).forEach(([forkPointId, forkBranches]) => {
			const forkPoint = entryPositions[Number(forkPointId)]
			if (!forkPoint) return

			positions[Number(forkPointId)] = []
			let branchY = forkPoint.y + ENTRY_HEIGHT

			forkBranches.forEach((branch) => {
				branch.entries.forEach((entry) => {
					positions[Number(forkPointId)].push({
						id: entry.id,
						x: LINE_X + BRANCH_OFFSET_X,
						y: branchY,
						branchId: branch.id
					})
					branchY += ENTRY_HEIGHT
				})
				branchY += 16 // Extra space after branch
			})
		})

		return positions
	})

	// Helper to create SVG path for branch connection
	function createBranchPath(startX: number, startY: number, endX: number, endY: number): string {
		const controlPointOffset = 20
		return `M ${startX} ${startY} C ${startX + controlPointOffset} ${startY}, ${
			endX - controlPointOffset
		} ${endY}, ${endX} ${endY}`
	}

	// Custom draw function for smoother animation
	function drawPath(node: SVGPathElement, { duration = 600 } = {}) {
		const length = node.getTotalLength()

		return {
			duration,
			css: (t: number) => {
				const eased = cubicOut(t)
				return `
					stroke-dasharray: ${length};
					stroke-dashoffset: ${length * (1 - eased)};
				`
			}
		}
	}
</script>

<div class="relative w-full min-h-[200px]" role="list" aria-label="History timeline">
	<!-- SVG for lines and connections -->
	<svg class="absolute inset-0 pointer-events-none" style="width: 100%; height: 100%;">
		<!-- Main timeline line (green gradient) -->
		<defs>
			<linearGradient id="mainLineGradient" x1="0%" y1="0%" x2="0%" y2="100%">
				<stop offset="0%" style="stop-color:rgb(34, 197, 94);stop-opacity:1" />
				<stop offset="100%" style="stop-color:rgb(34, 197, 94);stop-opacity:0.3" />
			</linearGradient>
		</defs>

		{#if entries.length > 0}
			{@const lastEntry = entries[entries.length - 1]}
			{@const lastY = entryPositions[lastEntry.id]?.y ?? 100}
			<line
				x1={LINE_X}
				y1="10"
				x2={LINE_X}
				y2={lastY + 20}
				stroke="url(#mainLineGradient)"
				stroke-width="2"
				class="transition-all duration-300"
			/>
		{/if}

		<!-- Branch connection lines -->
		{#each Object.entries(branchPositions) as [forkPointId, branchNodes] (forkPointId)}
			{@const forkPoint = entryPositions[Number(forkPointId)]}
			{#if forkPoint}
				{#each branchNodes as node (node.id)}
					<path
						d={createBranchPath(forkPoint.x, forkPoint.y, node.x, node.y)}
						stroke="rgb(251, 146, 60)"
						stroke-width="1.5"
						fill="none"
						stroke-dasharray="4 2"
						opacity="0.6"
						in:drawPath={{ duration: 600 }}
						class="transition-opacity duration-300"
					/>
				{/each}
			{/if}
		{/each}
	</svg>

	<!-- Interactive elements -->
	<div class="relative">
		<!-- Current Working State -->
		<button
			onclick={() => onSelectCurrent?.()}
			aria-label="Current working state"
			aria-current={selectedId === undefined ? 'true' : 'false'}
			class={classNames(
				'absolute flex items-center gap-2 px-3 py-1.5 rounded-lg transition-all duration-200 transform',
				'hover:scale-105 hover:shadow-md',
				selectedId === undefined
					? 'bg-green-500 dark:bg-green-600 text-white shadow-lg scale-105'
					: 'bg-surface hover:bg-surface-hover'
			)}
			style="left: {LINE_X - 8}px; top: 0;"
			in:fly={{ y: -20, duration: 300 }}
		>
			<div class="relative">
				{#if selectedId === undefined}
					<div
						class="absolute inset-0 bg-green-400 rounded-full animate-ping"
						style="animation-duration: 2s;"
					></div>
				{/if}
				<CircleDot
					size={12}
					class={selectedId === undefined ? 'text-white' : 'text-green-500 dark:text-green-400'}
				/>
			</div>
			<span
				class={classNames(
					'text-xs font-medium',
					selectedId === undefined ? 'text-white' : 'text-green-600 dark:text-green-400'
				)}
			>
				Current
			</span>
		</button>

		<!-- Main timeline entries -->
		{#each entries as entry (entry.id)}
			{@const pos = entryPositions[entry.id]}
			{@const isSelected = selectedId === entry.id}
			{@const entryBranches = branchesByForkPoint[entry.id] ?? []}

			{#if pos}
				<!-- Main entry button -->
				<button
					onclick={() => onSelect(entry.id)}
					aria-label={`Snapshot from ${displayDate(entry.timestamp.toISOString(), true, false)}`}
					aria-current={isSelected ? 'true' : 'false'}
					class={classNames(
						'absolute flex items-center gap-2 px-2 py-1 rounded-lg transition-all duration-200',
						'hover:bg-surface-hover hover:scale-105',
						isSelected ? 'bg-blue-50 dark:bg-blue-900/20 shadow-md scale-105' : ''
					)}
					style="left: {pos.x - 8}px; top: {pos.y - 12}px;"
					in:fade={{ duration: 200 }}
				>
					<!-- Timeline dot -->
					<div class="relative">
						<div
							class={classNames(
								'w-4 h-4 rounded-full border-2 bg-surface transition-all duration-200',
								isSelected
									? 'border-blue-500 dark:border-blue-400 shadow-sm'
									: 'border-gray-400 dark:border-gray-500'
							)}
						>
							{#if entryBranches.length > 0}
								<div
									class="absolute -bottom-1 -right-1 text-amber-500 dark:text-amber-400"
									in:fade={{ duration: 200 }}
								>
									<GitBranch size={8} />
								</div>
							{/if}
						</div>
					</div>
					<span
						class={classNames(
							'text-2xs',
							isSelected ? 'text-blue-600 dark:text-blue-400 font-medium' : 'text-tertiary'
						)}
					>
						{displayDate(entry.timestamp.toISOString(), true, false)}
					</span>
				</button>

				<!-- Branch entries -->
				{#if entryBranches.length > 0}
					{#each branchPositions[entry.id] ?? [] as branchNode (branchNode.id)}
						{@const branchEntry = branches
							.flatMap((b) => b.entries)
							.find((e) => e.id === branchNode.id)}
						{@const isBranchSelected = selectedId === branchNode.id}
						{#if branchEntry}
							<button
								onclick={() => onSelect(branchNode.id)}
								aria-label={`Branch snapshot from ${displayDate(
									branchEntry.timestamp.toISOString(),
									true,
									false
								)}`}
								aria-current={isBranchSelected ? 'true' : 'false'}
								class={classNames(
									'absolute flex items-center gap-2 px-2 py-1 rounded-lg transition-all duration-200',
									'hover:bg-surface-hover hover:scale-105',
									isBranchSelected ? 'bg-amber-50 dark:bg-amber-900/20 shadow-md scale-105' : ''
								)}
								style="left: {branchNode.x - 8}px; top: {branchNode.y - 12}px;"
								in:fly={{ x: -20, duration: 400, delay: 100 }}
							>
								<!-- Branch dot -->
								<div
									class={classNames(
										'w-3 h-3 rounded-full transition-all duration-200',
										isBranchSelected
											? 'bg-amber-500 dark:bg-amber-400 shadow-sm'
											: 'bg-gray-300 dark:bg-gray-600'
									)}
								></div>
								<span
									class={classNames(
										'text-2xs',
										isBranchSelected
											? 'text-amber-600 dark:text-amber-400 font-medium'
											: 'text-tertiary'
									)}
								>
									{displayDate(branchEntry.timestamp.toISOString(), true, false)}
								</span>
							</button>
						{/if}
					{/each}
				{/if}
			{/if}
		{/each}
	</div>
</div>

<style>
	@keyframes ping {
		75%,
		100% {
			transform: scale(2);
			opacity: 0;
		}
	}

	.animate-ping {
		animation: ping 2s cubic-bezier(0, 0, 0.2, 1) infinite;
	}
</style>
