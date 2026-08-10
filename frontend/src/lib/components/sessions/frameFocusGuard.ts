// A session preview frame boots the whole app, and SvelteKit's router focuses that
// frame's document body on every navigation (`reset_focus`, unconditionally) — which
// drags focus off whatever the user is typing in out here, mid-message, while the
// assistant opens or reloads previews. Some previewed pages then take focus a second
// time on their own (Runs autofocuses its filter field).
//
// The transfer cannot be prevented: it happens at the browsing-context level, and
// neither `focusout` nor `blur` is cancelable. So the field is given its focus back,
// and everything here exists to tell that apart from focus the user moved themselves,
// which must be left alone.

// How long a load the host triggered may hand focus back for.
const RECLAIM_WINDOW_MS = 10_000
const HOOK_INTERVAL_MS = 50

function textEntry(el: EventTarget | null): HTMLElement | undefined {
	if (el instanceof HTMLTextAreaElement || el instanceof HTMLInputElement) return el
	if (el instanceof HTMLElement && el.isContentEditable) return el
	return undefined
}

export type FrameFocusGuard = {
	/** Call for every load this host drives: mount, `src` change, in-frame reload. */
	arm: () => void
	destroy: () => void
}

export function createFrameFocusGuard(
	getFrame: () => HTMLIFrameElement | undefined
): FrameFocusGuard {
	// Kept independently of the field below, so leaving it and coming back re-arms.
	let until = 0
	// The field to hand focus back to; re-pointed as focus moves around this document,
	// and undefined whenever it rests somewhere untypable.
	let target: HTMLElement | undefined
	// The user moved focus into the frame themselves — clicked or typed in there, or
	// tabbed out of the field — so the frame holding focus is their doing.
	let userMovedFocus = false
	const hookedDocs = new WeakSet<Document>()
	let hookTimer: ReturnType<typeof setInterval> | undefined
	let listening = false

	// Whether the user acted inside the frame is the whole question, and a click or
	// keypress in there reaches no listener out here — so read it off the frame's own
	// document (same-origin), where those fire ahead of the focus move they cause.
	// The hook has to be in place before the frame's app hydrates, which is well
	// before its load event, so poll for each new document rather than waiting.
	function hookFrameDocument() {
		try {
			const doc = getFrame()?.contentDocument
			if (!doc || hookedDocs.has(doc)) return
			hookedDocs.add(doc)
			const mark = () => (userMovedFocus = true)
			doc.addEventListener('pointerdown', mark, true)
			doc.addEventListener('keydown', mark, true)
		} catch {
			// Defensively cross-origin; stays unhooked, which is never reclaimable.
		}
	}

	// A steal worth undoing: either the frame's document is hooked, so `userMovedFocus`
	// is trustworthy, or — for the tick before that, and for a page that hydrates
	// faster than the poll — focus is parked on the frame's body, the router's
	// telltale. That last check is too coarse to rely on once hooked: a page
	// autofocusing a field of its own would read as a user's click.
	function stealLooksAutomatic(): boolean {
		try {
			const doc = getFrame()?.contentDocument
			if (!doc) return false
			return hookedDocs.has(doc) || !doc.activeElement || doc.activeElement === doc.body
		} catch {
			return false
		}
	}

	function onFocusIn(e: FocusEvent) {
		// Focus entering the frame is never the user picking another field out here
		// (and per spec the frame element is in the new focus chain).
		if (Date.now() > until || e.target === getFrame()) return
		target = textEntry(e.target)
		if (target) userMovedFocus = false
	}

	function onKeyDown(e: KeyboardEvent) {
		// Tab out of the field is the user steering focus, and the frame may well be
		// the next stop — nothing to reclaim. This runs before the field decides
		// whether to consume the key (the chat composer's @-mention and /-command
		// pickers take Tab to accept an item), so take the mark back once the dust
		// settles and the field still has focus.
		const field = target
		if (e.key !== 'Tab' || e.target !== field) return
		userMovedFocus = true
		setTimeout(() => {
			if (document.activeElement === field) userMovedFocus = false
		}, 0)
	}

	function onFocusOut(e: FocusEvent) {
		const field = target
		if (!field || e.target !== field || Date.now() > until) return
		// Both verdicts are read as focus leaves, not when the reclaim resolves: what
		// the user did *before* the transfer is what says whether they meant to leave,
		// while a keystroke that lands in the frame in the gap is a symptom of the steal.
		const userLeft = userMovedFocus
		const looksAutomatic = stealLooksAutomatic()
		// The verdict on this document waits a task, because the transfer into the frame
		// is still in flight here — Chrome still reports the body as document.activeElement
		// — and because focusing from inside the handler moves activeElement but not the
		// keyboard: the in-flight transfer lands last, leaving the caret drawn in the
		// field and the keystrokes going to the frame.
		setTimeout(() => {
			const frame = getFrame()
			if (target !== field || !frame || document.activeElement !== frame) return
			if (userLeft || Date.now() > until || !field.isConnected) return
			if (!looksAutomatic) return
			field.focus({ preventScroll: true })
		}, 0)
	}

	return {
		arm() {
			if (!listening) {
				document.addEventListener('focusin', onFocusIn, true)
				document.addEventListener('keydown', onKeyDown, true)
				document.addEventListener('focusout', onFocusOut, true)
				listening = true
			}
			target = textEntry(document.activeElement)
			until = Date.now() + RECLAIM_WINDOW_MS
			userMovedFocus = false
			hookFrameDocument()
			clearInterval(hookTimer)
			hookTimer = setInterval(() => {
				hookFrameDocument()
				if (Date.now() > until) {
					clearInterval(hookTimer)
					hookTimer = undefined
				}
			}, HOOK_INTERVAL_MS)
		},
		destroy() {
			clearInterval(hookTimer)
			hookTimer = undefined
			if (!listening) return
			document.removeEventListener('focusin', onFocusIn, true)
			document.removeEventListener('keydown', onKeyDown, true)
			document.removeEventListener('focusout', onFocusOut, true)
			listening = false
		}
	}
}
