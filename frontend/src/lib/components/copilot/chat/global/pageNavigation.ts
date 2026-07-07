import { buildFilterUrl } from '$lib/navigation'
import { buildRunsFilterSearchbarSchema } from '$lib/components/runs/runsFilter'
import { buildSchedulesFilterSchema } from '$lib/components/schedules/schedulesFilter'
import { TRIGGER_PAGES, type TriggerKind } from '$lib/components/sessions/previewRouter'

// In-app paths for the deep-linkable preview pages the AI chat can open.
export const RUNS_PATH = '/runs'
export const SCHEDULES_PATH = '/schedules'
export const VARIABLES_PATH = '/variables'
export const RESOURCES_PATH = '/resources'
export const ASSETS_PATH = '/assets'
export const AUDIT_LOGS_PATH = '/audit_logs'
export const WORKSPACE_SETTINGS_PATH = '/workspace_settings'
export const FOLDERS_PATH = '/folders'
export const GROUPS_PATH = '/groups'

// Selectable tabs on the Workspace settings page (the `?tab=` query param). Mirrors the
// union in routes/(root)/(logged)/workspace_settings/+page.svelte.
export const WORKSPACE_SETTINGS_TABS = [
	'users',
	'slack',
	'teams',
	'premium',
	'general',
	'webhook',
	'deploy_to',
	'dev_workspace',
	'error_handler',
	'success_handler',
	'critical_alerts',
	'ai',
	'windmill_data_tables',
	'windmill_lfs',
	'volume_storage',
	'ducklake',
	'git_sync',
	'default_app',
	'native_triggers',
	'encryption',
	'dependencies',
	'rulesets',
	'shared_ui'
] as const

// Valid query-param keys are derived from the real filter schemas (option arrays are
// irrelevant to the key set), so a renamed filter key propagates here for free.
const RUNS_FILTER_KEYS = Object.keys(
	buildRunsFilterSearchbarSchema({
		paths: [],
		usernames: [],
		folders: [],
		jobTriggerKinds: [],
		isSuperAdminOrDevops: false,
		isAdminsWorkspace: false
	})
)
const SCHEDULES_FILTER_KEYS = Object.keys(
	buildSchedulesFilterSchema({ paths: [], scriptPaths: [] })
)

/** Deep-link to the Runs page with the given filters (keys must match `runsFilter`). */
export function buildRunsUrl(filters: Record<string, unknown>): string {
	return buildFilterUrl(RUNS_PATH, filters, { validKeys: RUNS_FILTER_KEYS })
}

/**
 * Deep-link to the Schedules page. When `open` is set, the schedule at that exact path
 * is opened in the edit drawer via the `#<schedule_path>` hash the page already handles.
 */
export function buildSchedulesUrl({
	open,
	filters
}: {
	open?: string
	filters?: Record<string, unknown>
}): string {
	return buildFilterUrl(SCHEDULES_PATH, filters ?? {}, {
		validKeys: SCHEDULES_FILTER_KEYS,
		hash: open
	})
}

// The remaining pages expose a curated subset of each page's real query params (not the
// full filter schema), so the allow-list is the exact set of keys the builder emits —
// these names match the query params the pages read (variablesFilter/resourcesFilter/
// assetsFilter and audit_logs/+page.svelte).
export function buildVariablesUrl(filters: Record<string, unknown>): string {
	return buildFilterUrl(VARIABLES_PATH, filters, { validKeys: ['path', 'owner'] })
}

export function buildResourcesUrl(filters: Record<string, unknown>): string {
	return buildFilterUrl(RESOURCES_PATH, filters, {
		validKeys: ['path', 'resource_type', 'owner']
	})
}

export function buildAssetsUrl(filters: Record<string, unknown>): string {
	return buildFilterUrl(ASSETS_PATH, filters, { validKeys: ['path'] })
}

export function buildAuditLogsUrl(filters: Record<string, unknown>): string {
	return buildFilterUrl(AUDIT_LOGS_PATH, filters, {
		validKeys: ['username', 'operation', 'resource']
	})
}

/** Deep-link to the Workspace settings page, optionally on a specific `?tab=`. */
export function buildWorkspaceSettingsUrl({ tab }: { tab?: string }): string {
	return buildFilterUrl(WORKSPACE_SETTINGS_PATH, tab ? { tab } : {})
}

/** Folders and Groups list pages have no query filters — just open them. */
export function buildFoldersUrl(): string {
	return FOLDERS_PATH
}

export function buildGroupsUrl(): string {
	return GROUPS_PATH
}

/**
 * Deep-link to a trigger list page (by kind). When `open` is set, the trigger at that
 * exact path is opened in the edit drawer via the `#<path>` hash the page handles.
 */
export function buildTriggersUrl({
	trigger_kind,
	open
}: {
	trigger_kind: TriggerKind
	open?: string
}): string {
	return buildFilterUrl(TRIGGER_PAGES[trigger_kind].path, {}, { hash: open })
}
