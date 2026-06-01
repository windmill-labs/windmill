/**
 * Whether a navigation destination keeps the user within the same editor
 * entity, so the AI chat about that entity should be preserved across the
 * resulting unmount/remount instead of cleared.
 *
 * Used by FlowEditor / ScriptEditor / RawAppEditor to decide, in a
 * `beforeNavigate` guard, whether leaving for `destPathname` is a real leave
 * (clear the chat) or an intra-editor transition (keep it):
 *
 * - `editPrefix` is the editor's edit-route prefix, e.g. `'/flows/edit/'`.
 * - `currentPath` is the entity currently open. It is `undefined`/`''` on the
 *   add page, which is exactly the `add → edit/{path}` first-save promotion —
 *   that case must preserve, hence the `!currentPath` branch.
 *
 * Note this only covers *navigations*. Non-navigation remounts (e.g. an edit
 * page wiping then reloading its data) fire no `beforeNavigate`, so callers
 * default to preserving in that case.
 */
export function navStaysInEditor(
	destPathname: string,
	editPrefix: string,
	currentPath: string | undefined
): boolean {
	if (!destPathname.startsWith(editPrefix)) return false
	const destPath = destPathname.slice(editPrefix.length)
	return !currentPath || destPath === currentPath
}
