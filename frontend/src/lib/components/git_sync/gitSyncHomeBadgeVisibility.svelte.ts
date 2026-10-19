import { getLocalSetting, storeLocalSetting } from '$lib/utils'

const hiddenKey = (workspace: string) => `git-sync-home-badge-hidden:${workspace}`

// One reactive copy for the page: the badge's Hide and the homepage menu's Show flip the same
// value, and a localStorage write alone re-renders nothing.
const hidden = $state<Record<string, boolean>>({})

export function isGitSyncHomeBadgeHidden(workspace: string): boolean {
	return hidden[workspace] ?? getLocalSetting(hiddenKey(workspace)) === 'true'
}

export function setGitSyncHomeBadgeHidden(workspace: string, value: boolean) {
	hidden[workspace] = value
	try {
		storeLocalSetting(hiddenKey(workspace), value ? 'true' : undefined)
	} catch {}
}
