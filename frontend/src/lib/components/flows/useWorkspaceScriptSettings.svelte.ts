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

	async function load(
		path: string | undefined,
		hash: string | undefined,
		workspace: string | undefined
	) {
		if (!path || !workspace || path.startsWith('hub/')) {
			settings = undefined
			return
		}
		loading = true
		try {
			const script = hash
				? await ScriptService.getScriptByHash({ workspace, hash })
				: await ScriptService.getScriptByPath({ workspace, path })
			settings = script as ScriptAdvancedSettingsFields
		} catch (e) {
			console.error('Could not load referenced script settings', e)
			settings = undefined
		} finally {
			loading = false
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
		reload() {
			return load(pathGetter(), hashGetter(), workspaceGetter())
		}
	}
}
