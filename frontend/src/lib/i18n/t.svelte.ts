import { getLocalSetting, storeLocalSetting } from '$lib/utils'
import { type Locale, SUPPORTED_LOCALES, detectLocale } from './index'
import { en, type MessageKey } from './messages/en'
import { messages } from './messages/index'

export type LocaleChoice = Locale | 'auto'

function getInitialChoice(): LocaleChoice {
	const stored = getLocalSetting('locale')
	if (stored && SUPPORTED_LOCALES.includes(stored as Locale)) {
		return stored as Locale
	}
	return 'auto'
}

let choice = $state<LocaleChoice>(getInitialChoice())

/** The resolved locale actually used for translations */
export function getLocale(): Locale {
	return choice === 'auto' ? detectLocale() : choice
}

/** The raw user choice ('auto' or a specific locale) */
export function getLocaleChoice(): LocaleChoice {
	return choice
}

export function setLocale(l: LocaleChoice) {
	choice = l
	storeLocalSetting('locale', l === 'auto' ? undefined : l)
	if (typeof document !== 'undefined') {
		document.documentElement.lang = getLocale()
	}
}

export function t(key: MessageKey): string {
	const loc = getLocale()
	if (loc === 'en') {
		return en[key]
	}
	return messages[loc]?.[key] ?? en[key]
}
