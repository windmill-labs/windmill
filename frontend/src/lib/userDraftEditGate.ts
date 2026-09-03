import { onDestroy } from 'svelte'

/**
 * A draft is supposed to record what the USER changed, but an editor built
 * from a schema writes into the value on its own: the form materializes a
 * property the stored item never carried (an empty string, `false`, the first
 * option of a required enum, a schema `default`) and deletes one a `showExpr`
 * hides. So merely opening an item whose schema has moved on makes it diverge
 * from the deployed value with nobody having touched it — a draft nobody asked
 * for, cluttering the workspace.
 *
 * An editor guards against that by gating its draft on this: nothing the form
 * settles on counts until the user has actually put something in. Callers
 * decide what a gate covers (the resource editor keys it by workspace, since
 * switching workspaces re-renders the form against a fresh value) and what
 * gating means for them — suspending the autosave, absorbing the settled value
 * into the deployed baseline, or both.
 *
 * `pointerdown` and `keydown` are the two events that precede every human
 * edit, and capture phase puts this ahead of the handler that writes the
 * value, so a gate opened here is already open by the time the edit lands.
 * Listening on the document rather than the editor's own subtree is
 * deliberate: pickers and modals render in portals outside it, and missing a
 * real edit would silently drop the user's work, while opening the gate too
 * eagerly only costs the phantom draft that existed before.
 *
 * Registers for the lifetime of the calling component — call it during init.
 */
export function onUserInput(handle: () => void): void {
	if (typeof document === 'undefined') return
	const onEvent = (e: Event) => {
		if (e.isTrusted) handle()
	}
	document.addEventListener('pointerdown', onEvent, true)
	document.addEventListener('keydown', onEvent, true)
	onDestroy(() => {
		document.removeEventListener('pointerdown', onEvent, true)
		document.removeEventListener('keydown', onEvent, true)
	})
}
