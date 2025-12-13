<script lang="ts">
	import type { HistoryEntry } from './RawAppHistoryManager.svelte'
	import { displayDate, classNames, sendUserToast } from '$lib/utils'
	import { Clock, Camera, History } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'

	interface Props {
		entries: HistoryEntry[]
		selectedId: number | undefined
		currentEntryId: number | undefined
		onSelect: (id: number) => void
		onManualSnapshot: () => void
	}

	let { entries, selectedId, currentEntryId, onSelect, onManualSnapshot }: Props = $props()

	const reversedEntries = $derived(entries.slice().reverse())

	function handleManualSnapshot() {
		onManualSnapshot()
		sendUserToast('Snapshot created manually')
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
				{#each reversedEntries as entry (entry.id)}
					{@const fileCount = Object.keys(entry.files).length}
					{@const runnableCount = Object.keys(entry.runnables).length}
					{@const isPreviewSelected = selectedId === entry.id}
					{@const isCurrentEntry = currentEntryId === entry.id}

					<button
						onclick={() => onSelect(entry.id)}
						class={classNames(
							'border flex flex-col gap-1 p-3 rounded-md cursor-pointer transition-colors text-left',
							'hover:bg-surface-hover',
							isPreviewSelected
								? 'bg-surface-selected text-primary border-blue-500 dark:border-blue-400'
								: isCurrentEntry
									? 'bg-emerald-50 dark:bg-emerald-900/20 text-primary border-emerald-500 dark:border-emerald-400'
									: 'border-gray-200 dark:border-gray-700'
						)}
					>
						<div class="flex items-center gap-2 text-sm">
							{#if isCurrentEntry}
								<History size={14} class="flex-shrink-0 text-emerald-600 dark:text-emerald-400" />
							{:else}
								<Clock size={14} class="flex-shrink-0" />
							{/if}
							<span class="font-medium truncate">
								#{entry.id} · {displayDate(entry.timestamp.toISOString(), true, false)}
							</span>
						</div>
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>
