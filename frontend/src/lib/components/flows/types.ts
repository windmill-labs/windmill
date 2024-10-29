import type { FlowModule, OpenFlow } from '$lib/gen'
import type { History } from '$lib/history'
import type { Writable } from 'svelte/store'
import type ScriptEditorDrawer from './content/ScriptEditorDrawer.svelte'
import type { FlowState } from './flowState'
import type { FlowBuilderWhitelabelCustomUi } from '../custom_ui'

export type InsertionMode = 'connect' | 'insert' | 'append'
export type SelectCallback = (detail: any) => boolean

export type PropPickerConfig = {
	propName: string
	insertionMode: InsertionMode
	onSelect: SelectCallback
}

export type PropPickerWrapper = {
	propPickerConfig: Writable<PropPickerConfig | undefined>
	focusProp: (propName: string, insertionMode: InsertionMode, onSelect: SelectCallback) => void
	clearFocus: () => void
}

export type FlowInput = Record<
	string,
	{
		connectingInputs?: PropPickerWrapper
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
	moving: Writable<{ module: FlowModule; modules: FlowModule[] } | undefined>
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
