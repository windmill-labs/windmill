import { describe, it, expect } from 'vitest'
import { groupMemberDiff, isGroupDraftDirty, type GroupDraft, type GroupRole } from './groupDraft'

function baseline(): GroupDraft {
	return {
		summary: 'On-call engineers',
		members: [
			{ member_name: 'admin', role: 'admin' },
			{ member_name: 'alice', role: 'member' }
		]
	}
}

describe('groupMemberDiff', () => {
	// The whole transition matrix: which endpoints each role change maps to. A role is a
	// membership row plus an ACL entry, so only the halves that actually change are sent —
	// an extra call would log a permission-history row for something that did not move.
	const transitions: Array<
		[from: GroupRole | 'absent', to: GroupRole | 'absent', expected: unknown[]]
	> = [
		['absent', 'member', [{ kind: 'addUser', username: 'bob' }]],
		['absent', 'manager', [{ kind: 'setAcl', username: 'bob' }]],
		[
			'absent',
			'admin',
			[
				{ kind: 'addUser', username: 'bob' },
				{ kind: 'setAcl', username: 'bob' }
			]
		],
		['member', 'admin', [{ kind: 'setAcl', username: 'bob' }]],
		[
			'member',
			'manager',
			[
				{ kind: 'removeUser', username: 'bob' },
				{ kind: 'setAcl', username: 'bob' }
			]
		],
		['manager', 'admin', [{ kind: 'addUser', username: 'bob' }]],
		[
			'manager',
			'member',
			[
				{ kind: 'addUser', username: 'bob' },
				{ kind: 'removeAcl', username: 'bob' }
			]
		],
		['admin', 'member', [{ kind: 'removeAcl', username: 'bob' }]],
		['admin', 'manager', [{ kind: 'removeUser', username: 'bob' }]],
		['member', 'absent', [{ kind: 'removeUser', username: 'bob' }]],
		['manager', 'absent', [{ kind: 'removeAcl', username: 'bob' }]],
		[
			'admin',
			'absent',
			[
				{ kind: 'removeUser', username: 'bob' },
				{ kind: 'removeAcl', username: 'bob' }
			]
		]
	]

	for (const [from, to, expected] of transitions) {
		it(`${from} to ${to}`, () => {
			const prev = from === 'absent' ? [] : [{ member_name: 'bob', role: from }]
			const next = to === 'absent' ? [] : [{ member_name: 'bob', role: to }]
			expect(groupMemberDiff(prev, next)).toEqual(expected)
		})
	}

	it('sends nothing for an unchanged member', () => {
		expect(groupMemberDiff(baseline().members, baseline().members)).toEqual([])
	})

	it('revokes the caller last so the rest of the save stays authorized', () => {
		const prev = [{ member_name: 'admin', role: 'admin' as GroupRole }]
		const next = [
			{ member_name: 'admin', role: 'member' as GroupRole },
			{ member_name: 'bob', role: 'admin' as GroupRole }
		]
		expect(groupMemberDiff(prev, next, 'admin')).toEqual([
			{ kind: 'addUser', username: 'bob' },
			{ kind: 'setAcl', username: 'bob' },
			{ kind: 'removeAcl', username: 'admin' }
		])
	})
})

describe('isGroupDraftDirty', () => {
	it('is clean against an equal baseline and dirty on any field', () => {
		expect(isGroupDraftDirty(baseline(), baseline())).toBe(false)
		expect(isGroupDraftDirty({ ...baseline(), summary: 'Other' }, baseline())).toBe(true)
		expect(
			isGroupDraftDirty(
				{ ...baseline(), members: [{ member_name: 'admin', role: 'member' }] },
				baseline()
			)
		).toBe(true)
	})

	it('is clean while nothing has loaded', () => {
		expect(isGroupDraftDirty(baseline(), undefined)).toBe(false)
	})

	// A reload rebuilds the members in the server's order, which is not the order they were
	// added in. Order-sensitive, an applied change would keep Save lit with nothing to send.
	it('ignores the order the members are held in', () => {
		const reordered = baseline()
		reordered.members = [...reordered.members].reverse()
		expect(isGroupDraftDirty(reordered, baseline())).toBe(false)
	})
})
