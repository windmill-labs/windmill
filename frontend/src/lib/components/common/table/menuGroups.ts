import type { Item } from '$lib/utils'

/**
 * Joins a row menu's groups into one list with a divider above each group. The divider rides on
 * the group's first visible item, so a group whose items are all hidden leaves no stray line.
 */
export function menuGroups(...groups: Item[][]): Item[] {
	const out: Item[] = []
	for (const group of groups) {
		const visible = group.filter((item) => !item.hide)
		if (visible.length === 0) continue
		out.push({ ...visible[0], separatorTop: out.length > 0 }, ...visible.slice(1))
	}
	return out
}

/**
 * A submenu for actions that share a heading. With a single visible child the child is shown
 * in place, since a submenu of one is an extra hover for nothing.
 */
export function submenu(displayName: string, icon: any, children: Item[]): Item {
	const visible = children.filter((item) => !item.hide)
	if (visible.length === 1) return visible[0]
	return { displayName, icon, submenuItems: visible, hide: visible.length === 0 }
}
