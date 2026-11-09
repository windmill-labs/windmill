import {
	disposeHighlighter,
	type HighlighterTypes,
	type SupportedLanguages,
	type ThemesType
} from '@pierre/diffs'
import { WorkerPoolManager } from '@pierre/diffs/worker'
import PierreWorkerUrl from '@pierre/diffs/worker/worker.js?worker&url'

export const PIERRE_THEME: ThemesType = { dark: 'pierre-dark', light: 'pierre-light' }

// Keyed by the editor language `scriptLangToEditorLang` returns. Every other id is passed
// through, since Monaco and Shiki share most names.
// for related places search: ADD_NEW_LANG
const SHIKI_LANG_BY_MONACO: Record<string, string> = {
	shell: 'shellscript',
	nu: 'nushell',
	plaintext: 'text'
}

export function pierreLang(monacoLang: string | undefined): SupportedLanguages | undefined {
	if (!monacoLang) return undefined
	return (SHIKI_LANG_BY_MONACO[monacoLang] ?? monacoLang) as SupportedLanguages
}

let mainThreadHighlighter: HighlighterTypes | undefined

/**
 * Components without a pool share one main-thread highlighter, created with the first backend
 * any of them asked for; later `preferredHighlighter` values are ignored until it is disposed.
 * Call before mounting a component that needs another backend, after unmounting the old ones.
 */
export async function switchMainThreadHighlighter(type: HighlighterTypes): Promise<void> {
	if (mainThreadHighlighter !== undefined && mainThreadHighlighter !== type) {
		await disposeHighlighter()
	}
	mainThreadHighlighter = type
}

const pools = new Map<HighlighterTypes, WorkerPoolManager>()

/**
 * One pool per highlighter backend: a pool's backend is fixed when its workers start. Pools live
 * as long as the page, shared by every component that asks; a caller that wants its workers back
 * must `terminate()` the pool and accept that later components restart it.
 */
export function pierreWorkerPool(preferredHighlighter: HighlighterTypes): WorkerPoolManager {
	let pool = pools.get(preferredHighlighter)
	if (!pool) {
		pool = new WorkerPoolManager(
			{
				workerFactory: () => new Worker(PierreWorkerUrl, { type: 'module' }),
				poolSize: Math.min(4, navigator.hardwareConcurrency || 2)
			},
			{ theme: PIERRE_THEME, preferredHighlighter }
		)
		pools.set(preferredHighlighter, pool)
	}
	return pool
}
