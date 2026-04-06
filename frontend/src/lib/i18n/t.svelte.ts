import { type Locale, SUPPORTED_LOCALES, detectLocale } from './index'
import { en, type MessageKey } from './messages/en'
import { messages } from './messages/index'

/**
 * Persist locale in localStorage directly (not via getLocalSetting)
 * to survive as long as possible — localStorage has no expiry.
 */
const STORAGE_KEY = 'wm_locale'

function readStored(): Locale | undefined {
	try {
		const v = localStorage.getItem(STORAGE_KEY)
		if (v && SUPPORTED_LOCALES.includes(v as Locale)) return v as Locale
	} catch {}
	return undefined
}

function writeStored(l: Locale) {
	try {
		localStorage.setItem(STORAGE_KEY, l)
	} catch {}
}

/** Default is English. User can switch to any supported locale. */
let locale = $state<Locale>(readStored() ?? 'en')

export function getLocale(): Locale {
	return locale
}

/** Whether the user's browser language differs from the active locale */
export function detectedDiffers(): boolean {
	return detectLocale() !== locale
}

/** The browser-detected locale (for the alert banner) */
export function getDetectedLocale(): Locale {
	return detectLocale()
}

export function setLocale(l: Locale) {
	locale = l
	writeStored(l)
	if (typeof document !== 'undefined') {
		document.documentElement.lang = l
	}
}

export function t(key: MessageKey): string {
	if (locale === 'en') {
		return en[key]
	}
	return messages[locale]?.[key] ?? en[key]
}
