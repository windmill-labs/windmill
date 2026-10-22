import { describe, it, expect } from 'vitest'
import { filterMatchesQuery } from './FilterSearchbar.svelte'

describe('filterMatchesQuery', () => {
	const archived = { type: 'boolean' as const, label: 'Only archived' }

	it('matches each word anywhere in the label or key', () => {
		expect(filterMatchesQuery('archived', archived, 'archive')).toBe(true)
		expect(filterMatchesQuery('archived', archived, 'archived only')).toBe(true)
		expect(filterMatchesQuery('archived', archived, 'archived scripts')).toBe(false)
		expect(
			filterMatchesQuery('include_library', { type: 'boolean', label: 'Library' }, 'include lib')
		).toBe(true)
	})

	it('reads an escaped space as a word break', () => {
		expect(filterMatchesQuery('archived', archived, 'archived\\ only')).toBe(true)
	})
})
