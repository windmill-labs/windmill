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
	graph?: { aiBuilder?: boolean; dataflow?: boolean }
	stepInputs?: { ai?: boolean }
	stepAdvancedSettings?: boolean
	languages?: (SupportedLanguage | 'docker' | 'bunnative')[]
	scriptFork?: boolean
	editorBar?: EditorBarUi
	downloadLogs?: boolean
}

export type EditorBarUi = {
	contextVar?: boolean
	variable?: boolean
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

export type ScriptEditorWhitelabelCustomUi = {
	editorBar?: EditorBarUi
}

export type ScriptBuilderWhitelabelCustomUi = {
	topBar?: {
		path?: boolean
		settings?: boolean
		extraDeployOptions?: boolean
		editableSummary?: boolean
		diff?: boolean
	}
	editorBar?: EditorBarUi
}
