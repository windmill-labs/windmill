import { get } from 'svelte/store'
import { getLocalSetting, storeLocalSetting } from '$lib/utils'
import { userStore } from '$lib/stores'

// Keyed by user as well: two admins sharing a browser profile each keep their own choice.
const scope = (workspace: string) => `${get(userStore)?.email ?? ''}:${workspace}`
const hiddenKey = (workspace: string) => `git-sync-home-badge-hidden:${scope(workspace)}`

// One reactive copy for the page: the badge's Hide and the homepage menu's Show flip the same
// value, and a localStorage write alone re-renders nothing.
const hidden = $state<Record<string, boolean>>({})

export function isGitSyncHomeBadgeHidden(workspace: string): boolean {
	return hidden[scope(workspace)] ?? getLocalSetting(hiddenKey(workspace)) === 'true'
}

export function setGitSyncHomeBadgeHidden(workspace: string, value: boolean) {
	hidden[scope(workspace)] = value
	try {
		storeLocalSetting(hiddenKey(workspace), value ? 'true' : undefined)
	} catch {}
}
