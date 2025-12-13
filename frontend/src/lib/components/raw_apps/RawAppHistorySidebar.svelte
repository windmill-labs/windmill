<script lang="ts">
	import RawAppHistoryList from './RawAppHistoryList.svelte'
	import type { RawAppHistoryManager } from './RawAppHistoryManager.svelte'

	interface Props {
		historyManager: RawAppHistoryManager
		onSelect: (index: number) => void
		onManualSnapshot: () => void
		selectedId: number | undefined
	}

	let { historyManager, onSelect, onManualSnapshot, selectedId }: Props = $props()
</script>

<div class="h-full flex flex-col border-r border-gray-200 dark:border-gray-700">
	<RawAppHistoryList
		entries={historyManager.allEntries}
		{selectedId}
		currentEntryId={historyManager.currentEntryId}
		{onSelect}
		{onManualSnapshot}
	/>
</div>
