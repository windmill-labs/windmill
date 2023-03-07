import type { Flow, FlowModule } from '$lib/gen'
import type { History } from '$lib/history'
import type { Writable } from 'svelte/store'
import type ScriptEditorDrawer from './content/ScriptEditorDrawer.svelte'
import type { FlowState } from './flowState'
import type { Schedule } from './scheduleUtils'

export type FlowEditorContext = {
	selectedId: Writable<string>
	moving: Writable<{ module: FlowModule, modules: FlowModule[] } | undefined>
	schedule: Writable<Schedule>,
	previewArgs: Writable<Record<string, any>>,
	scriptEditorDrawer: Writable<ScriptEditorDrawer | undefined>,
	history: History<Flow>,
	flowStore: Writable<Flow>,
	flowStateStore: Writable<FlowState>,
	testStepStore: Writable<Record<string, any>>
}
