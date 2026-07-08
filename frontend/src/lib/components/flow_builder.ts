import type { Flow, OpenFlow } from '$lib/gen'
import type { StateStore } from '$lib/utils'
import type { FlowState } from './flows/flowState'
import type { Trigger } from './triggers/utils'
import type { DiffDrawerI } from './diff_drawer'
import type { FlowBuilderWhitelabelCustomUi } from './custom_ui'
import type { ScheduleTrigger } from './triggers'
import type { stepState } from './stepHistoryLoader.svelte'
import type { WorkspaceItem } from './workspacePicker'

export type FlowBuilderProps = {
	initialPath?: string
	pathStoreInit?: string | undefined
	newFlow: boolean
	selectedId: string | undefined
	initialArgs?: Record<string, any>
	loading?: boolean
	flowStore: StateStore<OpenFlow>
	flowStateStore: StateStore<FlowState>
	savedFlow?: Flow & { no_deployed?: boolean }
	diffDrawer?: DiffDrawerI | undefined
	customUi?: FlowBuilderWhitelabelCustomUi
	disableAi?: boolean
	disabledFlowInputs?: boolean
	/** Opt into the responsive modal panel: when the flow editor mounts narrower
	 *  than the breakpoint, the right (step-details) pane starts as a modal opened
	 *  by double-clicking a graph node. Only the sessions embed sets this; the
	 *  full-page editor leaves it false and always shows the split pane. */
	allowModalPanel?: boolean
	savedPrimarySchedule?: ScheduleTrigger | undefined // used to set the primary schedule in the legacy primaryScheduleStore
	version?: number | undefined
	/** flow_version the draft was forked from; when set, the deploy-time staleness
	 *  check compares it (not the load-time head `version`) against the latest. */
	draftBaseVersion?: number | undefined
	draftTriggersFromUrl?: Trigger[] | undefined
	selectedTriggerIndexFromUrl?: number | undefined
	children?: import('svelte').Snippet
	loadedFromHistoryFromUrl?: {
		flowJobInitial: boolean | undefined
		stepsState: Record<string, stepState>
	}
	noInitial?: boolean
	liveEditorDraftStoragePath?: string
	// Indicator-only draft key overrides. When the flow editor is embedded
	// (e.g. the sessions preview) its autosave runs under a different
	// (workspace, path) than `$workspaceStore`/`liveEditorDraftStoragePath`
	// (a forked workspace, and a path this component doesn't own). These let
	// the host point the `AutosaveIndicator` at the key its own autosave uses,
	// WITHOUT repurposing `liveEditorDraftStoragePath` (which still drives this
	// component's setLiveEditorDraft/flush). Undefined → fall back, so the
	// full-page editor is unaffected.
	autosaveWorkspace?: string
	autosavePath?: string
	onDeploy?: ({ path }: { path: string }) => void
	onDeployError?: ({ error }: { error: any }) => void
	onDetails?: ({ path }: { path: string }) => void
	onHistoryRestore?: () => void
	onNavigate?: (item: WorkspaceItem) => void
	// Threaded to the `AutosaveIndicator` popover so its "Reset to
	// deployed" button can do the same thing the load-time toast offers.
	onResetToDeployed?: () => void | Promise<void>
	// See ScriptBuilderProps — same semantics for the flow editor's
	// indicator.
	loadedFromDraft?: boolean
	othersDraftsCount?: number
	onOpenOthersDrafts?: () => void
	// Fired whenever a test run is started from the flow editor, with the
	// preview job id. Used by whitelabel embedders to track test jobs.
	onTestJob?: (e: { jobId: string }) => void
}
