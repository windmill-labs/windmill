import type { Locale } from '../index'
import { en, type MessageKey } from './en'
import { fr } from './fr'
import { de } from './de'
import { es } from './es'
import { pt } from './pt'
import { ja } from './ja'
import { zh } from './zh'
import { ko } from './ko'

export type { MessageKey }

export const messages: Record<Locale, Record<MessageKey, string>> = {
	en,
	fr,
	de,
	es,
	pt,
	ja,
	zh,
	ko
}
