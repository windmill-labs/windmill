import { describe, it, expect, vi } from 'vitest'

const goto = vi.hoisted(() => vi.fn(async (_path: string) => {}))
vi.mock('$app/navigation', () => ({ goto: (path: string) => goto(path) }))
vi.mock('$app/paths', () => ({ base: '' }))

import { buildFilterUrl, setQuery } from './navigation'

// Parse the (un-based) app path the builder returns against a dummy origin so we can
// assert on pathname / searchParams / hash without caring about ordering.
function parse(appPath: string): URL {
	return new URL(appPath, 'http://x')
}

describe('buildFilterUrl', () => {
	it('drops keys not in validKeys, and nullish/empty values', () => {
		const u = parse(
			buildFilterUrl(
				'/runs',
				{
					status: 'failure',
					path: 'f/foo/bar',
					bogus: 'x',
					user: '',
					tag: null,
					worker: undefined
				},
				{ validKeys: ['status', 'path', 'user', 'tag', 'worker'] }
			)
		)
		expect(u.pathname).toBe('/runs')
		expect(u.searchParams.get('status')).toBe('failure')
		expect(u.searchParams.get('path')).toBe('f/foo/bar')
		expect(u.searchParams.has('bogus')).toBe(false) // not in validKeys
		expect(u.searchParams.has('user')).toBe(false) // empty string dropped
		expect(u.searchParams.has('tag')).toBe(false) // null dropped
		expect(u.searchParams.has('worker')).toBe(false) // undefined dropped
	})

	it('keeps every key when no validKeys are given', () => {
		const u = parse(buildFilterUrl('/runs', { anything: 'yes' }))
		expect(u.searchParams.get('anything')).toBe('yes')
	})

	it('appends a hash and no query when there are no params', () => {
		const u = parse(buildFilterUrl('/schedules', {}, { hash: 'f/a/b' }))
		expect(u.pathname).toBe('/schedules')
		expect(u.search).toBe('')
		expect(u.hash).toBe('#f/a/b')
	})

	it('combines params and hash', () => {
		const u = parse(buildFilterUrl('/schedules', { path: 'f/x' }, { hash: 'f/a/b' }))
		expect(u.searchParams.get('path')).toBe('f/x')
		expect(u.hash).toBe('#f/a/b')
	})
})

describe('setQuery', () => {
	// setQuery reads nothing but `window.location.search`; the node test env has no
	// real `location`, so a stub carrying the query is the whole browser it needs.
	function addressBar(search: string) {
		Object.defineProperty(globalThis, 'location', {
			value: { search },
			writable: true,
			configurable: true
		})
	}

	it('preserves params that only the address bar knows about', async () => {
		goto.mockClear()
		addressBar('?workspace=wm-fork-x&search=foo')

		await setQuery('kind', 'script')

		const u = parse(goto.mock.calls[0][0])
		expect(u.searchParams.get('workspace')).toBe('wm-fork-x')
		expect(u.searchParams.get('search')).toBe('foo')
		expect(u.searchParams.get('kind')).toBe('script')
	})
})
