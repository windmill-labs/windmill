import type { SupportedLanguage } from '$lib/common'

export type FlowBuilderWhitelabelCustomUi = {
	topBar?: {
		path?: boolean
		export?: boolean
		history?: boolean
		aiBuilder?: boolean
		tutorials?: boolean
		diff?: boolean
		extraDeployOptions?: boolean
		editableSummary?: boolean
	}
	settingsPanel?: boolean
	settingsTabs?: {
		schedule?: boolean
		sharedDiretory?: boolean
		earlyStop?: boolean
		earlyReturn?: boolean
		workerGroup?: boolean
		concurrency?: boolean
		cache?: boolean
	}
	triggers?: boolean
	flowNode?: boolean
	hub?: boolean
	hubCode?: boolean
	graph?: { aiBuilder?: boolean; dataflow?: boolean }
	stepInputs?: { ai?: boolean }
	stepAdvancedSettings?: boolean
	languages?: (SupportedLanguage | 'docker' | 'bunnative')[]
	scriptFork?: boolean
	scriptEdit?: boolean
	tagEdit?: boolean
	editorBar?: EditorBarUi
	downloadLogs?: boolean
}

export type DisplayResultUi = {
	disableAiFix?: boolean
	disableDownload?: boolean
}

export type PreviewPanelUi = {
	disableHistory?: boolean
	disableTriggerCaptures?: boolean
	disableTriggerButton?: boolean
	displayResult?: DisplayResultUi
	disableVariablePicker?: boolean
	disableDownload?: boolean
}

export type EditorBarUi = {
	contextVar?: boolean
	variable?: boolean
	resource?: boolean
	reset?: boolean
	type?: boolean
	assistants?: boolean
	multiplayer?: boolean
	autoformatting?: boolean
	vimMode?: boolean
	aiGen?: boolean
	aiCompletion?: boolean
	library?: boolean
	useVsCode?: boolean
	diffMode?: boolean
	s3object?: boolean
}

export type EditableSchemaFormUi = {
	jsonOnly?: boolean
	disableVariablePicker?: boolean
}

export type SettingsPanelMetadataUi = {
	languages?: SupportedLanguage[]
	disableScriptKind?: boolean
	editableSchemaForm?: EditableSchemaFormUi
	disableMute?: boolean
}

export type SettingsPanelUi = {
	metadata?: SettingsPanelMetadataUi
	disableMetadata?: boolean
	disableRuntime?: boolean
	disableGeneratedUi?: boolean
	disableTriggers?: boolean
}

export type ScriptEditorWhitelabelCustomUi = {
	editorBar?: EditorBarUi
	previewPanel?: PreviewPanelUi
	disableTooltips?: boolean
}

export type ScriptBuilderWhitelabelCustomUi = {
	topBar?: {
		path?: boolean
		editablePath?: boolean
		settings?: boolean
		extraDeployOptions?: boolean
		editableSummary?: boolean
		diff?: boolean
	}
	settingsPanel?: SettingsPanelUi
	disableTooltips?: boolean

	editorBar?: EditorBarUi
	previewPanel?: PreviewPanelUi
}
