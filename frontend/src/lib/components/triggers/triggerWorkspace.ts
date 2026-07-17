import { getContext, setContext } from 'svelte'

const TRIGGER_WORKSPACE_KEY = 'triggerWorkspace'

/**
 * Context seam letting the native-trigger editors operate on a workspace other
 * than the globally-active `$workspaceStore`. An AI session can run against a
 * (possibly forked) workspace that differs from the nav workspace WITHOUT
 * switching `$workspaceStore` (see `SessionPicker`), so a host that embeds the
 * trigger editors in that context registers a resolver here.
 *
 * Every workspace-scoped backend call / navigation inside the trigger subtree
 * reads its workspace via {@link getTriggerWorkspace} and falls back to
 * `$workspaceStore` when no resolver is set — the default everywhere outside
 * such a host, so behavior there is unchanged. Mirrors the AI chat manager's
 * `operatingWorkspace` resolver.
 */
export function setTriggerWorkspace(resolver: () => string | undefined): void {
	setContext(TRIGGER_WORKSPACE_KEY, resolver)
}

/**
 * The trigger-workspace resolver set by an embedding host, or `undefined` when
 * none is set (fall back to `$workspaceStore`). Read once during component init;
 * use in a `$derived`: `const ws = $derived(triggerWs?.() ?? $workspaceStore)`.
 */
export function getTriggerWorkspace(): (() => string | undefined) | undefined {
	return getContext<(() => string | undefined) | undefined>(TRIGGER_WORKSPACE_KEY)
}
