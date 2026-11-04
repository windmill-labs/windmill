import type { PrincipalAccess } from '$lib/gen'

export type AccessItem = PrincipalAccess['items'][number]

/** A path segment under a folder or a space, like `archive` in `f/reporting/archive/q3_rollup`. */
export interface AccessPathNode {
	/** The full path prefix, e.g. `f/reporting/archive`. */
	key: string
	name: string
	subfolders: AccessPathNode[]
	items: { item: AccessItem; name: string }[]
}

/** A top-level row: a folder (`f/x`), a user's space (`u/x`) or a group's space (`g/x`). */
export interface AccessOwnerNode extends AccessPathNode {
	/**
	 * The folder or space itself is held, so every item in it is reachable, items added later
	 * included. False when only some of its items are shared with the principal, one by one.
	 */
	whole: boolean
	/** The level that holding the whole folder or space gives. */
	write: boolean
	/** The `u/` or `g/` grants on the folder that reach the principal. */
	via: string[]
	/** The principal, or one of its groups, is an admin of the folder. */
	admin: boolean
}

export type AccessTreeRow =
	| { type: 'owner'; node: AccessOwnerNode; depth: number; count: number; open: boolean }
	| { type: 'subfolder'; node: AccessPathNode; depth: number; count: number; open: boolean }
	| { type: 'item'; item: AccessItem; name: string; owner: AccessOwnerNode; depth: number }

/** Groups a principal's access by where each item lives, as the home page tree does. */
export function buildAccessTree(access: PrincipalAccess): AccessOwnerNode[] {
	const owners = new Map<string, AccessOwnerNode>()
	const ownerNode = (key: string): AccessOwnerNode => {
		let node = owners.get(key)
		if (!node) {
			node = {
				key,
				name: key,
				whole: false,
				write: false,
				via: [],
				admin: false,
				subfolders: [],
				items: []
			}
			owners.set(key, node)
		}
		return node
	}

	// A folder is held whole only through a grant on the folder itself. One that only shows up in
	// item paths has some of its items shared one by one, and nothing added to it later is reachable.
	for (const folder of access.folders) {
		Object.assign(ownerNode(`f/${folder.name}`), {
			whole: true,
			write: folder.write,
			via: folder.via,
			admin: folder.owner
		})
	}
	for (const item of access.items) {
		const segments = item.path.split('/')
		const owner = ownerNode(segments.slice(0, 2).join('/'))
		// The principal's own `u/` path, and the `g/` path of a group it is in, hold every item
		// under them with write. Any other user's or group's space is reached item by item.
		if (item.reasons.some((r) => r.type === 'own' || r.type === 'group_path')) {
			owner.whole = true
			owner.write = true
		}
		let parent: AccessPathNode = owner
		for (const segment of segments.slice(2, -1)) {
			let child = parent.subfolders.find((s) => s.name === segment)
			if (!child) {
				child = { key: `${parent.key}/${segment}`, name: segment, subfolders: [], items: [] }
				parent.subfolders.push(child)
			}
			parent = child
		}
		parent.items.push({ item, name: segments[segments.length - 1] })
	}

	const rank = (node: AccessOwnerNode) =>
		node.key === access.principal
			? 0
			: node.key.startsWith('f/')
				? 1
				: node.key.startsWith('g/')
					? 2
					: 3
	const roots = [...owners.values()].sort((a, b) => rank(a) - rank(b) || a.key.localeCompare(b.key))
	roots.forEach(sortNode)
	return roots
}

function sortNode(node: AccessPathNode): void {
	node.subfolders.sort((a, b) => a.name.localeCompare(b.name))
	node.items.sort((a, b) => a.name.localeCompare(b.name))
	node.subfolders.forEach(sortNode)
}

export function countItems(node: AccessPathNode, matches: (item: AccessItem) => boolean): number {
	return (
		node.items.filter(({ item }) => matches(item)).length +
		node.subfolders.reduce((n, sub) => n + countItems(sub, matches), 0)
	)
}

/**
 * The rows to render, top to bottom. A filter opens every branch holding a match and drops the
 * rest; without one, every folder and space shows, a folder held whole with no items included.
 */
export function flattenAccessTree(
	roots: AccessOwnerNode[],
	{
		matches,
		isOpen,
		filtering
	}: {
		matches: (item: AccessItem) => boolean
		isOpen: (key: string) => boolean
		filtering: boolean
	}
): AccessTreeRow[] {
	const rows: AccessTreeRow[] = []
	const walk = (node: AccessPathNode, owner: AccessOwnerNode, depth: number) => {
		for (const sub of node.subfolders) {
			const count = countItems(sub, matches)
			if (count === 0) continue
			const open = filtering || isOpen(sub.key)
			rows.push({ type: 'subfolder', node: sub, depth, count, open })
			if (open) walk(sub, owner, depth + 1)
		}
		for (const { item, name } of node.items) {
			if (matches(item)) rows.push({ type: 'item', item, name, owner, depth })
		}
	}
	for (const owner of roots) {
		const count = countItems(owner, matches)
		if (filtering && count === 0) continue
		const open = count > 0 && (filtering || isOpen(owner.key))
		rows.push({ type: 'owner', node: owner, depth: 0, count, open })
		if (open) walk(owner, owner, 1)
	}
	return rows
}
