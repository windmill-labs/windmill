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

	const selectedIdStore = writable<string>('settings')
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
<div class="flex-1">
	<HSplitPane leftPaneSize="30%" rightPaneSize="70%" minLeftPaneSize="50px" minRightPaneSize="50px">
		<left slot="left">
			<div class="bg-gray-100 h-full">MINI MAP</div>
		</left>
		<right slot="right">
			<div class="h-full">
				<FlowEditorPanel />
			</div>
		</right>
	</HSplitPane>
</div>
