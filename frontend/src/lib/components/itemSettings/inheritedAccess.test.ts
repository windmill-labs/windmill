import { describe, expect, it } from 'vitest'
import type { Folder } from '$lib/gen'
import { accessLostByMove, inheritedAccess, movesToAnotherOwner } from './inheritedAccess'

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

describe('accessLostByMove', () => {
	const folder = (name: string, extra_perms: Record<string, boolean>) =>
		({ name, owners: [], extra_perms }) as unknown as Folder

	it('takes access from whom neither the new folder nor the item itself covers', () => {
		const from = inheritedAccess(
			'f/sales/sync',
			folder('sales', { 'g/sales': true, 'u/bob': false, 'u/dan': true, 'u/eve': true })
		)
		const to = inheritedAccess('f/ops/sync', folder('ops', { 'u/dan': false, 'u/eve': true }))
		expect(accessLostByMove(from, to, { 'u/bob': false })).toEqual({
			lost: ['g/sales'],
			demoted: ['u/dan']
		})
		expect(movesToAnotherOwner('f/sales/sync', 'f/sales/sync_v2')).toBe(false)
		expect(movesToAnotherOwner('f/sales/sync', 'u/bob/sync')).toBe(true)
	})

	it('says nothing when a folder could not be read', () => {
		const from = inheritedAccess('f/sales/sync', undefined)
		const to = inheritedAccess('u/bob/sync', undefined)
		expect(accessLostByMove(from, to, {})).toBeUndefined()
	})
})
