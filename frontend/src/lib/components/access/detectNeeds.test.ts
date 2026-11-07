import { describe, expect, it } from 'vitest'
import { neededByGrantee, neededByRunAs, type NeededAccess, type Needs } from './detectNeeds'

const root = { type: 'root' } as const
const account = (principal: string) => ({ type: 'account', principal }) as const

function needs(runs_as: Needs['runs_as']): Needs {
	return {
		runs_as,
		references: [
			{
				kind: 'variable',
				path: 'f/x/front',
				access: 'read',
				reader: { type: 'viewer' },
				sources: ['f/x/app › /App.tsx']
			},
			{ kind: 'resource', path: 'f/x/db', access: 'read', reader: root, sources: ['f/x/app › a'] },
			{ kind: 'resource', path: 'f/x/db', access: 'write', reader: root, sources: ['f/x/app › b'] },
			{ kind: 'script', path: 'f/x/helper', access: 'run', reader: root, sources: ['f/x/app › a'] },
			{
				kind: 'variable',
				path: 'f/x/bot_token',
				access: 'read',
				reader: account('u/bot'),
				sources: ['f/x/bot']
			}
		],
		others: [{ kind: 'datatable', path: 'main', reader: root, sources: ['f/x/app › a'] }],
		errors: []
	}
}

const rows = (access: NeededAccess) => access.items.map((i) => `${i.kind}:${i.path}:${i.level}`)

describe('neededByGrantee', () => {
	it('lists what the item reads when it runs as whoever starts it', () => {
		const access = neededByGrantee(needs({ type: 'caller' }), 'u/alice')
		expect(rows(access)).toEqual([
			'variable:f/x/front:read',
			'resource:f/x/db:write',
			'script:f/x/helper:read'
		])
		expect(access.items[1].sources).toEqual(['f/x/app › a', 'f/x/app › b'])
		expect(access.others.map((o) => o.path)).toEqual(['main'])
	})

	it('leaves out what an account reads unless the grantee is that account', () => {
		const owned = needs(account('u/owner'))
		expect(rows(neededByGrantee(owned, 'u/alice'))).toEqual(['variable:f/x/front:read'])
		expect(rows(neededByGrantee(owned, 'u/bot'))).toEqual([
			'variable:f/x/front:read',
			'variable:f/x/bot_token:read'
		])
	})
})

describe('neededByRunAs', () => {
	it('lists what the account reads, not what whoever opens the app reads', () => {
		expect(rows(neededByRunAs(needs(account('u/bot')), 'u/bot'))).toEqual([
			'resource:f/x/db:write',
			'script:f/x/helper:read',
			'variable:f/x/bot_token:read'
		])
	})
})
