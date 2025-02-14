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
	aiFix?: { disabled?: boolean }
}

export type PreviewPanelUi = {
	disabled?: boolean
	history?: {
		disabled?: boolean
	}
	triggerCaptures?: {
		disabled?: boolean
	}
	triggerButton?: {
		disabled?: boolean
	}
	displayResult?: DisplayResultUi
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
}

export type GlobalUiConfig = {
	tooltips?: { disabled?: boolean }
}

export type EditableSchemaFormUi = {
	jsonOnly?: boolean
}

export type SettingsPanelMetadataUi = {
	disabled?: boolean
	languages?: SupportedLanguage[]
	scriptKind?: {
		disabled?: boolean
	}
	editableSchemaForm?: EditableSchemaFormUi
	mute?: { disabled?: boolean }
}

export type SettingsPanelUi = {
	disabled?: boolean
	metadata?: SettingsPanelMetadataUi
	runtime?: {
		disabled?: boolean
	}
	generatedUi?: {
		disabled?: boolean
		fields?: 'file'[]
	}
	triggers?: {
		disabled?: boolean
	}
}

export type ScriptEditorWhitelabelCustomUi = {
	editorBar?: EditorBarUi
	previewPanel?: PreviewPanelUi
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
	tooltips?: { disabled?: boolean }

	editorBar?: EditorBarUi
	previewPanel?: PreviewPanelUi
}
