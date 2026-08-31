import { deepEqual } from 'fast-equals'

/** What a member may hold on a group. `member` is the `usr_to_group` row server-side and
 *  `manager` is the `true` entry in `extra_perms`; `admin` is both at once. */
export type GroupRole = 'member' | 'manager' | 'admin'

export type GroupMember = { member_name: string; role: GroupRole }

/** Everything the group editor can change, held as one value so the whole edit is one
 *  comparison against the loaded group and one Save. */
export type GroupDraft = {
	summary: string
	members: GroupMember[]
}

/** Whether the draft still matches the group it was loaded from. Every field of `GroupDraft`
 *  participates, so a field added to the type is covered by construction — which is what the
 *  discard guard depends on: an edit this misses is an edit the drawer throws away without
 *  asking. No baseline means nothing has loaded yet, so nothing to lose. */
export function isGroupDraftDirty(draft: GroupDraft, baseline: GroupDraft | undefined): boolean {
	return baseline != undefined && !deepEqual(draft, baseline)
}

/** One backend call a group's members need. Kept as data so the mapping from role
 *  transitions to endpoints can be read — and tested — without a server. */
export type GroupMemberCall =
	/** `addUserToGroup` / `removeUserToGroup`: the `usr_to_group` row. */
	| { kind: 'addUser'; username: string }
	| { kind: 'removeUser'; username: string }
	/** `acls/add` / `acls/remove` on kind `group_`: the write entry that lets someone
	 *  manage the group. */
	| { kind: 'setAcl'; username: string }
	| { kind: 'removeAcl'; username: string }

/** The two independent things a role is made of: belonging to the group, and holding the
 *  write entry that lets you manage it. Every role is one combination of the two, which is
 *  why a transition needs at most one call per flag. */
function flagsOf(role: GroupRole | undefined): { belongs: boolean; manages: boolean } {
	return {
		belongs: role === 'member' || role === 'admin',
		manages: role === 'manager' || role === 'admin'
	}
}

/** The calls that turn `prev` into `next`. Members whose role is unchanged produce none, and
 *  a member dropped from `next` is treated as holding neither flag — which is what removing
 *  one means. */
export function groupMemberDiff(prev: GroupMember[], next: GroupMember[]): GroupMemberCall[] {
	const previousRole = new Map(prev.map((p) => [p.member_name, p.role]))
	const calls: GroupMemberCall[] = []

	const transition = (
		username: string,
		before: GroupRole | undefined,
		after: GroupRole | undefined
	) => {
		const from = flagsOf(before)
		const to = flagsOf(after)
		if (to.belongs !== from.belongs) {
			calls.push({ kind: to.belongs ? 'addUser' : 'removeUser', username })
		}
		if (to.manages !== from.manages) {
			calls.push({ kind: to.manages ? 'setAcl' : 'removeAcl', username })
		}
	}

	for (const member of next) {
		const before = previousRole.get(member.member_name)
		if (before === member.role) continue
		transition(member.member_name, before, member.role)
	}

	const kept = new Set(next.map((n) => n.member_name))
	for (const member of prev) {
		if (kept.has(member.member_name)) continue
		transition(member.member_name, member.role, undefined)
	}

	return calls
}
