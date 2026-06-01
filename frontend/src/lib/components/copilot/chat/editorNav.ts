/**
 * Whether a navigation from `fromPathname` to `toPathname` stays within the
 * same editor entity, so the AI chat about that entity should be preserved
 * across the resulting unmount/remount instead of cleared.
 *
 * Preserves when:
 * - the destination is the SAME entity currently open
 *   (`{editPrefix}A` → `{editPrefix}A`, e.g. a `?selected=…` query change), or
 * - the `{addPathname}` → `{editPrefix}{path}` first-save promotion.
 *
 * Clears for a different entity, or any destination outside this editor.
 *
 * This decides purely from the route pathnames (both reliably available from a
 * `beforeNavigate` callback's `from`/`to`) rather than from editor state such as
 * `flowOptions.path`, which is undefined for a new or draft-only entity — using
 * that as the promotion signal made cross-entity navigation wrongly preserve.
 *
 * Used by FlowEditor / ScriptEditor / RawAppEditor. Non-navigation remounts
 * (e.g. an edit page wiping then reloading its data) fire no `beforeNavigate`,
 * so callers default to preserving in that case.
 */
export function navStaysInEditor(
	fromPathname: string,
	toPathname: string,
	addPathname: string,
	editPrefix: string
): boolean {
	// Leaving this editor entirely (home, a different section, the entity's
	// non-edit pages, …).
	if (!toPathname.startsWith(editPrefix)) return false
	// `{addPathname}` → `{editPrefix}{path}`: the first-save promotion.
	if (fromPathname === addPathname) return true
	// Same entity (e.g. a `?selected=…` query change on the same edit route).
	if (fromPathname.startsWith(editPrefix)) {
		return fromPathname.slice(editPrefix.length) === toPathname.slice(editPrefix.length)
	}
	return false
}
