import { getLocalSetting, storeLocalSetting } from '$lib/utils'
import { type Locale, DEFAULT_LOCALE, SUPPORTED_LOCALES } from './index'
import { en, type MessageKey } from './messages/en'
import { messages } from './messages/index'

function getInitialLocale(): Locale {
	const stored = getLocalSetting('locale')
	if (stored && SUPPORTED_LOCALES.includes(stored as Locale)) {
		return stored as Locale
	}
	return DEFAULT_LOCALE
}

let locale = $state<Locale>(getInitialLocale())

export function getLocale(): Locale {
	return locale
}

export function setLocale(l: Locale) {
	locale = l
	storeLocalSetting('locale', l)
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
