import type { FlowModule, OpenFlow } from '$lib/gen'
import type { History } from '$lib/history'
import type { Writable } from 'svelte/store'
import type ScriptEditorDrawer from './content/ScriptEditorDrawer.svelte'
import type { FlowState } from './flowState'
import type { Schedule } from './scheduleUtils'

export type FlowEditorContext = {
	selectedId: Writable<string>
	moving: Writable<{ module: FlowModule; modules: FlowModule[] } | undefined>
	schedule: Writable<Schedule>
	previewArgs: Writable<Record<string, any>>
	scriptEditorDrawer: Writable<ScriptEditorDrawer | undefined>
	history: History<OpenFlow>
	pathStore: Writable<string>
	flowStore: Writable<
		OpenFlow & { tag?: string; ws_error_handler_muted?: boolean; dedicated_worker?: boolean }
	>
	flowStateStore: Writable<FlowState>
	testStepStore: Writable<Record<string, any>>
	saveDraft: () => void
	initialPath: string
}
