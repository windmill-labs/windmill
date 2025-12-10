<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Drawer from '../common/drawer/Drawer.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import RawAppHistoryList from './RawAppHistoryList.svelte'
	import RawAppHistoryPreview from './RawAppHistoryPreview.svelte'
	import type { RawAppHistoryManager } from './RawAppHistoryManager.svelte'

	interface Props {
		open: boolean
		historyManager: RawAppHistoryManager
		onRestore: (index: number) => void
		onManualSnapshot: () => void
	}

	let { open = $bindable(), historyManager, onRestore, onManualSnapshot }: Props = $props()

	let selectedIndex = $state<number | undefined>(undefined)
	let drawer: Drawer | undefined = $state(undefined)

	const selectedEntry = $derived(
		selectedIndex !== undefined ? historyManager.getEntry(selectedIndex) : undefined
	)

	function handleSelect(index: number) {
		selectedIndex = index
		historyManager.setPreview(index)
	}

	function handleRestore() {
		if (selectedIndex !== undefined) {
			onRestore(selectedIndex)
			// Close drawer - cleanup will happen in onClose callback
			drawer?.closeDrawer()
		}
	}

	function handleClose() {
		// Clear preview state after drawer animation completes
		selectedIndex = undefined
		historyManager.clearPreview()
	}

	// Keyboard shortcut
	function handleKeydown(e: KeyboardEvent) {
		if (open && e.key === 'Escape') {
			drawer?.closeDrawer()
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<Drawer bind:this={drawer} bind:open size="800px" placement="left">
	<DrawerContent title="History" on:close={handleClose}>
		<div class="h-full flex flex-col">
			<Splitpanes class="flex-1">
				<Pane size={30} minSize={20} maxSize={50}>
					<RawAppHistoryList
						entries={historyManager.allEntries}
						{selectedIndex}
						onSelect={handleSelect}
						{onManualSnapshot}
					/>
				</Pane>
				<Pane size={70}>
					<RawAppHistoryPreview entry={selectedEntry} onRestore={handleRestore} />
				</Pane>
			</Splitpanes>
		</div>
	</DrawerContent>
</Drawer>
