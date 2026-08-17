import { ScriptService } from '$lib/gen'
import type { ScriptAdvancedSettingsFields } from '$lib/components/scriptSettings'

// Loads the advanced runtime settings (concurrency, cache, timeout, ...) of the
// workspace script referenced by a flow step, so the flow editor can surface the
// current values instead of only a "set it on the script" warning. Reactive to
// the path/hash/workspace getters; call reload() after saving new settings.
export function useWorkspaceScriptSettings(
	pathGetter: () => string | undefined,
	hashGetter: () => string | undefined,
	workspaceGetter: () => string | undefined
) {
	let settings = $state<ScriptAdvancedSettingsFields | undefined>(undefined)
	let loading = $state(false)
	let error = $state<string | undefined>(undefined)
	// Guards against an older in-flight load resolving after a newer one and
	// clobbering the displayed settings when path/hash change quickly.
	let loadSeq = 0

	async function load(
		path: string | undefined,
		hash: string | undefined,
		workspace: string | undefined
	) {
		const seq = ++loadSeq
		if (!path || !workspace || path.startsWith('hub/')) {
			settings = undefined
			error = undefined
			// Clear here too: this supersedes any in-flight load, whose guarded
			// finally can no longer reset loading, else the card spins forever.
			loading = false
			return
		}
		loading = true
		error = undefined
		try {
			const script = hash
				? await ScriptService.getScriptByHash({ workspace, hash })
				: await ScriptService.getScriptByPath({ workspace, path })
			if (seq !== loadSeq) return
			settings = script as ScriptAdvancedSettingsFields
		} catch (e) {
			console.error('Could not load referenced script settings', e)
			if (seq === loadSeq) {
				settings = undefined
				// Surface failure so cards distinguish "load failed" from "not set".
				error = `${(e as { body?: string })?.body ?? e}`
			}
		} finally {
			if (seq === loadSeq) loading = false
		}
	}

	$effect(() => {
		load(pathGetter(), hashGetter(), workspaceGetter())
	})

	return {
		get settings() {
			return settings
		},
		get loading() {
			return loading
		},
		get error() {
			return error
		},
		reload() {
			return load(pathGetter(), hashGetter(), workspaceGetter())
		}
	}
}
