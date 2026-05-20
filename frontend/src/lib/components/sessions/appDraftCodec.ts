import type { RawAppData } from '$lib/components/raw_apps/dataTableRefUtils'
import type { AppDraftValue } from '$lib/components/copilot/chat/global/draftStore.svelte'

// The shape `runtime.rawApp.val` actually holds (see SessionRuntime in
// sessionRuntime.svelte.ts lines 74-84). Slightly flatter than the AI's
// `AppDraftValue`: `path` is metadata not present on the AI side, and
// `summary` is required here.
export type RuntimeRawApp = {
	summary: string
	path: string
	files: Record<string, string>
	runnables: Record<string, any>
	data: RawAppData
	policy: any
}

// Strip runtime metadata (just `path` for raw apps) and project into the
// AI-facing `AppDraftValue` envelope.
export function rawAppToDraftValue(raw: RuntimeRawApp): AppDraftValue {
	return {
		summary: raw.summary,
		files: raw.files,
		runnables: raw.runnables,
		data: raw.data,
		policy: raw.policy
		// custom_path is read-only and not held on the runtime.
	}
}

// Overlay an AI-produced draft onto an existing runtime raw app,
// preserving metadata fields (path) that don't live in `AppDraftValue`.
export function applyDraftValueToRawApp(raw: RuntimeRawApp, dv: AppDraftValue): RuntimeRawApp {
	return {
		...raw,
		summary: dv.summary ?? raw.summary,
		files: dv.files,
		runnables: dv.runnables,
		data: (dv.data as RawAppData | undefined) ?? raw.data,
		policy: dv.policy ?? raw.policy
	}
}
