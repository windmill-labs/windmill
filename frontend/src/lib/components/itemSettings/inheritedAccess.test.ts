import { describe, expect, it } from 'vitest'
import type { Folder } from '$lib/gen'
import { inheritedAccess } from './inheritedAccess'

describe('inheritedAccess', () => {
	it('lists folder owners, then members that are not owners, each by name', () => {
		const folder = {
			name: 'finance',
			owners: ['g/finance'],
			extra_perms: { 'u/zoe': false, 'g/finance': true, 'u/marc': true }
		} as unknown as Folder
		expect(inheritedAccess('f/finance/sync', folder)).toEqual({
			source: 'Folder finance',
			rows: [
				{ name: 'g/finance', write: true },
				{ name: 'u/marc', write: true },
				{ name: 'u/zoe', write: false }
			]
		})
	})

	it('names the folder without listing it when it could not be read', () => {
		const other = { name: 'ops', owners: ['u/x'], extra_perms: {} } as unknown as Folder
		expect(inheritedAccess('f/finance/sync', other)).toEqual({
			source: 'Folder finance',
			rows: [],
			note: 'Members of folder finance'
		})
	})

	it('names the owning user or group', () => {
		expect(inheritedAccess('u/alice/sync', undefined)).toEqual({
			source: 'Owner',
			rows: [{ name: 'u/alice', write: true }]
		})
		expect(inheritedAccess('g/ops/sync', undefined)?.source).toBe('Owning group')
	})
})
