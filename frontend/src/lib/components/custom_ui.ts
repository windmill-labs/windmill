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
		settings?: boolean
		draft?: boolean
	}
	settingsPanel?: boolean
	settingsTabs?: {
		schedule?: boolean
		sharedDiretory?: boolean
		earlyStop?: boolean
		earlyReturn?: boolean
		workerGroup?: boolean
		concurrency?: boolean
		debouncing?: boolean
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
	tagSelectPlaceholder?: string
	tagSelectNoLabel?: boolean
	tagLabel?: string
	aiAgent?: boolean
	aiSandbox?: boolean
	suggestIntegration?: boolean
	suggestScript?: boolean
	// Default timeout (in seconds) prefilled when enabling a custom step timeout.
	// Defaults to 300 (5 minutes) when unset.
	defaultTimeout?: number
}

export type DisplayResultUi = {
	disableAiFix?: boolean
	disableDownload?: boolean
}

export type PreviewPanelUi = {
	disableHistory?: boolean
	disableTracing?: boolean
	disableTriggerCaptures?: boolean
	disableTriggerButton?: boolean
	disableJsonView?: boolean
	displayResult?: DisplayResultUi
	disableVariablePicker?: boolean
	disableDownload?: boolean
	tagLabel?: string
	// Hide the args/SchemaForm pane entirely (use cases where the script
	// is known to take no inputs — e.g. the pipeline editor's per-script
	// preview). When set, the Test/Cancel button is rendered as a small
	// floating affordance at the top-left of the preview area instead of
	// inside the (now-absent) args column.
	hideArgs?: boolean
	// Render the LogPanel's logs/result as a left/right split instead of
	// the default top/bottom. Pairs naturally with `hideArgs` when the
	// preview area is the full bottom band.
	logsResultSideBySide?: boolean
	// Number of scripts that subscribe to assets this script writes (via
	// `// on s3://…` etc.). When > 0, the Test button is rendered as a
	// split: primary action runs just this step (cascade suppressed via
	// `_wmill_skip_asset_dispatch`), and a caret exposes "Test + trigger N
	// downstream" that lets the asset-dispatch hook fire downstream jobs.
	// Undefined or 0 → plain Test button (no cascade UI).
	downstreamSubscribers?: number
	// Pairs with `hideArgs`: still hide the full args column, but when the
	// script actually declares inputs render a compact SchemaForm between
	// the floating Test button and the logs/result panel (e.g. a
	// partitioned pipeline script that needs a `partition` arg to run).
	// No-op when the script has no input properties.
	argsAboveLogs?: boolean
	// On mount, query the most recent top-level completed job for this
	// script's path and load it into the preview pane so users immediately
	// see the last run's logs/result instead of an empty panel. Used by the
	// asset-graph details pane where selecting a script node should expose
	// "what happened the last time this ran". No-op when a test is already
	// in progress (we don't clobber a live run).
	loadLastRunOnMount?: boolean
	// Pipeline-only: when set, the Test split's caret popover gains a "Run
	// downstream up to…" entry that calls this, letting the user bound the
	// cascade from the script they're editing. Wired by the pipeline details
	// pane only when the open script is a valid bounded-run start.
	onBoundedRun?: () => void
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
	editorSettings?: boolean
	vimMode?: boolean
	relativeLineNumbers?: boolean
	aiGen?: boolean
	aiCompletion?: boolean
	library?: boolean
	useVsCode?: boolean
	diffMode?: boolean
	s3object?: boolean
	database?: boolean
	ducklake?: boolean
	dataTable?: boolean
	debug?: boolean
	history?: boolean
	saveToWorkspace?: boolean
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
	disableAiFilling?: boolean
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
		tagEdit?: boolean
	}
	settingsPanel?: SettingsPanelUi
	disableTooltips?: boolean
	editorBar?: EditorBarUi
	previewPanel?: PreviewPanelUi
	tagSelectPlaceholder?: string
	// Default timeout (in seconds) prefilled when enabling a custom script timeout.
	// Defaults to 300 (5 minutes) when unset.
	defaultTimeout?: number
}
