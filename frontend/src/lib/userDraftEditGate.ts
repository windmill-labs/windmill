import { onDestroy } from 'svelte'

/**
 * What kind of event opened the gate.
 *
 * - `value`: the user changed something — the event *is* the edit.
 * - `precursor`: a gesture that usually precedes an edit. Needed because a
 *   custom component (a picker, a toggle built out of divs) writes its value
 *   through Svelte state and fires no native value event at all, so waiting for
 *   one would drop those edits. The cost is that a bare click counts too.
 */
export type UserInputKind = 'value' | 'precursor'

/** Events that ARE an edit. `drop` and `paste` matter on their own: text
 * dragged in from another application produces no pointer or key event in this
 * document at all. */
const VALUE_EVENTS = ['input', 'change', 'drop', 'paste'] as const
/** `click` is here for the controls that mutate state from a click handler and
 * fire no native value event — ArgInput's "Add item", say. A mouse always sends
 * `pointerdown` first, but an assistive technology can activate one with a
 * trusted `click` alone, and that is a real edit with nothing else to catch it. */
const PRECURSOR_EVENTS = ['pointerdown', 'keydown', 'click'] as const

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
 * Capture phase puts this ahead of the handler that writes the value, so a gate
 * opened here is already open by the time the edit lands. Listening on the
 * document rather than the editor's own subtree is deliberate: pickers and
 * modals render in portals outside it, and missing a real edit would silently
 * drop the user's work, while opening the gate too eagerly only costs the
 * phantom draft that existed before.
 *
 * Registers for the lifetime of the calling component — call it during init.
 */
export function onUserInput(handle: (kind: UserInputKind) => void): void {
	if (typeof document === 'undefined') return
	const listeners: Array<[string, (e: Event) => void]> = []
	const register = (type: string, kind: UserInputKind) => {
		const onEvent = (e: Event) => {
			// A programmatic `dispatchEvent` is untrusted, which is what keeps the
			// form's own settling from opening the gate it is gated by.
			if (e.isTrusted) handle(kind)
		}
		document.addEventListener(type, onEvent, true)
		listeners.push([type, onEvent])
	}
	for (const type of VALUE_EVENTS) register(type, 'value')
	for (const type of PRECURSOR_EVENTS) register(type, 'precursor')
	onDestroy(() => {
		for (const [type, onEvent] of listeners) document.removeEventListener(type, onEvent, true)
	})
}
