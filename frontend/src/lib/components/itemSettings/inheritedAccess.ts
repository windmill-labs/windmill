import type { Folder } from '$lib/gen'

export type AccessRow = {
	/** `u/name` or `g/name`. */
	name: string
	write: boolean
}

/** Who reaches an item through its path, and where that access comes from. */
export type InheritedAccess = {
	/** Shown in the access list's "Access from" column. */
	source: string
	rows: AccessRow[]
	/** Shown in place of the rows when they cannot be listed. */
	note?: string
}

export function byName(a: AccessRow, b: AccessRow): number {
	return a.name.localeCompare(b.name)
}

/** The folder a path lives in, or undefined for a `u/` or `g/` path. */
export function folderOfPath(path: string): string | undefined {
	const parts = path.split('/')
	return parts[0] === 'f' && parts.length >= 2 ? parts[1] : undefined
}

/** The owner a path names, such as `u/alice` or `f/team`. */
export function ownerOfPath(path: string): string {
	return path.split('/').slice(0, 2).join('/')
}

/** Whether moving an item between these paths changes who reaches it through its path. */
export function movesToAnotherOwner(from: string, to: string): boolean {
	return !!from && !!to && ownerOfPath(from) !== ownerOfPath(to)
}

/** An owner as a sentence names it: "folder team", or the `u/` or `g/` name. */
export function ownerLabel(path: string): string {
	const folder = folderOfPath(path)
	return folder !== undefined ? `folder ${folder}` : ownerOfPath(path)
}

/** What moving an item to another owner takes away: the users and groups that reach it through
 *  the old owner and through neither the new one nor the item's own members, and the writers left
 *  with a viewer's role. `groupMembers` lists the usernames in each group that reaches the item
 *  after the move, so that a user one of them covers keeps access; a group missing from it counts
 *  as empty, which can only overstate the loss. Undefined when either owner's members could not be
 *  read. */
export function accessLostByMove(
	from: InheritedAccess | undefined,
	to: InheritedAccess | undefined,
	members: Record<string, boolean>,
	groupMembers: Record<string, string[]> = {}
): { lost: string[]; demoted: string[] } | undefined {
	if (!from || !to || from.note !== undefined || to.note !== undefined) return undefined
	const after = new Map<string, boolean>(to.rows.map((r) => [r.name, r.write]))
	for (const [name, write] of Object.entries(members)) after.set(name, after.get(name) || write)
	for (const [name, write] of [...after]) {
		for (const username of groupMembers[name] ?? []) {
			after.set(`u/${username}`, after.get(`u/${username}`) || write)
		}
	}
	const lost: string[] = []
	const demoted: string[] = []
	for (const row of from.rows) {
		const write = after.get(row.name)
		if (write === undefined) lost.push(row.name)
		else if (row.write && !write) demoted.push(row.name)
	}
	return { lost, demoted }
}

/** Who reaches an item through its path rather than through its own extra_perms: the folder's
 *  owners and members for `f/`, the owning user or group for `u/` and `g/`. Workspace admins
 *  reach everything and are not listed. `folder` is the item's folder, undefined when it could
 *  not be read. Undefined for a name that is not a path, such as a volume's. */
export function inheritedAccess(
	path: string,
	folder: Folder | undefined
): InheritedAccess | undefined {
	const parts = path.split('/')
	const folderName = folderOfPath(path)
	if (folderName !== undefined) {
		const source = `Folder ${folderName}`
		if (folder?.name !== folderName) {
			return { source, rows: [], note: `Members of folder ${folderName}` }
		}
		// Sorted by name: the API returns members in jsonb key order, shortest name first, which
		// reads as no order at all in a folder shared with many people.
		const owners = folder.owners.map((owner) => ({ name: owner, write: true })).sort(byName)
		const members = Object.entries(folder.extra_perms ?? {})
			.filter(([member]) => !folder.owners.includes(member))
			.map(([member, write]) => ({ name: member, write: !!write }))
			.sort(byName)
		return { source, rows: [...owners, ...members] }
	}
	if ((parts[0] === 'u' || parts[0] === 'g') && parts.length >= 2) {
		return {
			source: parts[0] === 'u' ? 'Owner' : 'Owning group',
			rows: [{ name: `${parts[0]}/${parts[1]}`, write: true }]
		}
	}
	return undefined
}
