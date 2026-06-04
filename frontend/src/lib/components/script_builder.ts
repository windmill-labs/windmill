import type { NewScript, Script } from '$lib/gen'
import type { AssetWithAltAccessType } from './assets/lib'
import type { ScriptBuilderWhitelabelCustomUi } from './custom_ui'
import type { DiffDrawerI } from './diff_drawer'
import type { ScriptBuilderFunctionExports } from './scriptBuilder'
import type { ScheduleTrigger } from './triggers'
import type { Trigger } from './triggers/utils'
import type { WorkspaceItem } from './workspacePicker'

export interface ScriptBuilderProps {
	script: NewScript & {
		draft_triggers?: Trigger[]
		assets?: AssetWithAltAccessType[]
	}
	disableAi?: boolean
	fullyLoaded?: boolean
	initialPath?: string
	/**
	 * Path the route's `UserDraft.use<EditableScript>('script', ...)`
	 * handle is keyed by. Distinct from `initialPath` for new drafts —
	 * `initialPath` is the displayed/editor path (empty for new), while
	 * this is the URL path the draft is persisted under (`u/{user}/
	 * draft_{uuid}`). Used to bracket the bootstrap `initContent` write
	 * with `UserDraft.stopSync` / `restartSync` so the template seed
	 * doesn't POST before the user's first real edit. Default to `''`
	 * for backwards compat with callers that don't manage drafts; the
	 * stop/restart pair is a no-op on a non-live entry.
	 */
	userDraftPath?: string
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
	savedScript?: Script | NewScript | undefined
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
	onHistoryRestore?: () => void
	onSeeDetails?: (e: { path: string }) => void
	onNavigate?: (item: WorkspaceItem) => void
	// Forwarded to the underlying ScriptEditor. When true, the right-hand
	// test/run pane opens collapsed. Used by the session preview.
	initialTestPanelCollapsed?: boolean
	// Treat the path as already chosen (seeds the path "dirty" flag) so the
	// summary→path auto-slug for new scripts (initialPath == '') doesn't
	// overwrite it. Used by the session preview, which opens AI-created scripts
	// as new but with a path the AI already assigned.
	initialPathChosen?: boolean
}
