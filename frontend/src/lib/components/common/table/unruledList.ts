import { getContext, setContext } from 'svelte'

const KEY = Symbol('unruledList')

/** Marks the rows below as one unruled list (Home's items): no line between rows, a rounded
 * hover, and row actions shown only on the row in use. Every other list keeps its ruled rows. */
export function setUnruledList() {
	setContext(KEY, true)
}

export function inUnruledList(): boolean {
	return getContext<boolean | undefined>(KEY) ?? false
}

/** The divider an ordinary list row draws, or none in an unruled list. */
export function rowRule(unruled: boolean): string {
	return unruled ? 'rounded-md' : 'border-b'
}

/** Row actions in an unruled list: shown for the row hovered, tabbed into, keyboard-selected,
 * or with one of its menus open. Opacity rather than visibility, so Tab still reaches them;
 * focus counts only when visible, since a menu hands focus back to its trigger on close and a
 * mouse user would otherwise keep that row's actions up after leaving it. Needs `group/row`
 * on the row. Set per child, not on the container: the menu trigger (`button[data-menu]`)
 * stays visible so every row ends on the same control. */
export const HOVER_ACTIONS_CLASS =
	'*:opacity-0 *:transition-opacity *:duration-100 group-hover/row:*:opacity-100 group-has-[:focus-visible]/row:*:opacity-100 group-data-[row-keyboard-selected=true]/row:*:opacity-100 *:has-[[aria-expanded=true]]:opacity-100 [&>button[data-menu]]:opacity-100'
