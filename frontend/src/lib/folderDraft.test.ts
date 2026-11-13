import { describe, it, expect } from 'vitest'
import {
	folderPermissionDiff,
	isFolderDraftDirty,
	type FolderDraft,
	type FolderMember,
	type FolderRole
} from './folderDraft'

function member(role: FolderRole): FolderMember {
	return { owner_name: 'u/alice', role }
}

function baseline(): FolderDraft {
	return {
		summary: 'Reporting jobs',
		labels: ['prod'],
		defaultPermissionedAs: [{ path_glob: '**', permissioned_as: 'u/admin' }],
		perms: [
			{ owner_name: 'u/admin', role: 'admin' },
			{ owner_name: 'g/all', role: 'runner' }
		]
	}
}

describe('folderPermissionDiff', () => {
	// The whole transition matrix: which endpoint each role change maps to. `admin` lives in
	// `owners` and the other two in `extra_perms`, so leaving admin is the one transition that
	// cannot go through the ACL endpoint.
	const transitions: Array<[from: FolderRole | 'absent', to: FolderRole, expected: unknown]> = [
		['absent', 'runner', { kind: 'setAcl', owner: 'u/alice', level: 'run' }],
		['absent', 'writer', { kind: 'setAcl', owner: 'u/alice', level: 'write' }],
		['absent', 'admin', { kind: 'grantAdmin', owner: 'u/alice' }],
		['runner', 'writer', { kind: 'setAcl', owner: 'u/alice', level: 'write' }],
		['runner', 'admin', { kind: 'grantAdmin', owner: 'u/alice' }],
		['writer', 'runner', { kind: 'setAcl', owner: 'u/alice', level: 'run' }],
		['writer', 'admin', { kind: 'grantAdmin', owner: 'u/alice' }],
		['admin', 'runner', { kind: 'demoteAdmin', owner: 'u/alice', write: false }],
		['admin', 'writer', { kind: 'demoteAdmin', owner: 'u/alice', write: true }]
	]

	it.each(transitions)('%s → %s', (from, to, expected) => {
		const prev = from === 'absent' ? [] : [member(from)]
		expect(folderPermissionDiff(prev, [member(to)])).toEqual([expected])
	})

	// `removeowner` leaves a grant that runs, so a view-only one needs a second call.
	it('admin → viewer demotes, then withholds run', () => {
		expect(folderPermissionDiff([member('admin')], [member('viewer')])).toEqual([
			{ kind: 'demoteAdmin', owner: 'u/alice', write: false },
			{ kind: 'setAcl', owner: 'u/alice', level: 'read' }
		])
	})

	it.each(['runner', 'writer', 'admin'] as const)('%s → removed drops owner and acl', (role) => {
		expect(folderPermissionDiff([member(role)], [])).toEqual([{ kind: 'remove', owner: 'u/alice' }])
	})

	it.each(['runner', 'writer', 'admin'] as const)('%s unchanged calls nothing', (role) => {
		expect(folderPermissionDiff([member(role)], [member(role)])).toEqual([])
	})

	// The caller is a folder admin only through `g/ops`, so that demotion is the one the write
	// policy refuses. Sent first it takes the rest of the save down with it.
	it('gives up the caller own admin last', () => {
		const prev: FolderMember[] = [
			{ owner_name: 'g/ops', role: 'admin' },
			{ owner_name: 'u/bob', role: 'runner' }
		]
		const next: FolderMember[] = [
			{ owner_name: 'g/ops', role: 'runner' },
			{ owner_name: 'u/bob', role: 'admin' }
		]
		expect(folderPermissionDiff(prev, next, ['u/alice', 'g/ops'])).toEqual([
			{ kind: 'grantAdmin', owner: 'u/bob' },
			{ kind: 'demoteAdmin', owner: 'g/ops', write: false }
		])
	})

	// `g/z` is a group the caller belongs to but holds no admin through, so removing it is an
	// ordinary call — queued behind the refused one it would never run.
	it('defers only the rows the caller is an admin through', () => {
		const prev: FolderMember[] = [
			{ owner_name: 'g/a', role: 'admin' },
			{ owner_name: 'g/z', role: 'runner' }
		]
		expect(folderPermissionDiff(prev, [], ['u/alice', 'g/a', 'g/z'])).toEqual([
			{ kind: 'remove', owner: 'g/z' },
			{ kind: 'remove', owner: 'g/a' }
		])
	})

	it('touches only the members that changed', () => {
		const prev: FolderMember[] = [
			{ owner_name: 'u/admin', role: 'admin' },
			{ owner_name: 'g/all', role: 'runner' },
			{ owner_name: 'g/ops', role: 'writer' }
		]
		const next: FolderMember[] = [
			{ owner_name: 'u/admin', role: 'admin' },
			{ owner_name: 'g/all', role: 'writer' }
		]
		expect(folderPermissionDiff(prev, next)).toEqual([
			{ kind: 'setAcl', owner: 'g/all', level: 'write' },
			{ kind: 'remove', owner: 'g/ops' }
		])
	})
})

describe('isFolderDraftDirty', () => {
	it('is clean against its own baseline', () => {
		expect(isFolderDraftDirty(baseline(), baseline())).toBe(false)
	})

	it('is clean before anything has loaded', () => {
		expect(isFolderDraftDirty(baseline(), undefined)).toBe(false)
	})

	// A reload rebuilds the members in the server's order, which is not the order they were
	// added in. Order-sensitive, an applied change would keep Save lit with nothing to send.
	it('ignores the order the members are held in', () => {
		const reordered = baseline()
		reordered.perms = [...reordered.perms].reverse()
		expect(isFolderDraftDirty(reordered, baseline())).toBe(false)
	})

	// Enumerated from the value itself rather than a hand-written list: a field added to
	// `FolderDraft` and to `baseline()` is covered here without anyone remembering to add a
	// case. An edit this misses is one the drawer discards without asking.
	it.each(Object.keys(baseline()) as Array<keyof FolderDraft>)('notices a change to %s', (key) => {
		const edited = baseline()
		if (key === 'summary') edited.summary = 'Something else'
		else if (key === 'labels') edited.labels = [...edited.labels, 'staging']
		else if (key === 'defaultPermissionedAs') edited.defaultPermissionedAs = []
		else if (key === 'perms') edited.perms[1].role = 'writer'
		else throw new Error(`no edit defined for ${key} — add one so the field stays covered`)

		expect(isFolderDraftDirty(edited, baseline())).toBe(true)
	})

	it('notices a member added and a member removed', () => {
		const added = baseline()
		added.perms.push({ owner_name: 'g/ops', role: 'writer' })
		expect(isFolderDraftDirty(added, baseline())).toBe(true)

		const removed = baseline()
		removed.perms.pop()
		expect(isFolderDraftDirty(removed, baseline())).toBe(true)
	})

	it('is clean again once the baseline catches up', () => {
		const saved = baseline()
		saved.summary = 'Renamed'
		expect(isFolderDraftDirty(saved, structuredClone(saved))).toBe(false)
	})
})
