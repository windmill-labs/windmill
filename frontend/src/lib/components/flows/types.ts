import type { OpenFlow } from '$lib/gen'
import type { History } from '$lib/history'
import type { Writable } from 'svelte/store'
import type ScriptEditorDrawer from './content/ScriptEditorDrawer.svelte'
import type { FlowState } from './flowState'
import type { FlowBuilderWhitelabelCustomUi } from '../custom_ui'
import type Editor from '../Editor.svelte'
import type SimpleEditor from '../SimpleEditor.svelte'
import type { StateStore } from '$lib/utils'
import type { TestSteps } from './testSteps.svelte'
import type { Asset } from '../assets/lib'
import type S3FilePicker from '../S3FilePicker.svelte'
import type DbManagerDrawer from '../DBManagerDrawer.svelte'
import type ResourceEditorDrawer from '../ResourceEditorDrawer.svelte'

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
	moving: Writable<{ id: string } | undefined>
	previewArgs: StateStore<Record<string, any>>
	scriptEditorDrawer: Writable<ScriptEditorDrawer | undefined>
	history: History<OpenFlow>
	pathStore: Writable<string>
	flowStore: StateStore<ExtendedOpenFlow>
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

export type FlowGraphAssetContext = StateStore<{
	selectedAsset: Asset | undefined
	assetsMap?: Record<string, Asset[]> // Maps module ids to their assets
	s3FilePicker: S3FilePicker | undefined
	dbManagerDrawer: DbManagerDrawer | undefined
	resourceEditorDrawer: ResourceEditorDrawer | undefined
	// Maps resource paths to their metadata. undefined is for error
	resourceMetadataCache: Record<string, { resourceType?: string } | undefined>
}>
