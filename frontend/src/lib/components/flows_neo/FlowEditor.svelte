<!-- Flow Editor: Top level component -->
<script lang="ts">
	import { HSplitPane } from 'svelte-split-pane'

	import { workspaceStore } from '$lib/stores'
	import { setContext } from 'svelte'
	import { writable } from 'svelte/store'
	import FlowEditorHeader from './header/FlowEditorHeader.svelte'
	import { loadFlowSchedule, type Schedule } from './scheduleUtils'

	import type { FlowEditorContext } from './types'
	import FlowEditorPanel from './content/FlowEditorPanel.svelte'

	const selectedIdStore = writable<string>('0')
	const scheduleStore = writable<Schedule>(undefined)

	// Props
	export let path: string

	function select(selectedId: string) {
		selectedIdStore.set(selectedId)
	}

	setContext<FlowEditorContext>('FlowEditorContext', {
		selectedId: $selectedIdStore,
		schedule: $scheduleStore,
		select,
		path
	})

	// Load the schedule given the path
	$: {
		loadFlowSchedule(path, $workspaceStore).then((schedule) => {
			scheduleStore.set(schedule)
		})
	}
</script>

<FlowEditorHeader />
<HSplitPane leftPaneSize="50%" rightPaneSize="50%" minLeftPaneSize="20%" minRightPaneSize="20%">
	<left slot="left">
		<div class="bg-gray-100 h-full">MINI MAP</div>
	</left>
	<right slot="right">
		<FlowEditorPanel />
	</right>
</HSplitPane>
