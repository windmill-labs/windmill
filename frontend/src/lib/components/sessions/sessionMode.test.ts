import { describe, it, expect } from 'vitest'
import { sessionTargetHref, withMenuHidden } from './sessionMode.svelte'

describe('sessionTargetHref', () => {
	it('maps each editor kind to its full-page route', () => {
		expect(sessionTargetHref({ kind: 'script', path: 'u/me/foo' })).toBe('/scripts/edit/u/me/foo')
		expect(sessionTargetHref({ kind: 'flow', path: 'u/me/bar' })).toBe('/flows/edit/u/me/bar')
		expect(sessionTargetHref({ kind: 'raw_app', path: 'u/me/app' })).toBe('/apps_raw/edit/u/me/app')
	})

	it('returns undefined for no target or a kind without a full-page route', () => {
		expect(sessionTargetHref(undefined)).toBeUndefined()
		expect(sessionTargetHref({ kind: 'pipeline', path: 'u/me/pipe' })).toBeUndefined()
	})
})

describe('withMenuHidden', () => {
	it('appends the nomenubar flag', () => {
		expect(withMenuHidden('/runs')).toBe('/runs?nomenubar=true')
	})

	it('preserves existing query params and the hash', () => {
		expect(withMenuHidden('/runs?tag=x#frag')).toBe('/runs?tag=x&nomenubar=true#frag')
	})

	it('overwrites an existing nomenubar value instead of duplicating it', () => {
		expect(withMenuHidden('/runs?nomenubar=false')).toBe('/runs?nomenubar=true')
	})

	it('returns unparseable input unchanged', () => {
		expect(withMenuHidden('http://[bad')).toBe('http://[bad')
	})
})
