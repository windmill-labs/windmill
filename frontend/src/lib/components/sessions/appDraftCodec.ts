import { type RawAppData, DEFAULT_DATA } from '$lib/components/raw_apps/dataTableRefUtils'

// The raw-app draft shape stored under `UserDraft<RawAppDraft>` — matches the
// regular `/apps_raw/edit` route's UserDraft handle exactly. The chat's
// `userDraftAdapter.saveGlobalAppDraft` writes through the same shape, so
// session previews and the chat round-trip identically.
export type RawAppDraft = {
	files: Record<string, string>
	runnables: Record<string, any>
	data: RawAppData
	summary: string
	policy?: any
	custom_path?: string
}

// The shape `runtime.rawApp.val` actually holds (see SessionRuntime in
// sessionRuntime.svelte.ts). Adds `path` (a key, not a draft field) and
// makes `policy` required for the editor's live binding.
export type RuntimeRawApp = {
	summary: string
	path: string
	files: Record<string, string>
	runnables: Record<string, any>
	data: RawAppData
	policy: any
	custom_path?: string
}

// Strip runtime-only metadata (just `path`, the storage key) when persisting
// to UserDraft. `custom_path` is a real draft field and must round-trip — else
// session sync erases a draft's custom URL.
export function runtimeRawAppToDraft(raw: RuntimeRawApp): RawAppDraft {
	return {
		summary: raw.summary,
		files: raw.files,
		runnables: raw.runnables,
		data: raw.data,
		policy: raw.policy,
		custom_path: raw.custom_path
	}
}

// Project a raw-app `value` payload (a deployed app's `value`, or a draft
// pocket) into the `RawAppDraft` shape UserDraft stores. Mirrors the data-table
// normalization loadRawApp applies on load: the stored `value` keeps tables
// under `data.creation` / `datatables`, which the runtime flattens into a
// `RawAppData`. Used by the session "Restore to deployed" action to reset the
// draft cell to the deployed value without re-fetching (which would race the
// draft delete).
export function rawAppValueToDraft(
	value: any,
	meta: { summary: string | undefined; policy?: any; custom_path?: string }
): RawAppDraft {
	let data: RawAppData = { ...DEFAULT_DATA }
	if (value?.data) {
		const d = value.data
		if (d.creation) {
			data = { tables: d.tables ?? [], datatable: d.creation.datatable, schema: d.creation.schema }
		} else {
			data = d
		}
	} else if (value?.datatables) {
		data = { ...DEFAULT_DATA, tables: value.datatables }
	}
	return {
		files: (value?.files ?? {}) as Record<string, string>,
		runnables: (value?.runnables ?? {}) as Record<string, any>,
		data,
		summary: meta.summary ?? '',
		policy: meta.policy,
		custom_path: meta.custom_path
	}
}

// Overlay a UserDraft-stored raw-app draft onto an existing runtime raw app,
// preserving the runtime-only `path` field.
export function applyDraftToRuntimeRawApp(raw: RuntimeRawApp, dv: RawAppDraft): RuntimeRawApp {
	return {
		...raw,
		summary: dv.summary,
		files: dv.files,
		runnables: dv.runnables,
		data: dv.data,
		policy: dv.policy ?? raw.policy,
		custom_path: dv.custom_path ?? raw.custom_path
	}
}
