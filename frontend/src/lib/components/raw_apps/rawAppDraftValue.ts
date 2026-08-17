/**
 * Shared projection of a raw-app *source* into the flat `AppDraftValue` the
 * deploy paths consume. The single source for BOTH the global AI chat
 * (`copilot/chat/global/core.ts`) and the Review & Deploy page
 * (`rawAppDeploy.ts`), so both bundle the identical shape. It must read a
 * draft's top-level `files` (a `RawAppDraft` carries them there, not under
 * `value.files`) — otherwise the bundle is empty and deploy fails with "Raw app
 * bundle requires /index.ts".
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
		custom_path: app.custom_path ?? fallback?.custom_path,
		// Pin the fork base: a deployed app exposes `versions` (head = last); an
		// existing draft already carries `parent_version` — preserve it.
		parent_version:
			app.parent_version ??
			(Array.isArray(app.versions) ? app.versions[app.versions.length - 1] : undefined) ??
			fallback?.parent_version
	}
}
