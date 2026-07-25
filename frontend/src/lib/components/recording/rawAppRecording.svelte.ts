/**
 * Raw-app session recorder: watches a same-origin app iframe and turns what the
 * user does into a step-by-step recording — one step per interaction (click,
 * fill, select, toggle, submit, key), each carrying the DOM before it and the
 * DOM once the app settled after it.
 *
 * The bundle only runs same-origin when the app is not sandbox-isolated (see
 * RawAppPreview): `start` returns false when the document can't be read, and the
 * caller surfaces that instead of silently recording nothing.
 */
import type { RawAppRecording, RawAppStep } from './types'
import {
	cssSelectorFor,
	describeElement,
	isElementNode,
	isRedacted,
	isTag,
	redactedDescription,
	MAX_RECORDED_STEPS,
	maskValue,
	serializeDocument,
	stepLabel,
	textWithoutRedacted,
	type RawAppInteractionKind
} from './rawAppSnapshot'

/** A step's "after" frame is taken once mutations stop for this long… */
const SETTLE_QUIET_MS = 400
/** …but never later than this after the interaction (an app that animates or
 * polls forever would otherwise keep the frame pending). */
const SETTLE_MAX_MS = 3000
/** Typing is one step per field, committed after this much inactivity. */
const FILL_DEBOUNCE_MS = 800
/** `<input>` types that act as buttons: they report a click and never a change. */
const BUTTON_INPUT_TYPES = new Set(['button', 'submit', 'reset', 'image'])
/** `<input>` types whose value moves continuously, so repeats are one gesture. */
const CONTINUOUS_INPUT_TYPES = new Set([
	'range',
	'color',
	'date',
	'time',
	'datetime-local',
	'month',
	'week'
])
/** `<input>` types a user types into. An input with no `type` is one of them. */
const TEXT_INPUT_TYPES = new Set([
	'',
	'text',
	'search',
	'url',
	'tel',
	'email',
	'password',
	'number'
])
/** Snapshots are full documents; stop storing them (steps keep coming) rather
 * than let a long session grow the tab's memory without bound. */
const MAX_TOTAL_FRAME_BYTES = 40 * 1024 * 1024
/** A step's value is shown in a one-line label; a pasted novel is not. */
const MAX_STEP_VALUE_CHARS = 200
/** Repeats of the same interaction on the same control (a held arrow key, a
 * drag along a slider) are one step, not one per event. */
const CONTROL_COALESCE_MS = 250
/** A form submit this soon after a click inside it is that click's consequence. */
const SUBMIT_FOLD_MS = 500

type PendingFill = {
	el: Element
	before: string | undefined
	timer: ReturnType<typeof setTimeout>
}

export type RawAppRecordingStore = {
	readonly active: boolean
	readonly stepCount: number
	/** Attach to a same-origin app iframe. False when its document is unreachable. */
	start(iframe: HTMLIFrameElement, opts: { appPath: string; workspace?: string }): boolean
	stop(): RawAppRecording
	download(recording: RawAppRecording): void
}

