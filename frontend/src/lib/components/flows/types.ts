import type { FlowModule, OpenFlow } from '$lib/gen'
import type { History } from '$lib/history'
import type { Writable } from 'svelte/store'
import type ScriptEditorDrawer from './content/ScriptEditorDrawer.svelte'
import type { FlowState } from './flowState'
import type { FlowBuilderWhitelabelCustomUi } from '../custom_ui'
import type Editor from '../Editor.svelte'
import type SimpleEditor from '../SimpleEditor.svelte'
import type { TestSteps } from './testSteps.svelte'

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
	on_behalf_of_email?: string
}

export type FlowInputEditorState = {
	selectedTab:
		| 'inputEditor'
		| 'history'
		| 'savedInputs'
		| 'json'
		| 'captures'
		| 'firstStepInputs'
		| undefined
	editPanelSize: number | undefined
	payloadData: Record<string, any> | undefined
}

export type CurrentEditor =
	| ((
			| {
					type: 'script'
					editor: Editor
					showDiffMode: () => void
					hideDiffMode: () => void
					diffMode: boolean
					lastDeployedCode: string | undefined
			  }
			| { type: 'iterator'; editor: SimpleEditor }
	  ) & {
			stepId: string
	  })
	| undefined

export type FlowEditorContext = {
	selectedId: Writable<string>
	currentEditor: Writable<CurrentEditor>
	moving: Writable<{ module: FlowModule; modules: FlowModule[] } | undefined>
	previewArgs: Writable<Record<string, any>>
	scriptEditorDrawer: Writable<ScriptEditorDrawer | undefined>
	history: History<OpenFlow>
	pathStore: Writable<string>
	flowStore: Writable<ExtendedOpenFlow>
	flowInputEditorState: Writable<FlowInputEditorState>
	flowStateStore: Writable<FlowState>
	testSteps: TestSteps
	saveDraft: () => void
	initialPathStore: Writable<string>
	fakeInitialPath: string
	flowInputsStore: Writable<FlowInput>
	customUi: FlowBuilderWhitelabelCustomUi
	insertButtonOpen: Writable<boolean>
	executionCount: Writable<number>
}
