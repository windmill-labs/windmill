export const USER_SETTINGS_HASH = '#user-settings' as const
export const SUPERADMIN_SETTINGS_HASH = '#superadmin-settings' as const

export type AccountSettingsPanel = 'profile' | 'tokens' | 'ai' | 'mcp'

const ACCOUNT_SETTINGS_PANELS: AccountSettingsPanel[] = ['tokens', 'ai', 'mcp']

/** `#user-settings` opens the profile, `#user-settings-<panel>` one of the other drawers. */
export function accountSettingsHref(panel: AccountSettingsPanel): string {
	return panel === 'profile' ? USER_SETTINGS_HASH : `${USER_SETTINGS_HASH}-${panel}`
}

/**
 * The account settings drawer a hash asks for. `#user-settings` is linked from the docs and
 * from older instances, so it and any unknown `#user-settings-*` suffix open the profile.
 */
export function parseAccountSettingsHash(hash: string): AccountSettingsPanel | undefined {
	if (!hash.startsWith(USER_SETTINGS_HASH)) return undefined
	const suffix = hash.slice(USER_SETTINGS_HASH.length)
	if (suffix !== '' && !suffix.startsWith('-')) return undefined
	if (suffix.startsWith('-')) {
		const panel = ACCOUNT_SETTINGS_PANELS.find((p) => p === suffix.slice(1))
		if (panel) return panel
	}
	return 'profile'
}

/**
 * `#superadmin-settings`, optionally `#superadmin-settings?tab=<tab id>` to open on one tab.
 * Undefined when the hash is not the instance settings one at all.
 */
export function parseSuperadminSettingsHash(hash: string): { tab: string | undefined } | undefined {
	if (hash === SUPERADMIN_SETTINGS_HASH) return { tab: undefined }
	if (!hash.startsWith(SUPERADMIN_SETTINGS_HASH + '?')) return undefined
	const query = new URLSearchParams(hash.slice(SUPERADMIN_SETTINGS_HASH.length + 1))
	return { tab: query.get('tab') ?? undefined }
}

export function superadminSettingsHref(tab: string): string {
	return `${SUPERADMIN_SETTINGS_HASH}?tab=${encodeURIComponent(tab)}`
}
