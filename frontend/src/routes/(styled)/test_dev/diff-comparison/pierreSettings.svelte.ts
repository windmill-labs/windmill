import type { BaseDiffOptions, HighlighterTypes, LineDiffTypes, ThemesType } from '@pierre/diffs'
import type { WorkerPoolManager } from '@pierre/diffs/worker'
import { pierreWorkerPool } from '$lib/components/pierre/pierre'

export type PierreSettings = {
	diffStyle: 'split' | 'unified'
	diffIndicators: 'bars' | 'classic' | 'none'
	lineDiffType: LineDiffTypes
	hunkSeparators: 'line-info' | 'line-info-basic' | 'metadata' | 'simple'
	wrap: boolean
	disableBackground: boolean
	disableLineNumbers: boolean
	expandUnchanged: boolean
	highlighter: HighlighterTypes
	useWorkers: boolean
	theme: string
}

export const DEFAULT_SETTINGS: PierreSettings = {
	diffStyle: 'unified',
	diffIndicators: 'bars',
	lineDiffType: 'word-alt',
	hunkSeparators: 'line-info',
	wrap: false,
	disableBackground: false,
	disableLineNumbers: false,
	expandUnchanged: false,
	highlighter: 'shiki-js',
	useWorkers: false,
	theme: 'pierre'
}

export const THEMES: Record<string, ThemesType> = {
	pierre: { light: 'pierre-light', dark: 'pierre-dark' },
	github: { light: 'github-light', dark: 'github-dark' },
	vitesse: { light: 'vitesse-light', dark: 'vitesse-dark' },
	one: { light: 'one-light', dark: 'one-dark-pro' },
	catppuccin: { light: 'catppuccin-latte', dark: 'catppuccin-mocha' }
}

/** The options `FileDiff` and `CodeView` share. */
export function diffOptions(
	s: PierreSettings
): BaseDiffOptions & { hunkSeparators: PierreSettings['hunkSeparators'] } {
	return {
		theme: THEMES[s.theme],
		diffStyle: s.diffStyle,
		diffIndicators: s.diffIndicators,
		lineDiffType: s.lineDiffType,
		hunkSeparators: s.hunkSeparators,
		overflow: s.wrap ? 'wrap' : 'scroll',
		disableBackground: s.disableBackground,
		disableLineNumbers: s.disableLineNumbers,
		expandUnchanged: s.expandUnchanged,
		preferredHighlighter: s.highlighter
	}
}

// Pools are shared across components, so the options last applied are tracked per pool.
const appliedOptions = new WeakMap<WorkerPoolManager, string>()

/**
 * The pool for these settings, with its render options kept in sync: theme and inline-diff
 * granularity belong to the pool when one is used, not to the component. Call during component
 * init; the getter is safe in `$derived` and templates.
 */
export function useSettingsPool(settings: () => PierreSettings): {
	readonly current: WorkerPoolManager | undefined
} {
	const pool = $derived(
		settings().useWorkers ? pierreWorkerPool(settings().highlighter) : undefined
	)
	$effect(() => {
		const s = settings()
		if (!pool) return
		const key = `${s.theme}:${s.lineDiffType}`
		// Setting options re-renders every mounted component and clears the pool's cache.
		// `pierreWorkerPool` starts a pool on the pierre theme and Pierre's default `word-alt`.
		if ((appliedOptions.get(pool) ?? 'pierre:word-alt') === key) return
		appliedOptions.set(pool, key)
		void pool.setRenderOptions({ theme: THEMES[s.theme], lineDiffType: s.lineDiffType })
	})
	return {
		get current() {
			return pool
		}
	}
}
