import { getContext, setContext } from 'svelte'

const DETAIL_WORKSPACE_KEY = 'detailWorkspace'

/**
 * Context seam letting the script/flow detail pages and the drawers they open
 * operate on a workspace other than the globally-active `$workspaceStore`. An AI
 * session views an item in its own (possibly forked) workspace WITHOUT switching
 * `$workspaceStore`, which stays on whatever the top nav points at, so
 * `ScriptDetail`/`FlowDetail` register a resolver here from their `workspace` prop.
 *
 * Every workspace-scoped backend call inside the detail subtree — ShareModal,
 * DeployWorkspace, MoveDrawer, PersistentScriptDrawer, ScriptVersionHistory and the
 * saved-input components — reads its workspace via {@link getDetailWorkspace} and
 * falls back to `$workspaceStore` when no resolver is set. No resolver is set
 * anywhere outside a detail component, so every other call site of those shared
 * components is unchanged. Mirrors `triggerWorkspace` and `rawAppWorkspace`.
 *
 * A getter (not a value) so a live `$derived` is read reactively at each call site.
 */
export function setDetailWorkspace(resolver: () => string | undefined): void {
	setContext(DETAIL_WORKSPACE_KEY, resolver)
}

/**
 * The detail-workspace resolver set by an embedding detail component, or `undefined`
 * when none is set (fall back to `$workspaceStore`). Read once during component init;
 * use in a `$derived`: `const ws = $derived(detailWs?.() ?? $workspaceStore)`.
 */
export function getDetailWorkspace(): (() => string | undefined) | undefined {
	return getContext<(() => string | undefined) | undefined>(DETAIL_WORKSPACE_KEY)
}
