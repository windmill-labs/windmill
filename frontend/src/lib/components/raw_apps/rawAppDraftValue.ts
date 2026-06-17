/**
 * Shared projection of a raw-app *source* into the flat `AppDraftValue` the
 * deploy paths consume. Used by BOTH the global AI chat
 * (`copilot/chat/global/core.ts`) and the Review & Deploy page
 * (`rawAppDeploy.ts`) so the two can't drift — a past divergence here silently
 * dropped a draft's files and broke deploys with "Raw app bundle requires
 * /index.ts".
 */
import type { AppDraftValue } from '$lib/components/copilot/chat/global/workspaceItems'
import { DEFAULT_DATA as DEFAULT_RAW_APP_DATA } from '$lib/components/raw_apps/dataTableRefUtils'

/** Collapse the historical `data` shapes a raw-app source might carry into the
 * canonical `AppDraftValue['data']`. */
export function normalizeRawAppData(value: Record<string, any>): AppDraftValue['data'] {
	if (value.data?.creation) {
		return {
			tables: value.data.tables ?? [],
			datatable: value.data.creation.datatable,
			schema: value.data.creation.schema
		}
	}
	if (value.data) {
		return value.data
	}
	if (value.datatables) {
		return { ...DEFAULT_RAW_APP_DATA, tables: value.datatables }
	}
	if (value.dataTableRefs) {
		return { ...DEFAULT_RAW_APP_DATA, tables: value.dataTableRefs }
	}
	return { ...DEFAULT_RAW_APP_DATA }
}

/**
 * Project a raw-app source into a flat `AppDraftValue`. Handles both shapes:
 *  - a deployed app nests its source under `value` (`app.value.files`);
 *  - a `RawAppDraft` already carries `files`/`runnables`/`data` at the top level.
 * Falling back to the object itself when there's no nested `value` keeps a
 * draft's bundle from being dropped.
 */
export function appSourceToDraftValue(app: any, fallback?: any): AppDraftValue {
	const value = (app.value ?? app) as Record<string, any>
	return {
		summary: app.summary ?? '',
		files: { ...(value.files ?? {}) },
		runnables: { ...(value.runnables ?? {}) },
		data: normalizeRawAppData(value),
		policy: app.policy ?? fallback?.policy,
		custom_path: app.custom_path ?? fallback?.custom_path
	}
}
