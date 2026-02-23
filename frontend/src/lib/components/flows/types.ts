import type { Job, OpenFlow } from '$lib/gen'
import type { History } from '$lib/history.svelte'
import type { Writable } from 'svelte/store'
import type ScriptEditorDrawer from './content/ScriptEditorDrawer.svelte'
import type FlowEditorDrawer from './content/FlowEditorDrawer.svelte'
import type { FlowState } from './flowState'
import type { FlowBuilderWhitelabelCustomUi } from '../custom_ui'
import type Editor from '../Editor.svelte'
import type SimpleEditor from '../SimpleEditor.svelte'
import type { StateStore } from '$lib/utils'
import type { StepsInputArgs } from './stepsInputArgs.svelte'
import type { Asset, AssetWithAccessType } from '../assets/lib'
import type S3FilePicker from '../S3FilePicker.svelte'
import type ResourceEditorDrawer from '../ResourceEditorDrawer.svelte'
import type { ModulesTestStates } from '../modulesTest.svelte'
import type { ButtonProp } from '$lib/components/diffEditorTypes'

import type { SelectionManager } from '../graph/selectionUtils.svelte'
import type { InferAssetsSqlQueryDetails } from '$lib/infer'

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

// Extended OpenFlow with additional properties not in the core spec
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
					setDiffOriginal?: (code: string) => void
					setDiffButtons?: (buttons: ButtonProp[]) => void
			  }
			| { type: 'iterator'; editor: SimpleEditor }
	  ) & {
			stepId: string
	  })
	| undefined

export type FlowEditorContext = {
	selectionManager: SelectionManager
	currentEditor: Writable<CurrentEditor>
	moving: Writable<{ id: string } | undefined>
	previewArgs: StateStore<Record<string, any>>
	scriptEditorDrawer: Writable<ScriptEditorDrawer | undefined>
	flowEditorDrawer: Writable<FlowEditorDrawer | undefined>
	history: History<OpenFlow>
	pathStore: Writable<string>
	flowStore: StateStore<ExtendedOpenFlow>
	flowInputEditorState: Writable<FlowInputEditorState>
	flowStateStore: StateStore<FlowState>
	stepsInputArgs: StepsInputArgs
	saveDraft: () => void
	initialPathStore: Writable<string>
	fakeInitialPath: string
	flowInputsStore: Writable<FlowInput>
	customUi: FlowBuilderWhitelabelCustomUi
	insertButtonOpen: Writable<boolean>
	executionCount: Writable<number>
	modulesTestStates: ModulesTestStates
	outputPickerOpenFns: Record<string, () => void>
}

export type FlowGraphAssetContext = StateStore<{
	selectedAsset: Asset | undefined
	s3FilePicker: S3FilePicker | undefined
	resourceEditorDrawer: ResourceEditorDrawer | undefined
	// Maps resource paths to their metadata. undefined is for error
	resourceMetadataCache: Record<string, { resource_type?: string } | undefined>
	additionalAssetsMap: Record<string, AssetWithAccessType[]>
	computeAssetsCount: (asset: Asset) => number
	sqlQueries: Record<string, InferAssetsSqlQueryDetails[] | undefined>
}>

export type OutputViewerJob =
	| ((
			| Job
			| {
					id: string
					result: unknown
					type: 'CompletedJob'
					workspace_id: string
					success: boolean
			  }
	  ) & { result_stream?: string; result?: unknown })
	| undefined
