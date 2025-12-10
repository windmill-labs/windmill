<script lang="ts">
	import RawAppHistoryList from './RawAppHistoryList.svelte'
	import type { RawAppHistoryManager } from './RawAppHistoryManager.svelte'

	interface Props {
		historyManager: RawAppHistoryManager
		onSelect: (index: number) => void
		onManualSnapshot: () => void
		selectedIndex: number | undefined
	}

	let { historyManager, onSelect, onManualSnapshot, selectedIndex }: Props = $props()
</script>

<div class="h-full flex flex-col border-r border-gray-200 dark:border-gray-700">
	<RawAppHistoryList
		entries={historyManager.allEntries}
		{selectedIndex}
		{onSelect}
		{onManualSnapshot}
	/>
</div>
