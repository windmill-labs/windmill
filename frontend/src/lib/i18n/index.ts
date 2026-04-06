export const SUPPORTED_LOCALES = ['en', 'fr', 'de', 'es', 'pt', 'ja', 'zh', 'ko'] as const
export type Locale = (typeof SUPPORTED_LOCALES)[number]

export const LOCALE_NAMES: Record<Locale, string> = {
	en: 'English',
	fr: 'Français',
	de: 'Deutsch',
	es: 'Español',
	pt: 'Português',
	ja: '日本語',
	zh: '中文',
	ko: '한국어'
}

/** Map browser language (e.g. "fr-FR") to the closest supported locale, fallback to 'en' */
export function detectLocale(): Locale {
	if (typeof navigator === 'undefined') return 'en'
	for (const lang of navigator.languages ?? [navigator.language]) {
		const prefix = lang.split('-')[0].toLowerCase() as Locale
		if (SUPPORTED_LOCALES.includes(prefix)) return prefix
	}
	return 'en'
}
