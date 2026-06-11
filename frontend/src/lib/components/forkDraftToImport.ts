import { goto } from '$lib/navigation'
import { base } from '$app/paths'
import type { Flow, NewScript, UserDraftItemKind } from '$lib/gen'
import { importStore } from '$lib/components/apps/store'
import { importFlowStore } from '$lib/components/flows/flowStore.svelte'
import { importScriptStore } from '$lib/components/scripts/scriptStore.svelte'

/**
 * Open a fetched draft value as a brand-new item of `itemKind` — the
 * same one-shot import handoff the "Import from YAML/JSON" actions use:
 * stash the payload in the matching import store and route to the
 * kind's `/add` page, whose redirect mints a fresh
 * `u/{user}/draft_{uuid}` slot. The destination editor seeds from the
 * store, so the fork behaves exactly like a new item of one's own:
 * nothing is saved server-side until the first real edit, the path is
 * an owned auto-generated one (fully renamable), and none of the source
 * item's identity (path, draft_path, perms) rides along.
 *
 * Used by the "Fork" actions on other users' drafts
 * (OtherUsersDraftsModal, DraftBadge). Only the cross-user-visible
 * kinds can be forked — drawer kinds (resource/variable/triggers) never
 * surface other users' drafts in the first place.
 */
export function forkDraftToImport(itemKind: UserDraftItemKind, value: unknown): void {
	switch (itemKind) {
		case 'script':
			importScriptStore.set(value as NewScript)
			goto(`${base}/scripts/add`)
			return
		case 'flow':
			importFlowStore.set(value as Flow)
			goto(`${base}/flows/add`)
			return
		case 'app':
			// App drafts store the bare `App` value (no summary/policy
			// wrapper) — the /apps/edit import branch accepts both shapes.
			importStore.set(value as any)
			goto(`${base}/apps/add`)
			return
		case 'raw_app': {
			// Raw-app drafts bundle `{files, runnables, data, summary,
			// policy, ...}` flat; wrap so the /apps_raw/edit import branch
			// picks up summary and policy alongside the value.
			const v = value as any
			importStore.set({ summary: v?.summary ?? '', value: v, policy: v?.policy })
			goto(`${base}/apps_raw/add`)
			return
		}
		default:
			throw new Error(`Cannot fork drafts of kind ${itemKind}`)
	}
}
