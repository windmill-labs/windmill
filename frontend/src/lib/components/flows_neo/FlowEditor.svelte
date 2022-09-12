<!-- Flow Editor: Top level component -->
<script lang="ts">
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
	<Splitpanes>
		<Pane minSize={20} size={20}>
			<div class="h-full overflow-auto p-4 ">
				<FlowModuleSchemaMap bind:flowModuleSchemas={$flowStateStore.modules} />
			</div>
		</Pane>
		<Pane minSize={40} size={60}>
			<div class="h-full overflow-auto bg-white ">
				<FlowEditorPanel />
			</div>
		</Pane>
	</Splitpanes>
</div>
