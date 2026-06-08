import type { NewScript } from '$lib/gen'
import type { AssetWithAltAccessType } from './assets/lib'
import type { ScriptBuilderWhitelabelCustomUi } from './custom_ui'
import type { DiffDrawerI } from './diff_drawer'
import type { ScriptBuilderFunctionExports } from './scriptBuilder'
import type { ScheduleTrigger } from './triggers'
import type { NewScriptWithDraftAndDraftTriggers, Trigger } from './triggers/utils'
import type { WorkspaceItem } from './workspacePicker'

export interface ScriptBuilderProps {
	script: NewScript & {
		draft_triggers?: Trigger[]
		assets?: AssetWithAltAccessType[]
	}
	disableAi?: boolean
	fullyLoaded?: boolean
	initialPath?: string
	template?:
		| 'docker'
		| 'bunnative'
		| 'claudesandbox'
		| 'wac_python'
		| 'wac_typescript'
		| 'ci_test_bun'
		| 'ci_test_python'
		| 'script'
	initialArgs?: Record<string, any>
	lockedLanguage?: boolean
	showMeta?: boolean
	neverShowMeta?: boolean
	diffDrawer?: DiffDrawerI | undefined
	savedScript?: NewScriptWithDraftAndDraftTriggers | undefined
	searchParams?: URLSearchParams
	disableHistoryChange?: boolean
	customUi?: ScriptBuilderWhitelabelCustomUi
	savedPrimarySchedule?: ScheduleTrigger | undefined
	functionExports?: ((exports: ScriptBuilderFunctionExports) => void) | undefined
	children?: import('svelte').Snippet
	// Fires on every successful deploy. `stay` is true for "Deploy & Stay here"
	// and for lib scripts (the editor stays in place rather than navigating to
	// the deployed item) — consumers should skip post-deploy navigation when set.
	onDeploy?: (e: { path: string; hash: string; stay: boolean }) => void
	onDeployError?: (e: { path: string; error: any }) => void
	onSaveInitial?: (e: { path: string; hash: string }) => void
	onHistoryRestore?: () => void
	onSaveDraftOnlyAtNewPath?: (e: { path: string }) => void
	onSaveDraft?: (e: { path: string; savedAtNewPath: boolean; script: NewScript }) => void
	onSeeDetails?: (e: { path: string }) => void
	onSaveDraftError?: (e: { path: string; error: any }) => void
	onNavigate?: (item: WorkspaceItem) => void
	// Fired whenever a test run is started from the script editor, with the
	// preview job id. Used by whitelabel embedders to track test jobs.
	onTestJob?: (e: { jobId: string }) => void
	// Forwarded to the underlying ScriptEditor. When true, the right-hand
	// test/run pane opens collapsed. Used by the session preview.
	initialTestPanelCollapsed?: boolean
	// Treat the path as already chosen (seeds the path "dirty" flag) so the
	// summary→path auto-slug for new scripts (initialPath == '') doesn't
	// overwrite it. Used by the session preview, which opens AI-created scripts
	// as new but with a path the AI already assigned.
	initialPathChosen?: boolean
}
