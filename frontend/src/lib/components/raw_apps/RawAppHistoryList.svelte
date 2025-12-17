<script lang="ts">
	import type { HistoryEntry, HistoryBranch } from './RawAppHistoryManager.svelte'
	import { classNames, displayDate } from '$lib/utils'
	import { GitBranch } from 'lucide-svelte'

	interface Props {
		entries: HistoryEntry[]
		branches: HistoryBranch[]
		selectedId: number | undefined
		onSelect: (id: number) => void
	}

	let { entries, branches, selectedId, onSelect }: Props = $props()

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

	// Entries in reverse order (newest first)
	const reversedEntries = $derived(entries.slice().reverse())
</script>

{#if entries.length === 0}
	<div class="text-tertiary py-2 text-center text-2xs">
		No snapshots yet. Auto-saved every 5 min.
	</div>
{:else}
	<div class="relative w-full">
		<!-- Timeline line -->
		<div class="absolute left-[0.95rem] top-2 bottom-2 w-px bg-gray-200 dark:bg-gray-700"></div>

		{#each reversedEntries as entry, i (entry.id)}
			{@const isSelected = selectedId === entry.id}
			{@const isFirst = i === 0}
			{@const entryBranches = branchesByForkPoint[entry.id] ?? []}

			<!-- Render branches ABOVE their fork point (newest first within branch) -->
			{#each entryBranches as branch (branch.id)}
				<div
					class="ml-4 relative border-l border-dashed border-gray-300 dark:border-gray-600 pl-2 my-1"
				>
					<div class="absolute left-3 bottom-2 text-tertiary">
						<GitBranch size={10} />
					</div>
					<!-- Branch entries in reverse order (newest first) -->
					{#each branch.entries.slice().reverse() as branchEntry (branchEntry.id)}
						{@const isBranchSelected = selectedId === branchEntry.id}
						<button
							onclick={() => onSelect(branchEntry.id)}
							class={classNames(
								'relative flex items-center gap-2 py-1 pr-1 pl-2 w-full text-left rounded transition-colors',
								'hover:bg-surface-hover',
								isBranchSelected ? 'bg-amber-50 dark:bg-amber-900/20' : ''
							)}
						>
							<!-- Branch dot -->
							<div
								class={classNames(
									'w-1 h-1 rounded-full',
									isBranchSelected
										? 'bg-amber-500 dark:bg-amber-400'
										: 'bg-gray-300 dark:bg-gray-600'
								)}
							></div>
							<span
								class={classNames(
									'text-2xs truncate',
									isBranchSelected
										? 'text-amber-600 dark:text-amber-400 font-medium'
										: 'text-tertiary'
								)}
							>
								{displayDate(branchEntry.timestamp.toISOString(), true, false)}
							</span>
						</button>
					{/each}
				</div>
			{/each}

			<!-- Main timeline entry -->
			<button
				onclick={() => onSelect(entry.id)}
				class={classNames(
					'relative flex items-center gap-2 py-1 pr-1 pl-3 w-full text-left rounded transition-colors',
					'hover:bg-surface-hover',
					isSelected ? 'bg-blue-50 dark:bg-blue-900/20' : ''
				)}
			>
				<!-- Timeline dot -->
				<div
					class={classNames(
						'absolute left-0 w-1.5 h-1.5 rounded-full border-[1.5px] bg-surface',
						isSelected
							? 'border-blue-500 dark:border-blue-400'
							: 'border-gray-300 dark:border-gray-600'
					)}
				></div>

				<span
					class={classNames(
						'text-2xs truncate',
						isSelected ? 'text-blue-600 dark:text-blue-400 font-medium' : 'text-secondary'
					)}
				>
					{#if isFirst && !isSelected}
						<span class="text-tertiary">Latest · </span>
					{/if}
					{displayDate(entry.timestamp.toISOString(), true, false)}
				</span>
			</button>
		{/each}
	</div>
{/if}
