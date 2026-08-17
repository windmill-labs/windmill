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
	// User-typed path while the app is parked at a `…/draft_<uuid>` storage path.
	// Must round-trip through the draft so the home/review/Drafts lists render the
	// friendly name (they read `value->>'draft_path'`) — and so editing the path
	// in the editor changes the persisted draft and triggers an autosave.
	draft_path?: string
}

// The shape a raw-app cell's store (`RawAppRuntimeValue` in
// sessionRuntime.svelte.ts) actually holds. Adds `path` (a key, not a draft
// field) and makes `policy` required for the editor's live binding.
export type RuntimeRawApp = {
	summary: string
	path: string
	files: Record<string, string>
	runnables: Record<string, any>
	data: RawAppData
	policy: any
	custom_path?: string
	draft_path?: string
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
		custom_path: raw.custom_path,
		draft_path: raw.draft_path
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
		custom_path: dv.custom_path ?? raw.custom_path,
		draft_path: dv.draft_path ?? raw.draft_path
	}
}
