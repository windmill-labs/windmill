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
