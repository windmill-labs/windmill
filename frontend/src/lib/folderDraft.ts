import { deepEqual } from 'fast-equals'
import type { FolderDefaultPermissionedAs } from '$lib/gen'

/** What a member may hold on a folder. `admin` is the `owners` array server-side; `writer`
 *  and `viewer` are the `true`/`false` entries of `extra_perms`. */
export type FolderRole = 'viewer' | 'writer' | 'admin'

export type FolderMember = { owner_name: string; role: FolderRole }

/** Everything the folder editor can change, held as one value so the whole edit is one
 *  comparison against the loaded folder and one Save. */
export type FolderDraft = {
	summary: string
	labels: string[]
	defaultPermissionedAs: FolderDefaultPermissionedAs
	perms: FolderMember[]
}

/** Whether the draft still matches the folder it was loaded from. Every field of
 *  `FolderDraft` participates, so a field added to the type is covered by construction —
 *  which is what the discard guard depends on: an edit this misses is an edit the drawer
 *  throws away without asking. No baseline means nothing has loaded yet, so nothing to lose. */
export function isFolderDraftDirty(draft: FolderDraft, baseline: FolderDraft | undefined): boolean {
	return baseline != undefined && !deepEqual(draft, baseline)
}

/** One backend call the folder's members need. Kept as data so the mapping from role
 *  transitions to endpoints can be read — and tested — without a server. */
export type FolderPermissionCall =
	/** `addowner`: appends to `owners` and sets `extra_perms[owner] = true`. */
	| { kind: 'grantAdmin'; owner: string }
	/** `removeowner` with a write flag: takes the member out of `owners` and sets their
	 *  level, in one statement. The only way down from admin. */
	| { kind: 'demoteAdmin'; owner: string; write: boolean }
	/** `acls/add`: sets `extra_perms[owner]`, for a member who is not an admin. */
	| { kind: 'setAcl'; owner: string; write: boolean }
	/** Both removals. `removeowner` without a write only drops the member from `owners`,
	 *  leaving their `extra_perms` entry — alone it demotes an admin rather than removing
	 *  them, so the ACL delete is not optional. */
	| { kind: 'remove'; owner: string }

/** The calls that turn `prev` into `next`. Members whose role is unchanged produce none.
 *
 *  `callerOwners` is every `owners` entry the caller holds admin through — their own
 *  `u/name` and their groups. Taking one out goes last: the folder update policy matches on
 *  the live `owners`, so the rest of the save is filtered out by RLS, while `require_is_owner`
 *  reads a cached copy and still passes. The handlers discard the empty `UPDATE ... RETURNING`,
 *  so those calls report success having changed nothing. */
export function folderPermissionDiff(
	prev: FolderMember[],
	next: FolderMember[],
	callerOwners?: string[]
): FolderPermissionCall[] {
	const previousRole = new Map(prev.map((p) => [p.owner_name, p.role]))
	const calls: FolderPermissionCall[] = []

	for (const member of next) {
		const before = previousRole.get(member.owner_name)
		if (before === member.role) continue
		if (member.role === 'admin') {
			calls.push({ kind: 'grantAdmin', owner: member.owner_name })
		} else if (before === 'admin') {
			calls.push({
				kind: 'demoteAdmin',
				owner: member.owner_name,
				write: member.role === 'writer'
			})
		} else {
			calls.push({ kind: 'setAcl', owner: member.owner_name, write: member.role === 'writer' })
		}
	}

	const kept = new Set(next.map((n) => n.owner_name))
	for (const member of prev) {
		if (kept.has(member.owner_name)) continue
		calls.push({ kind: 'remove', owner: member.owner_name })
	}

	const revokesCaller = (call: FolderPermissionCall) =>
		(call.kind === 'demoteAdmin' || call.kind === 'remove') &&
		(callerOwners?.includes(call.owner) ?? false)
	return [...calls.filter((c) => !revokesCaller(c)), ...calls.filter(revokesCaller)]
}
