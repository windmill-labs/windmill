import { SettingService } from '$lib/gen'

/** Drives the banner palette and icon; the subset of `AlertType` that fits an announcement. */
export type InstanceBannerSeverity = 'info' | 'warning' | 'error'

/** Stored shape of the `instance_banner` global setting. Every field is optional:
 *  the value can predate a field, or be written by declarative config sync. */
export interface InstanceBanner {
	enabled?: boolean
	message?: string
	severity?: InstanceBannerSeverity
	/** Whether a viewer may dismiss the banner for themselves. Absent means yes. */
	dismissible?: boolean
	link?: string
	link_label?: string
}

export const INSTANCE_BANNER_SETTING = 'instance_banner'

/** Mirrors `INSTANCE_BANNER_MESSAGE_MAX_LEN` in backend/windmill-common/src/global_settings.rs,
 *  which rejects a longer message at write time. */
export const INSTANCE_BANNER_MESSAGE_MAX_LEN = 500

export type ResolvedInstanceBanner = {
	message: string
	severity: InstanceBannerSeverity
	dismissible: boolean
	link?: string
	linkLabel: string
	/** Dismissal token: a viewer who dismissed one announcement sees the next one,
	 *  because editing any displayed part of the banner changes this string. */
	fingerprint: string
}

export function isHttpUrl(value: string): boolean {
	try {
		const url = new URL(value)
		return url.protocol === 'http:' || url.protocol === 'https:'
	} catch {
		return false
	}
}

/**
 * Turn the raw setting into what the banner renders, or `undefined` for "show nothing".
 *
 * The scheme check is not redundant with the one the setter runs: declarative instance
 * config writes `global_settings` rows directly, so a `javascript:`/`data:` link can reach
 * this without passing the API validator — and it would become the href of an anchor shown
 * to every user of the instance.
 */
export function resolveInstanceBanner(raw: unknown): ResolvedInstanceBanner | undefined {
	if (!raw || typeof raw !== 'object') return undefined
	const banner = raw as InstanceBanner
	const message = typeof banner.message === 'string' ? banner.message.trim() : ''
	if (banner.enabled !== true || message === '') return undefined

	const severity: InstanceBannerSeverity =
		banner.severity === 'warning' || banner.severity === 'error' ? banner.severity : 'info'
	const rawLink = typeof banner.link === 'string' ? banner.link.trim() : ''
	const link = isHttpUrl(rawLink) ? rawLink : undefined
	const linkLabel =
		(typeof banner.link_label === 'string' ? banner.link_label.trim() : '') || 'Learn more'

	return {
		message: message.slice(0, INSTANCE_BANNER_MESSAGE_MAX_LEN),
		severity,
		dismissible: banner.dismissible !== false,
		link,
		linkLabel,
		fingerprint: JSON.stringify([message, severity, link ?? '', link ? linkLabel : ''])
	}
}

export async function fetchInstanceBanner(): Promise<ResolvedInstanceBanner | undefined> {
	return resolveInstanceBanner(await SettingService.getGlobal({ key: INSTANCE_BANNER_SETTING }))
}
