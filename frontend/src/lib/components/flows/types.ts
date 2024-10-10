import type { FlowModule, OpenFlow } from '$lib/gen'
import type { History } from '$lib/history'
import type { Writable } from 'svelte/store'
import type ScriptEditorDrawer from './content/ScriptEditorDrawer.svelte'
import type { FlowState } from './flowState'
import type { FlowBuilderWhitelabelCustomUi } from '../custom_ui'
import { type HttpTrigger } from '$lib/gen'
import type { Schedule } from '$lib/gen'
import type { ScheduleTrigger } from '../triggers'

export type FlowInput = Record<
	string,
	{
		flowStepWarnings?: Record<
			string,
			{
				message: string
				type: 'error' | 'warning'
			}
		>
	}
>

export type ExtendedOpenFlow = OpenFlow & {
	tag?: string
	ws_error_handler_muted?: boolean
	dedicated_worker?: boolean
	visible_to_runner_only?: boolean
}

export type FlowEditorContext = {
	selectedId: Writable<string>
	selectedTrigger: Writable<string>
	httpTriggers: Writable<(HttpTrigger & { canWrite: boolean })[] | undefined>
	moving: Writable<{ module: FlowModule; modules: FlowModule[] } | undefined>
	schedule: Writable<ScheduleTrigger>
	primarySchedule: Writable<Schedule | undefined | boolean>
	schedules: Writable<Schedule[] | undefined>
	previewArgs: Writable<Record<string, any>>
	scriptEditorDrawer: Writable<ScriptEditorDrawer | undefined>
	history: History<OpenFlow>
	pathStore: Writable<string>
	flowStore: Writable<ExtendedOpenFlow>
	flowStateStore: Writable<FlowState>
	testStepStore: Writable<Record<string, any>>
	saveDraft: () => void
	initialPath: string
	flowInputsStore: Writable<FlowInput>
	customUi: FlowBuilderWhitelabelCustomUi
	insertButtonOpen: Writable<boolean>
}
