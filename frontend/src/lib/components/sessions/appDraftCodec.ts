import type { RawAppData } from '$lib/components/raw_apps/dataTableRefUtils'

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
	// User-typed path while the app is parked at a `…/draft_<uuid>` storage key.
	// Round-trips through the draft as its own `path` so the home/review/Drafts
	// lists render the friendly name (they read `value->>'path'`) — and so editing
	// the path in the editor changes the persisted draft and triggers an autosave.
	// Present only on a rename; the storage key is the URL, not this field.
	path?: string
}

// The shape `runtime.rawApp.val` actually holds (see SessionRuntime in
// sessionRuntime.svelte.ts). `path` is the storage key (a runtime-only field,
// the session editor target); `typedPath` is the user-typed rename, set only
// when it differs from the deployed path and persisted as the draft's own `path`.
export type RuntimeRawApp = {
	summary: string
	path: string
	files: Record<string, string>
	runnables: Record<string, any>
	data: RawAppData
	policy: any
	custom_path?: string
	typedPath?: string
}

// Strip runtime-only metadata (the storage-key `path`) when persisting to
// UserDraft, and surface the user-typed rename under the draft's own `path` so
// the backend lists read it from `value->>'path'`. `custom_path` is a real draft
// field and must round-trip — else session sync erases a draft's custom URL.
export function runtimeRawAppToDraft(raw: RuntimeRawApp): RawAppDraft {
	return {
		summary: raw.summary,
		files: raw.files,
		runnables: raw.runnables,
		data: raw.data,
		policy: raw.policy,
		custom_path: raw.custom_path,
		...(raw.typedPath ? { path: raw.typedPath } : {})
	}
}

// Overlay a UserDraft-stored raw-app draft onto an existing runtime raw app,
// preserving the runtime-only storage-key `path` and mapping the draft's `path`
// (the typed rename) back onto `typedPath`.
export function applyDraftToRuntimeRawApp(raw: RuntimeRawApp, dv: RawAppDraft): RuntimeRawApp {
	return {
		...raw,
		summary: dv.summary,
		files: dv.files,
		runnables: dv.runnables,
		data: dv.data,
		policy: dv.policy ?? raw.policy,
		custom_path: dv.custom_path ?? raw.custom_path,
		typedPath: dv.path ?? raw.typedPath
	}
}
