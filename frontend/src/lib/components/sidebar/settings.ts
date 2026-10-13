export const USER_SETTINGS_HASH = '#user-settings' as const
export const SUPERADMIN_SETTINGS_HASH = '#superadmin-settings' as const

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
