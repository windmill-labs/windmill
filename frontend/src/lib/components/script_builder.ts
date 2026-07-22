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
	 * Wrapper-only signal (consumed by `ScriptWrapper`, not `ScriptBuilder`):
	 * this editor is mounting a brand-new script. When set and no caller path
	 * is provided, the wrapper mints a `u/<user>/draft_<uuid>` storage path so
	 * autosave attaches — mirrors what the `/scripts/add` route does before the
	 * full-page editor mounts. Left unset for read-only / pathless views so
	 * autosave stays intentionally detached.
	 */
	newScript?: boolean
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
	/**
	 * Workspace + path the AutosaveIndicator watches for sync state. Default
	 * (undefined) falls back to `$workspaceStore` / `userDraftPath` — the
	 * full-page editor. The sessions preview sets these to the session's
	 * (possibly forked) workspace and target path, where autosave is owned by
	 * `SessionEditorTarget`/`useUserDraftSync`, so the indicator must watch that
	 * key rather than the global store + the unset `userDraftPath`.
	 */
	autosaveWorkspace?: string
	autosavePath?: string
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
	savedScript?: (Script | NewScript) & { no_deployed?: boolean }
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
	// Fired whenever a test run is started from the script editor, with the
	// preview job id. Used by whitelabel embedders to track test jobs.
	onTestJob?: (e: { jobId: string }) => void
	// Forwarded to the underlying ScriptEditor. Seeds the right-hand test/run
	// pane collapsed, and edge-triggers a collapse/expand on later changes. The
	// session preview drives it from full-screen state.
	testPanelCollapsed?: boolean
	// Treat the path as already chosen (seeds the path "dirty" flag) so the
	// summary→path auto-slug for new scripts (initialPath == '') doesn't
	// overwrite it. Used by the session preview, which opens AI-created scripts
	// as new but with a path the AI already assigned.
	initialPathChosen?: boolean
	// Threaded to the `AutosaveIndicator` popover so its "Reset to
	// deployed" button can do the same thing the load-time toast offers.
	// Routes pass their own re-load-without-draft callback here; omit on
	// callers (session preview, embedded SDK) that shouldn't surface the
	// action at all.
	onResetToDeployed?: () => void | Promise<void>
	// Triggers the AutosaveIndicator's on-mount "Loaded from draft" hint
	// (with a one-shot green flash) the first time it flips to true.
	loadedFromDraft?: boolean
	// Non-zero when other workspace users have a draft at this path.
	// Drives both the indicator's hint label ("Others are working on
	// this script") and the popover's "See others' drafts" button.
	othersDraftsCount?: number
	// Wired by the route to flip the OtherUsersDraftsModal open.
	onOpenOthersDrafts?: () => void
	// Condensed top bar: smaller (sm) buttons, a shorter bar, and the
	// EditorHeader's path/breadcrumb row dropped (summary only). Used by the
	// session preview to save vertical room.
	condensedHeader?: boolean
}
