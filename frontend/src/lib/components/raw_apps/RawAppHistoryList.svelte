<script lang="ts">
	import type { HistoryEntry } from './RawAppHistoryManager.svelte'
	import { displayDate, classNames, sendUserToast } from '$lib/utils'
	import { Clock, Camera } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'

	interface Props {
		entries: HistoryEntry[]
		selectedIndex: number | undefined
		onSelect: (index: number) => void
		onManualSnapshot: () => void
	}

	let { entries, selectedIndex, onSelect, onManualSnapshot }: Props = $props()

	const reversedEntries = $derived(entries.slice().reverse())

	function handleManualSnapshot() {
		onManualSnapshot()
		sendUserToast('Snapshot created manually')
	}

	function getOriginalIndex(reversedIndex: number): number {
		return entries.length - 1 - reversedIndex
	}
</script>

<div class="flex flex-col h-full">
	<div class="p-4 border-b border-gray-200 dark:border-gray-700">
		<Button
			size="xs"
			color="dark"
			variant="border"
			startIcon={{ icon: Camera }}
			on:click={handleManualSnapshot}
			btnClasses="w-full"
		>
			Create Snapshot
		</Button>
		<div class="text-xs text-secondary mt-2 text-center">
			{entries.length} / 50 snapshots
		</div>
	</div>

	<div class="flex-1 overflow-y-auto p-2">
		{#if entries.length === 0}
			<div class="text-secondary p-4 text-center text-sm">
				<div class="mb-2">No history entries yet.</div>
				<div class="text-xs">Snapshots are created automatically every 5 minutes.</div>
			</div>
		{:else}
			<div class="flex flex-col gap-2">
				{#each reversedEntries as entry, reversedIndex (entry.timestamp.getTime())}
					{@const originalIndex = getOriginalIndex(reversedIndex)}
					{@const fileCount = Object.keys(entry.files).length}
					{@const runnableCount = Object.keys(entry.runnables).length}
					{@const isSelected = selectedIndex === originalIndex}

					<button
						onclick={() => onSelect(originalIndex)}
						class={classNames(
							'border flex flex-col gap-1 p-3 rounded-md cursor-pointer transition-colors text-left',
							'hover:bg-surface-hover',
							isSelected
								? 'bg-surface-selected text-primary border-blue-500 dark:border-blue-400'
								: 'border-gray-200 dark:border-gray-700'
						)}
					>
						<div class="flex items-center gap-2 text-sm">
							<Clock size={14} class="flex-shrink-0" />
							<span class="font-medium truncate">
								{displayDate(entry.timestamp.toISOString())}
							</span>
						</div>
						<div class="text-xs text-secondary truncate">
							{entry.summary || 'Untitled App'} · {fileCount} files · {runnableCount} runnables
						</div>
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>
