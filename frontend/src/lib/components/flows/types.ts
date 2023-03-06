import type { FlowModule } from '$lib/gen'
import type { Writable } from 'svelte/store'
import type ScriptEditorDrawer from './content/ScriptEditorDrawer.svelte'
import type { Schedule } from './scheduleUtils'

export type FlowEditorContext = {
	selectedId: Writable<string>
	moving: Writable<{ module: FlowModule, modules: FlowModule[] } | undefined>
	schedule: Writable<Schedule>,
	previewArgs: Writable<Record<string, any>>,
	scriptEditorDrawer: Writable<ScriptEditorDrawer | undefined>
}
