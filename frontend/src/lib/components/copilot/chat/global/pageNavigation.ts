import { buildFilterUrl } from '$lib/navigation'
import {
	COMPARE_PAGE,
	TRIGGER_PAGES,
	pageRequestParams,
	type TriggerKind
} from '$lib/components/sessions/previewRouter'
import {
	COMPARE_ITEMS_PARAM,
	serializeItemsMaskParam
} from '$lib/components/sessions/modifiedItemsMask'

import {
	RUNS_PATH,
	SCHEDULES_PATH,
	VARIABLES_PATH,
	RESOURCES_PATH,
	ASSETS_PATH,
	AUDIT_LOGS_PATH,
	WORKSPACE_SETTINGS_PATH,
	FOLDERS_PATH,
	GROUPS_PATH
} from '$lib/components/sessions/previewPaths'

// Selectable tabs on the Workspace settings page (the `?tab=` query param). Mirrors the
// union in routes/(root)/(logged)/workspace_settings/+page.svelte.
export const WORKSPACE_SETTINGS_TABS = [
	'users',
	'slack',
	'teams',
	'premium',
	'general',
	'webhook',
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

// Every builder below allows exactly the params `previewRouter` records as
// request-settable for that page, so the URLs this emits and the preview's reading of
// them stay one set. Wherever the page declares a filter schema that set is its full
// key list, so a renamed or added filter propagates here for free — including the keys
// only some viewers see: gating `all_workspaces` is the caller's job, and the Runs page
// ignores it for anyone whose own schema lacks it. What the chat may actually pass is
// narrower and lives in the open_page tool schema, not here.

/** Deep-link to the Runs page with the given filters (keys must match `runsFilter`). */
export function buildRunsUrl(filters: Record<string, unknown>): string {
	return buildFilterUrl(RUNS_PATH, filters, { validKeys: pageRequestParams(RUNS_PATH) })
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
		validKeys: pageRequestParams(SCHEDULES_PATH),
		hash: open
	})
}

/** When `open` is set, the variable at that exact path is opened in the edit
 * drawer via the `#<path>` hash the page already handles. */
export function buildVariablesUrl({
	open,
	filters
}: {
	open?: string
	filters?: Record<string, unknown>
}): string {
	return buildFilterUrl(VARIABLES_PATH, filters ?? {}, {
		validKeys: pageRequestParams(VARIABLES_PATH),
		hash: open
	})
}

/** When `open` is set, the resource at that exact path is opened in the edit
 * drawer via the `#/resource/<path>` hash the page already handles. */
export function buildResourcesUrl({
	open,
	filters
}: {
	open?: string
	filters?: Record<string, unknown>
}): string {
	return buildFilterUrl(RESOURCES_PATH, filters ?? {}, {
		validKeys: pageRequestParams(RESOURCES_PATH),
		hash: open ? `/resource/${open}` : undefined
	})
}

export function buildAssetsUrl(filters: Record<string, unknown>): string {
	return buildFilterUrl(ASSETS_PATH, filters, { validKeys: pageRequestParams(ASSETS_PATH) })
}

export function buildAuditLogsUrl(filters: Record<string, unknown>): string {
	return buildFilterUrl(AUDIT_LOGS_PATH, filters, {
		validKeys: pageRequestParams(AUDIT_LOGS_PATH)
	})
}

/** Deep-link to the Workspace settings page, optionally on a specific `?tab=`. */
export function buildWorkspaceSettingsUrl({ tab }: { tab?: string }): string {
	return buildFilterUrl(WORKSPACE_SETTINGS_PATH, tab ? { tab } : {}, {
		validKeys: pageRequestParams(WORKSPACE_SETTINGS_PATH)
	})
}

/** Folders and Groups list pages have no query filters — just open them. */
export function buildFoldersUrl(): string {
	return FOLDERS_PATH
}

export function buildGroupsUrl(): string {
	return GROUPS_PATH
}

/**
 * Deep-link to the Compare & Deploy page (`/forks/compare`). `workspace_id` is required:
 * inside a session preview the page loads with the *navigation* workspace as its store
 * default, which is not necessarily the session's (possibly forked) workspace. `items`
 * preselects exactly those `kind:path` entries (see modifiedItemsMask.ts); omitted, the
 * page falls back to its select-all default. `mode` forces the draft or fork comparison;
 * omitted, the page auto-picks: on a fork it lands on the view containing the masked
 * items (draft when any of them is a pending draft, else the fork comparison); a
 * non-fork always gets the draft view.
 */
export function buildCompareUrl({
	workspace_id,
	mode,
	items
}: {
	workspace_id: string
	mode?: 'draft' | 'fork'
	items?: readonly string[]
}): string {
	return buildFilterUrl(
		COMPARE_PAGE.path,
		{
			workspace_id,
			mode,
			[COMPARE_ITEMS_PARAM]: items ? serializeItemsMaskParam(items) : undefined
		},
		{ validKeys: pageRequestParams(COMPARE_PAGE.path) }
	)
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