export function createRawAppRecording(): RawAppRecordingStore {
	let active = $state(false)
	let stepCount = $state(0)

	let startTime = 0
	let appPath = ''
	let workspace: string | undefined = undefined
	let iframeEl: HTMLIFrameElement | undefined = undefined
	let steps: RawAppStep[] = []
	let frames: string[] = []
	let frameIndexes = new Map<string, number>()
	let framesBytes = 0
	let truncated = false
	/** Set once an interaction was refused: from then on the recording can only
	 * grow, so the expensive snapshotting stops — but the last accepted step still
	 * gets its outcome. */
	let capped = false
	let viewport = { width: 0, height: 0 }
	let baseHref = ''

	let detachers: (() => void)[] = []
	let pendingFill: PendingFill | undefined = undefined
	/** Element and time of the last recorded step, for coalescing repeats of one
	 * interaction. The time is refreshed on every repeat, so a sustained gesture
	 * does not split once it outlives the window. */
	let lastStepEl: Element | undefined = undefined
	let lastStepAt = 0
	/** Pre-interaction snapshot taken on pointerdown, before a click handler runs.
	 * `at` separates a click a label is forwarding right now from a snapshot left
	 * over by an earlier interaction. */
	let pendingPointer: { el: Element; html: string | undefined; at: number } | undefined = undefined
	/** Same, taken on keydown before the key changes the focused field or control.
	 * `repeat` marks it as opening an auto-repeating gesture (a held arrow), whose
	 * changes are one step rather than one per event. */
	let pendingKey: { el: Element; html: string | undefined; repeat: boolean } | undefined = undefined
	type Settle = {
		step: RawAppStep
		observer: MutationObserver
		timer: ReturnType<typeof setTimeout>
		cap: ReturnType<typeof setTimeout>
	}
	let settle: Settle | undefined = undefined

	function doc(): Document | undefined {
		try {
			return iframeEl?.contentDocument ?? undefined
		} catch (_) {
			return undefined
		}
	}

	/** Store a snapshot and return its index. Only frames a step actually
	 * references get here: a pending snapshot is carried as HTML until the step
	 * that needs it exists, so nothing has to be garbage-collected or renumbered
	 * later (and stale indices can't outlive a compaction). */
	function frameIndex(html: string | undefined): number | undefined {
		if (html === undefined) return undefined
		const existing = frameIndexes.get(html)
		if (existing !== undefined) return existing
		if (framesBytes + html.length > MAX_TOTAL_FRAME_BYTES) {
			truncated = true
			return undefined
		}
		const index = frames.length
		frames.push(html)
		frameIndexes.set(html, index)
		framesBytes += html.length
		return index
	}

	/** Serialize the app document. The result is plain HTML — it becomes a frame
	 * only once a step claims it (see {@link frameIndex}). Serializing is the
	 * expensive part and runs on the app's own event path, so a recording that can
	 * no longer accept steps must stop doing it. */
	function capture(target?: Element | null): string | undefined {
		if (capped) return undefined
		const d = doc()
		if (!d) return undefined
		try {
			return serializeDocument(d, { target, baseHref })
		} catch (e) {
			console.warn('raw app recorder: snapshot failed', e)
			return undefined
		}
	}

	function clearSettle() {
		if (!settle) return
		settle.observer.disconnect()
		clearTimeout(settle.timer)
		clearTimeout(settle.cap)
		settle = undefined
	}

	/** Snapshot the app once it stops mutating, as the step's outcome. */
	function scheduleSettle(step: RawAppStep) {
		clearSettle()
		const d = doc()
		if (!d) return
		const finish = () => {
			clearSettle()
			step.after = frameIndex(capture())
		}
		const observer = new MutationObserver(() => {
			if (!settle) return
			clearTimeout(settle.timer)
			settle.timer = setTimeout(finish, SETTLE_QUIET_MS)
		})
		observer.observe(d, { subtree: true, childList: true, attributes: true, characterData: true })
		settle = {
			step,
			observer,
			timer: setTimeout(finish, SETTLE_QUIET_MS),
			cap: setTimeout(finish, SETTLE_MAX_MS)
		}
	}

	function pushStep(
		kind: RawAppInteractionKind,
		el: Element | undefined,
		before: string | undefined,
		value?: string,
		/** This change came from a key the browser is repeating, so it continues the
		 * gesture already recorded rather than starting a new step. */
		keyDriven = false
	) {
		if (!active) return
		const t = Date.now() - startTime
		const last = steps[steps.length - 1]
		const coalesces =
			!!last &&
			!!el &&
			sameTarget(lastStepEl, el) &&
			last.kind === kind &&
			(isContinuousControl(el) || keyDriven) &&
			t - lastStepAt < CONTROL_COALESCE_MS
		// A step's outcome must be settled before the next one starts; the pending
		// snapshot can't be deferred past this point. It can't reuse `before`
		// either: that frame carries the NEW step's target stamp. Runs before the
		// cap check, so the last accepted step keeps its result even when the
		// interaction that follows it is the one that gets refused. Skipped while a
		// gesture is still coalescing: each repeat's outcome would be indexed and
		// then immediately superseded, leaving a full document unreferenced.
		if (settle && !coalesces) {
			const pending = settle.step
			clearSettle()
			pending.after = frameIndex(capture())
		}
		if (steps.length >= MAX_RECORDED_STEPS && !coalesces) {
			truncated = true
			capped = true
			return
		}
		// A no-record subtree opted out of the recording entirely: its text is what
		// names the element and what a select/file step carries as a value, so the
		// step metadata has to be redacted here too — snapshot scrubbing can't
		// reach into `steps`.
		const redacted = !!el && isRedacted(el)
		const target =
			bound(el ? (redacted ? redactedDescription(el) : describeElement(el)) : 'the app') ??
			'the app'
		// A toggle's value is state ('checked'), not content, and the label reads it
		// back — masking it would render every redacted toggle as "Unchecked".
		// Metadata is stored and rendered like a frame is; an unbounded paste would
		// otherwise slip past the snapshot budget in `value` and `label`.
		const bounded =
			value && value.length > MAX_STEP_VALUE_CHARS
				? `${value.slice(0, MAX_STEP_VALUE_CHARS)}…`
				: value
		const shown = redacted && bounded && kind !== 'toggle' ? maskValue(bounded) : bounded
		const label = stepLabel(kind, target, shown)
		if (coalesces && last) {
			// Same control, same kind, still within the sweep: update the step in place
			// so a held arrow key or a slider drag reads as one interaction. Nothing is
			// indexed for a repeat — neither its pre-frame nor an outcome the next
			// repeat would supersede — and the window runs from this update, so a
			// sustained gesture stays one step however long it is held.
			last.value = shown
			last.label = label
			lastStepAt = t
			scheduleSettle(last)
			return
		}
		const step: RawAppStep = {
			t,
			kind,
			label,
			target,
			selector: el && !redacted ? bound(cssSelectorFor(el)) : undefined,
			value: shown,
			before: frameIndex(before)
		}
		steps.push(step)
		// The frames that described this step's "before" are spent. Without this a
		// later interaction on the same element (a keyboard activation of a button
		// already clicked) would reuse them and appear to rewind; with it, a pending
		// frame can wait as long as a native picker session takes.
		pendingPointer = undefined
		pendingKey = undefined
		lastStepEl = el
		lastStepAt = t
		stepCount = steps.length
		scheduleSettle(step)
	}

	/** True when this click landed on a label and will be forwarded to its control,
	 * which then reports the interaction itself — the label's own click is the
	 * duplicate. The forwarded click (target === the control) must NOT fold, or a
	 * button-shaped control, which never fires `change`, would vanish. Interactive
	 * content inside the label isn't forwarded either, so it stays a click step. */
	function labelDrivesControl(el: Element): boolean {
		const label = el.closest('label') as HTMLLabelElement | null
		const control = label?.control
		if (!control || el === control || control.contains(el)) return false
		return !el.closest('a, button, input, select, textarea')
	}

	/** Whether this key can change a control: activation, option picking, or the
	 * first letter of a `<select>` typeahead. */
	function mutatingKey(e: KeyboardEvent): boolean {
		if (e.ctrlKey || e.metaKey || e.altKey) return false
		return (
			e.key.length === 1 ||
			[' ', 'Enter', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(
				e.key
			)
		)
	}

	/** A control whose value `change` reports rather than typing: a select, or any
	 * input that is not a text field — a checkbox, a range, a date picker. They
	 * change on keys that produce no `beforeinput`, so their pre-change frame has
	 * to come from `keydown`. */
	function isControl(el: Element): boolean {
		if (isTag(el, 'SELECT')) return true
		if (!isTag(el, 'INPUT')) return false
		const type = (el as HTMLInputElement).type
		return !isTextEntry(el) && !BUTTON_INPUT_TYPES.has(type)
	}

	/** A control the user sweeps rather than sets once: repeats within a burst are
	 * one interaction. A checkbox is not one — two quick clicks are two toggles. */
	function isContinuousControl(el: Element): boolean {
		return isTag(el, 'INPUT') && CONTINUOUS_INPUT_TYPES.has((el as HTMLInputElement).type)
	}

	/** Metadata is stored and rendered like a frame is; an unbounded paste (or a
	 * pathological selector) would otherwise slip past the snapshot budget. */
	function bound(text: string | undefined): string | undefined {
		if (text === undefined) return undefined
		return text.length > MAX_STEP_VALUE_CHARS ? `${text.slice(0, MAX_STEP_VALUE_CHARS)}…` : text
	}

	/** A field whose value the user types into, character by character. Listed
	 * positively: everything else an `<input>` can be (a range, a colour, a date,
	 * a checkbox) is a control the browser mutates on keys that produce no
	 * `beforeinput`, and must take the control path instead. */
	function isTextEntry(el: Element): boolean {
		if (isTag(el, 'TEXTAREA')) return true
		if ((el as HTMLElement).isContentEditable) return true
		return isTag(el, 'INPUT') && TEXT_INPUT_TYPES.has((el as HTMLInputElement).type)
	}

	function currentValue(el: Element): string {
		// A contenteditable host can be recordable while a node inside it is not.
		const raw = isTag(el, 'INPUT')
			? (el as HTMLInputElement).value
			: isTag(el, 'TEXTAREA')
				? (el as HTMLTextAreaElement).value
				: textWithoutRedacted(el)
		const secret =
			(isTag(el, 'INPUT') && (el as HTMLInputElement).type === 'password') || isRedacted(el)
		return secret ? maskValue(raw) : raw
	}

	/** The pre-interaction frame for `el`, if the pointerdown that started this
	 * interaction landed on it — or on its label / an ancestor, which is what a
	 * click on `<label>Urgent</label>` looks like. */
	function pointerFrameFor(el: Element): string | undefined {
		const from = pendingPointer?.el
		if (!from) return undefined
		if (from === el || from.contains(el)) return pendingPointer?.html
		const labels = (el as HTMLInputElement).labels
		if (labels && Array.from(labels).some((l) => l === from || l.contains(from)))
			return pendingPointer?.html
		return undefined
	}

	/** One interaction target: the same element, or another radio of the same
	 * group — arrows move the selection between them, so `keydown`, `change` and
	 * the step already recorded can each land on a different member. */
	function sameTarget(a: Element | undefined, b: Element | undefined): boolean {
		if (!a || !b) return false
		if (a === b) return true
		const x = a as HTMLInputElement
		const y = b as HTMLInputElement
		return (
			x.type === 'radio' && y.type === 'radio' && !!x.name && x.name === y.name && x.form === y.form
		)
	}

	/** The pre-key snapshot, when the key landed on this interaction target. */
	function keyFrameFor(el: Element): string | undefined {
		if (!pendingKey) return undefined
		return sameTarget(pendingKey.el, el) ? pendingKey.html : undefined
	}

	function commitFill() {
		if (!pendingFill) return
		const { el, before } = pendingFill
		clearTimeout(pendingFill.timer)
		pendingFill = undefined
		// The frames that fed this edit are spent: a later burst in the same field
		// must snapshot itself instead of rewinding to the pre-typing DOM. A
		// pointerdown on something ELSE (the control the user just moved to) is that
		// control's pre-change frame and must survive — `pointerFrameFor` decides,
		// so the label and ancestor cases it accepts are cleared here too.
		if (pointerFrameFor(el) !== undefined) pendingPointer = undefined
		if (pendingKey?.el === el) pendingKey = undefined
		pushStep('fill', el, before, currentValue(el))
	}

	function attach(d: Document) {
		const on = (type: string, fn: (e: any) => void) => {
			d.addEventListener(type, fn, true)
			detachers.push(() => d.removeEventListener(type, fn, true))
		}

		on('pointerdown', (e: PointerEvent) => {
			const el = isElementNode(e.target) ? e.target : undefined
			if (!el) return
			pendingPointer = { el, html: capture(el), at: Date.now() }
		})

		on('click', (e: MouseEvent) => {
			const el = isElementNode(e.target) ? e.target : undefined
			if (!el) return
			// Before any early return: a fill still inside its debounce belongs before
			// whatever this click records, and the control paths below never commit it.
			if (pendingFill && pendingFill.el !== el) commitFill()
			// Controls report their own semantic step on `change`; a click on a text
			// field is just focus. Recording those too would double every step. Derived
			// from `isControl` so a type moving between the two can't double-record.
			if (isTextEntry(el) || isControl(el) || isTag(el, 'OPTION')) return
			// A click on a <label> is also delivered to its control, which reports the
			// real step on `change`. Recording the label click too would double one
			// physical action, with the second step appearing to rewind the state.
			if (labelDrivesControl(el)) return
			// `pendingPointer` is NOT cleared here: a click on a <label> is followed by
			// the control's own `change`, which needs the same pre-click frame. The
			// next pointerdown replaces it.
			pushStep('click', el, pointerFrameFor(el) ?? capture(el))
		})

		// Fires before the DOM changes for every edit, including the ones no keydown
		// describes: paste, cut, undo, drag-and-drop, IME composition.
		on('beforeinput', (e: Event) => {
			const el = isElementNode(e.target) ? e.target : undefined
			if (!el || !isTextEntry(el) || pendingFill?.el === el) return
			if (pointerFrameFor(el) === undefined) pendingKey = { el, html: capture(el), repeat: false }
		})

		on('input', (e: Event) => {
			const el = isElementNode(e.target) ? e.target : undefined
			if (!el || !isTextEntry(el)) return
			if (pendingFill && pendingFill.el !== el) commitFill()
			if (!pendingFill) {
				// The pre-keystroke DOM is gone by the time `input` fires: use the frame
				// taken on the pointerdown that focused the field, or on the keydown
				// that produced this character.
				const before = pointerFrameFor(el) ?? keyFrameFor(el) ?? capture(el)
				pendingFill = { el, before, timer: setTimeout(commitFill, FILL_DEBOUNCE_MS) }
			} else {
				clearTimeout(pendingFill.timer)
				pendingFill.timer = setTimeout(commitFill, FILL_DEBOUNCE_MS)
			}
		})

		on('change', (e: Event) => {
			const el = isElementNode(e.target) ? e.target : undefined
			if (!el) return
			if (isTextEntry(el)) {
				commitFill()
				return
			}
			if (pendingFill) commitFill()
			// `change` fires after the control already holds its new value, so a
			// snapshot taken here is the outcome, not the interaction. Only a frame
			// taken before the key or pointer that caused it will do.
			const before = pointerFrameFor(el) ?? keyFrameFor(el)
			pendingPointer = undefined
			// A sweep keeps its opening frame: every repeat updates one step, whose
			// Interaction is the state before the gesture started. A discrete control
			// consumes it, so its next activation snapshots afresh.
			// Only a browser-generated repeat continues a step; two deliberate presses
			// are two interactions even when they land inside the window.
			const keyDriven = keyFrameFor(el) !== undefined && !!pendingKey?.repeat
			if (isTag(el, 'SELECT')) {
				const options = Array.from((el as HTMLSelectElement).selectedOptions)
				const selected = options.map((o) => o.label || o.value).join(', ')
				// The <select> itself can be recordable while the option picked is not;
				// `pushStep` only looks at the event target, so mask it here.
				const secret = options.some((o) => isRedacted(o))
				pushStep('select', el, before, secret ? maskValue(selected) : selected, keyDriven)
			} else if (isTag(el, 'INPUT')) {
				const input = el as HTMLInputElement
				if (['checkbox', 'radio'].includes(input.type)) {
					pushStep('toggle', el, before, input.checked ? 'checked' : 'unchecked', keyDriven)
				} else if (input.type === 'file') {
					pushStep(
						'fill',
						el,
						before,
						Array.from(input.files ?? [])
							.map((f) => f.name)
							.join(', ')
					)
				} else {
					// Range, date, color…: a value the user picked rather than typed.
					pushStep('fill', el, before, currentValue(el), keyDriven)
				}
			}
		})

		on('submit', (e: Event) => {
			const el = isElementNode(e.target) ? e.target : undefined
			commitFill()
			// Clicking a submit button records the click; the submit that follows is
			// the same action, so it only becomes its own step when nothing in this
			// form was just clicked (Enter in a field, or a programmatic submit).
			const last = steps[steps.length - 1]
			const justClickedInside =
				last?.kind === 'click' &&
				!!lastStepEl &&
				!!el &&
				el.contains(lastStepEl) &&
				Date.now() - startTime - lastStepAt < SUBMIT_FOLD_MS
			if (justClickedInside) return
			pushStep('submit', el, capture(el))
		})

		on('keydown', (e: KeyboardEvent) => {
			const el = isElementNode(e.target) ? e.target : undefined
			// Space on a checkbox, arrows on a select: the key is about to change the
			// control, and this is the last moment the pre-change DOM exists. Text
			// fields are covered by `beforeinput` instead, which also sees paste and
			// undo. Snapshotting is expensive and runs on the app's own event path, so
			// keys that change nothing (Tab, modifiers, navigation) must not trigger it.
			if (el && isControl(el) && mutatingKey(e)) {
				// An auto-repeat continues the gesture the first press opened: keep that
				// frame (re-serializing per repeat would clone the whole document at the
				// key-repeat rate) and only note that the gesture is now repeating.
				if (e.repeat && pendingKey && keyFrameFor(el) !== undefined) pendingKey.repeat = true
				else pendingKey = { el, html: capture(el), repeat: e.repeat }
			}
			if (e.key !== 'Enter' && e.key !== 'Escape') return
			// Enter in a field ends the edit: the fill step must land before the key.
			commitFill()
			pushStep('key', el, capture(el), e.key)
		})
	}

	function detach() {
		detachers.forEach((fn) => fn())
		detachers = []
	}

	/** A reload replaces the document the listeners are bound to. Anything the old
	 * one had in flight (a debounced fill, a pending outcome) refers to detached
	 * nodes and must be dropped, not carried into the new page's timeline. */
	function onIframeLoad() {
		detach()
		if (pendingFill) clearTimeout(pendingFill.timer)
		pendingFill = undefined
		pendingPointer = undefined
		pendingKey = undefined
		clearSettle()
		const d = doc()
		if (!d) return
		attach(d)
		// The wrapper is a blob: URL, so only the in-app hash is meaningful here.
		pushStep('navigate', undefined, capture(), d.location?.hash || undefined)
	}

	return {
		get active() {
			return active
		},
		get stepCount() {
			return stepCount
		},
		start(iframe: HTMLIFrameElement, opts: { appPath: string; workspace?: string }): boolean {
			iframeEl = iframe
			const d = doc()
			if (!d?.documentElement) {
				iframeEl = undefined
				return false
			}
			active = true
			startTime = Date.now()
			appPath = opts.appPath
			workspace = opts.workspace
			steps = []
			lastStepEl = undefined
			lastStepAt = 0
			stepCount = 0
			frames = []
			frameIndexes = new Map()
			framesBytes = 0
			truncated = false
			capped = false
			baseHref = typeof window !== 'undefined' ? window.location.origin : ''
			viewport = {
				width: iframe.clientWidth || d.documentElement.clientWidth,
				height: iframe.clientHeight || d.documentElement.clientHeight
			}
			// frames[0] is the app as recording started: the player opens on it, and
			// it is the one frame no step claims.
			frameIndex(capture())
			attach(d)
			// NOT in `detachers`: onIframeLoad calls detach(), which would otherwise
			// remove the very listener that rebinds the recorder on the next reload.
			iframe.addEventListener('load', onIframeLoad)
			return true
		},
		stop(): RawAppRecording {
			commitFill()
			// The step the user just finished has no settled frame yet — take it now
			// rather than ship a step with no outcome.
			if (settle) {
				const step = settle.step
				clearSettle()
				step.after = frameIndex(capture())
			}
			detach()
			iframeEl?.removeEventListener('load', onIframeLoad)
			active = false
			pendingPointer = undefined
			pendingKey = undefined
			iframeEl = undefined
			const recording: RawAppRecording = {
				version: 1,
				type: 'app',
				recorded_at: new Date().toISOString(),
				app_path: appPath,
				workspace,
				total_duration_ms: Date.now() - startTime,
				viewport,
				frames,
				steps,
				truncated: truncated || undefined
			}
			// Multi-MB snapshots must not outlive the recording they were taken for.
			steps = []
			frames = []
			frameIndexes = new Map()
			framesBytes = 0
			return recording
		},
		download(recording: RawAppRecording) {
			const blob = new Blob([JSON.stringify(recording)], { type: 'application/json' })
			const url = URL.createObjectURL(blob)
			const a = document.createElement('a')
			a.href = url
			a.download = `app-recording-${(recording.app_path || 'untitled').replace(/\//g, '-')}-${Date.now()}.json`
			a.click()
			URL.revokeObjectURL(url)
		}
	}
}
