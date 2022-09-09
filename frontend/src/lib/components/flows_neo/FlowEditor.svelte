<!-- Flow Editor: Top level component -->
<script lang="ts">
	import { HSplitPane } from 'svelte-split-pane'
	import { Pane, Splitpanes } from 'svelte-splitpanes'

	import { workspaceStore } from '$lib/stores'
	import { setContext } from 'svelte'
	import { writable } from 'svelte/store'
	import FlowEditorHeader from './header/FlowEditorHeader.svelte'
	import { loadFlowSchedule, type Schedule } from './scheduleUtils'

	import type { FlowEditorContext } from './types'
	import FlowEditorPanel from './content/FlowEditorPanel.svelte'
	import { flowStateStore } from '../flows/flowState'
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

	// Load the schedule given the path
	$: {
		loadFlowSchedule(path, $workspaceStore).then((schedule) => {
			scheduleStore.set(schedule)
		})
	}
</script>

<FlowEditorHeader />

<div class="h-full overflow-hidden">
	<Splitpanes>
		<Pane minSize={20}>
			<div class="h-full overflow-auto p-4 ">
				<FlowModuleSchemaMap bind:flowModuleSchemas={$flowStateStore} />
			</div>
		</Pane>
		<Pane minSize={40}>
			<div class="h-full overflow-auto bg-white ">
				<FlowEditorPanel />
			</div>
		</Pane>
	</Splitpanes>
</div>
