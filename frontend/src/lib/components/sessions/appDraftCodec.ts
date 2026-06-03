import type { RawAppData } from '$lib/components/raw_apps/dataTableRefUtils'
import type { RawAppDraft } from '$lib/components/raw_apps/rawAppDraftCodec'

export type { RawAppDraft } from '$lib/components/raw_apps/rawAppDraftCodec'

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
