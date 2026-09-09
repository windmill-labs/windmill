import { SettingService } from '$lib/gen'

/** Drives the banner palette and icon; the subset of `AlertType` that fits an announcement. */
export type InstanceBannerSeverity = 'info' | 'warning' | 'error'

/** Stored shape of the `instance_banner` global setting. Every field is optional: a
 *  stored value can predate a field this code knows about. */
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

/** Mirror `INSTANCE_BANNER_MESSAGE_MAX_LEN` / `INSTANCE_BANNER_LINK_LABEL_MAX_LEN` in
 *  backend/windmill-common/src/global_settings.rs, which reject longer values at write time. */
export const INSTANCE_BANNER_MESSAGE_MAX_LEN = 500
export const INSTANCE_BANNER_LINK_LABEL_MAX_LEN = 60

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

/**
 * Read a stored banner field as a string.
 *
 * The setting is a raw `global_settings` row, so its shape is only ever as good as the
 * writer that last touched it — and anything here that calls `.trim()` on a number throws,
 * taking the whole settings form down with it.
 */
export function bannerString(value: unknown): string {
	return typeof value === 'string' ? value : ''
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
 * The scheme check repeats the one the writers run on purpose. The link becomes the href of
 * an anchor shown to every user of the instance, and this is the last place that can refuse
 * it — a row predating the validator, or written straight to the table, reaches here having
 * passed nothing.
 */
export function resolveInstanceBanner(raw: unknown): ResolvedInstanceBanner | undefined {
	if (!raw || typeof raw !== 'object') return undefined
	const banner = raw as InstanceBanner
	const message = bannerString(banner.message).trim()
	if (banner.enabled !== true || message === '') return undefined

	const severity: InstanceBannerSeverity =
		banner.severity === 'warning' || banner.severity === 'error' ? banner.severity : 'info'
	const rawLink = bannerString(banner.link).trim()
	const link = isHttpUrl(rawLink) ? rawLink : undefined
	const linkLabel =
		bannerString(banner.link_label).trim().slice(0, INSTANCE_BANNER_LINK_LABEL_MAX_LEN) ||
		'Learn more'

	return {
		message: message.slice(0, INSTANCE_BANNER_MESSAGE_MAX_LEN),
		severity,
		dismissible: banner.dismissible !== false,
		link,
		linkLabel,
		fingerprint: JSON.stringify([message, severity, link ?? '', link ? linkLabel : ''])
	}
}

/**
 * Whether a viewer holding `dismissedFingerprint` should see this announcement.
 *
 * A non-dismissible announcement ignores stored dismissals entirely: an admin escalating an
 * existing notice to mandatory must reach the people who already dismissed it, and the
 * fingerprint deliberately does not cover `dismissible`, so nothing else would bring it back.
 */
export function isInstanceBannerVisible(
	banner: ResolvedInstanceBanner | undefined,
	dismissedFingerprint: string
): banner is ResolvedInstanceBanner {
	if (banner == undefined) return false
	return !banner.dismissible || dismissedFingerprint !== banner.fingerprint
}

export async function fetchInstanceBanner(): Promise<ResolvedInstanceBanner | undefined> {
	return resolveInstanceBanner(await SettingService.getGlobal({ key: INSTANCE_BANNER_SETTING }))
}
