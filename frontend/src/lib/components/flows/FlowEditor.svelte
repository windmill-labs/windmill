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
	import { flowStateStore } from './flowState'
	import FlowModuleSchemaMap from './map/FlowModuleSchemaMap.svelte'

	const selectedIdStore = writable<string>('settings')
	const scheduleStore = writable<Schedule>(undefined)

	// Props
	export let path: string

	function select(selectedId: string) {
		selectedIdStore.set(selectedId)
	}

	setContext<FlowEditorContext>('FlowEditorContext', {
		selectedId: selectedIdStore,
		schedule: scheduleStore,
		select,
		path
	})

	$: {
		loadFlowSchedule(path, $workspaceStore)
			.then((schedule) => {
				scheduleStore.set(schedule)
			})
			.catch(() => {
				scheduleStore.set({
					cron: '0 */5 * * *',
					args: {},
					enabled: false,
					previewArgs: {}
				})
			})
	}
</script>

<FlowEditorHeader />

<div class="h-full overflow-hidden">
	<HSplitPane leftPaneSize="40%" rightPaneSize="60%" minLeftPaneSize="20%" minRightPaneSize="20%">
		<left slot="left" class="h-full ">
			<div class="h-full overflow-auto p-4 bg-gray-100">
				<FlowModuleSchemaMap bind:flowModuleSchemas={$flowStateStore.modules} />
			</div>
		</left>
		<right slot="right" class="h-full">
			<div class="h-full overflow-auto bg-white ">
				<FlowEditorPanel />
			</div>
		</right>
	</HSplitPane>
</div>
