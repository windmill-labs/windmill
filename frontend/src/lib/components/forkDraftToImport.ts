import { goto } from '$lib/navigation'
import { base } from '$app/paths'
import type { Flow, NewScript, UserDraftItemKind } from '$lib/gen'
import { importStore } from '$lib/components/apps/store'
import { importFlowStore } from '$lib/components/flows/flowStore.svelte'
import { importScriptStore } from '$lib/components/scripts/scriptStore.svelte'
import { getUsernameForNamespace } from '$lib/userNamespace'

/**
 * Re-home the source path into the forker's namespace: drop the first two
 * segments, prefix `u/{me}`. `u/admin/myflow` → `u/me/myflow`.
 */
function forkSeedPath(sourcePath: string): string {
	const rest = sourcePath.split('/').slice(2).join('/')
	return `u/${getUsernameForNamespace()}/${rest}`
}

/**
 * Open a fetched draft value as a brand-new item of `itemKind`, via the same
 * one-shot import handoff as "Import from YAML/JSON": stash the payload in the
 * import store and route to the kind's `/add` page. The fork behaves like a new
 * item of one's own — nothing saved until the first edit, no source identity
 * carried over. The re-homed source path travels as `?seed_path=` (not `?path=`,
 * which ScriptBuilder strips in transit) so the Path widget starts recognizable.
 * Only the cross-user-visible kinds can be forked.
 */
export function forkDraftToImport(
	itemKind: UserDraftItemKind,
	value: unknown,
	sourcePath: string
): void {
	const seed = `?seed_path=${encodeURIComponent(forkSeedPath(sourcePath))}`
	switch (itemKind) {
		case 'script':
			importScriptStore.set(value as NewScript)
			goto(`${base}/scripts/add${seed}`)
			return
		case 'flow':
			importFlowStore.set(value as Flow)
			goto(`${base}/flows/add${seed}`)
			return
		case 'app':
			// App drafts store the bare `App` value (no summary/policy
			// wrapper) — the /apps/edit import branch accepts both shapes.
			importStore.set(value as any)
			goto(`${base}/apps/add${seed}`)
			return
		case 'raw_app': {
			// Raw-app drafts bundle `{files, runnables, data, summary,
			// policy, ...}` flat; wrap so the /apps_raw/edit import branch
			// picks up summary and policy alongside the value.
			const v = value as any
			importStore.set({ summary: v?.summary ?? '', value: v, policy: v?.policy })
			goto(`${base}/apps_raw/add${seed}`)
			return
		}
		default:
			throw new Error(`Cannot fork drafts of kind ${itemKind}`)
	}
}
