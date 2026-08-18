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
	REC_TARGET_ATTR,
	redactedDescription,
	MAX_RECORDED_STEPS,
	MAX_TOTAL_FRAME_CHARS,
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
/** A step that launched a backend job waits for it instead: the DOM goes quiet
 * while the job runs, so the ordinary settle would capture the spinner as the
 * outcome. Still bounded — a job can outlive anyone's patience. */
const SETTLE_JOB_MAX_MS = 60000
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
	/** Stop is waiting on a runnable the last step kicked off. */
	readonly stopping: boolean
	/** Attach to a same-origin app iframe. False when its document is unreachable. */
	start(iframe: HTMLIFrameElement, opts: { appPath: string; workspace?: string }): boolean
	/** Async because a step still waiting on a backend job has to land before its
	 * outcome can be captured. */
	stop(): Promise<RawAppRecording>
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
	 * Lives until a step spends it or focus leaves what it describes. */
	let pendingPointer: { el: Element; html: string | undefined } | undefined = undefined
	/** Same, taken on keydown before the key changes the focused field or control.
	 * `repeat` marks it as opening an auto-repeating gesture (a held arrow), whose
	 * changes are one step rather than one per event. */
	let pendingKey: { el: Element; html: string | undefined; repeat: boolean } | undefined = undefined
	type Settle = {
		step: RawAppStep
		observer: MutationObserver
		startedAt: number
		timer: ReturnType<typeof setTimeout>
		cap: ReturnType<typeof setTimeout>
	}
	let settle: Settle | undefined = undefined
	/** Request ids the app is waiting on. Outlives a bridge rebind: a reload's
	 * bootstrap requests are seen by the outgoing watch and answered after the next
	 * one binds, so clearing it there would strand them. */
	const inFlight = new Set<unknown>()
	/** Document the response listener is currently bound to, so a navigation is
	 * noticed the moment the new document speaks rather than only at `load`. */
	let boundDoc: Document | undefined = undefined
	/** Runnable calls the app is waiting on right now (`inFlight.size`, as state). */
	let pendingJobs = 0
	let unwatchBridge: (() => void) | undefined = undefined
	let stopping = $state(false)

	/** Resolve once nothing is in flight, the step's job budget is spent, or the
	 * document is gone. Polled rather than driven off the bridge listener so a
	 * response that never arrives still ends the wait. */
	function drainPendingJobs(startedAt: number): Promise<void> {
		return new Promise((resolve) => {
			const tick = () => {
				if (pendingJobs === 0 || Date.now() - startedAt >= SETTLE_JOB_MAX_MS || !doc()) {
					resolve()
					return
				}
				setTimeout(tick, SETTLE_QUIET_MS)
			}
			tick()
		})
	}

	const wait = (ms: number) => new Promise((r) => setTimeout(r, ms))

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
		if (framesBytes + html.length > MAX_TOTAL_FRAME_CHARS) {
			// Nothing more can be stored, so nothing more should be serialized either.
			truncated = true
			capped = true
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

	/** A pre-interaction frame reused as the previous step's outcome must not carry
	 * the incoming target's highlight. */
	function unstamp(html: string): string {
		return html.replace(` ${REC_TARGET_ATTR}=""`, '')
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
			// A backend call is in flight: the app is waiting, not done. Settling now
			// would record the spinner as this interaction's result — the quiet period
			// cannot see the difference, because a document waiting on a job is quiet.
			if (pendingJobs > 0 && settle && Date.now() - settle.startedAt < SETTLE_JOB_MAX_MS) {
				clearTimeout(settle.timer)
				settle.timer = setTimeout(finish, SETTLE_QUIET_MS)
				return
			}
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
			startedAt: Date.now(),
			timer: setTimeout(finish, SETTLE_QUIET_MS),
			// The hard cap only bounds a *mutating* app; one waiting on a job is
			// bounded by SETTLE_JOB_MAX_MS in `finish` instead.
			cap: setTimeout(finish, SETTLE_MAX_MS)
		}
	}

	/** Close out the settling step: the DOM the next interaction starts from IS its
	 * outcome. Must use that interaction's own pre-frame, unstamped — `capture()`
	 * here would already include the new effect, and an older frame predates the
	 * step being settled. */
	function settlePendingStep(before: string | undefined) {
		if (!settle) return
		const pending = settle.step
		clearSettle()
		const fresh =
			before !== undefined && (pendingPointer?.html === before || pendingKey?.html === before)
		pending.after = frameIndex(fresh ? unstamp(before) : capture())
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
		// A repeat already means "same key, still held", so it folds untimed; gating
		// it would split every held key at the OS repeat delay. The window is only for
		// a continuous control, where nothing else separates a drag from the next
		// press.
		const coalesces =
			!!last &&
			!!el &&
			sameTarget(lastStepEl, el) &&
			last.kind === kind &&
			(keyDriven || (isContinuousControl(el) && t - lastStepAt < CONTROL_COALESCE_MS))
		// Before the cap check, so the last accepted step still gets its outcome when
		// the next interaction is the one refused. Skipped mid-gesture: each repeat's
		// outcome would be indexed and immediately superseded.
		if (!coalesces) settlePendingStep(before)
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
		// Metadata is stored and rendered like a frame is; an unbounded paste would
		// otherwise slip past the snapshot budget in `value` and `label`.
		const bounded =
			value && value.length > MAX_STEP_VALUE_CHARS
				? `${value.slice(0, MAX_STEP_VALUE_CHARS)}…`
				: value
		// Whether a marked control is ticked is withheld from the snapshot, so the
		// step must not answer it either. A masked "checked" would read back as
		// Unchecked, so a redacted toggle carries no value and gets a neutral label.
		const shown =
			!redacted || !bounded ? bounded : kind === 'toggle' ? undefined : maskValue(bounded)
		const label = stepLabel(kind, target, shown)
		if (coalesces && last) {
			// One gesture, one step: updated in place, indexing nothing for a repeat, and
			// the window runs from here so a sustained hold never splits.
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
			// A key repeat skips its snapshot because it expects to fold into the step
			// already recorded; when it turns out to start one instead, it still needs
			// a state to open on. Only for keys: for a `change`, a capture taken now
			// would be the outcome, not the interaction.
			before: frameIndex(before ?? (kind === 'key' ? capture(el) : undefined))
		}
		steps.push(step)
		// Spend only the frame this step used: reusing it later rewinds the replay,
		// while clearing both would take one belonging to an interaction still in
		// flight (a control clicked mid-debounce commits the fill first).
		if (before !== undefined && pendingPointer?.html === before) pendingPointer = undefined
		if (before !== undefined && pendingKey?.html === before) pendingKey = undefined
		lastStepEl = el
		lastStepAt = t
		stepCount = steps.length
		scheduleSettle(step)
	}

	/** A label's own click duplicates the control's report of the same interaction.
	 * Only the label's: the forwarded click must not fold, or a button-shaped
	 * control (which never fires `change`) would vanish. */
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

	/** The control a form submits through. */
	function isSubmitter(el: Element): boolean {
		if (isTag(el, 'BUTTON')) return ((el as HTMLButtonElement).type || 'submit') === 'submit'
		return isTag(el, 'INPUT') && ['submit', 'image'].includes((el as HTMLInputElement).type)
	}

	/** Whether the step just recorded was an Enter inside this form — the only key
	 * that submits, so an Escape before a submission must not swallow it. */
	function justPressedKeyInside(form: Element | null | undefined): boolean {
		const last = steps[steps.length - 1]
		return (
			last?.kind === 'key' &&
			last.value === 'Enter' &&
			!!form &&
			!!lastStepEl &&
			form.contains(lastStepEl) &&
			Date.now() - startTime - lastStepAt < SUBMIT_FOLD_MS
		)
	}

	/** A field where Enter inserts a newline rather than committing anything. */
	function isMultiline(el: Element): boolean {
		return isTag(el, 'TEXTAREA') || (el as HTMLElement).isContentEditable
	}

	/** An element the browser activates with Enter or Space by dispatching a click,
	 * which is the event the interaction is recorded from. */
	function isActivatable(el: Element): boolean {
		if (isTag(el, 'BUTTON') || isTag(el, 'SUMMARY')) return true
		if (isTag(el, 'A') && el.hasAttribute('href')) return true
		return isTag(el, 'INPUT') && BUTTON_INPUT_TYPES.has((el as HTMLInputElement).type)
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
		if (from === el || from.contains(el) || el.contains(from)) return pendingPointer?.html
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
		// Spent, so a later burst in this field snapshots itself instead of rewinding.
		// A pointerdown on something else is that control's own pre-change frame and
		// must survive, which is why `pointerFrameFor` decides what is cleared.
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
			pendingPointer = { el, html: capture(el) }
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
			// Enter in a field is already recorded; the browser then synthesises this
			// click on the form's default submitter to carry it out.
			if (e.detail === 0 && isSubmitter(el) && justPressedKeyInside(el.closest('form'))) return
			// A click on a <label> is also delivered to its control, which reports the
			// real step on `change`. Recording the label click too would double one
			// physical action, with the second step appearing to rewind the state.
			if (labelDrivesControl(el)) return
			// `pendingPointer` is NOT cleared here: a click on a <label> is followed by
			// the control's own `change`, which needs the same pre-click frame. The
			// next pointerdown replaces it.
			pushStep('click', el, pointerFrameFor(el) ?? capture(el))
		})

		on('focusin', (e: FocusEvent) => {
			const el = isElementNode(e.target) ? e.target : undefined
			// Focus moved somewhere the pending pointerdown does not describe (Tab to
			// another field), so that frame no longer states what was just before.
			if (pendingPointer && (!el || pointerFrameFor(el) === undefined)) pendingPointer = undefined
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
				const pointerBefore = pointerFrameFor(el)
				const keyBefore = keyFrameFor(el)
				const before = pointerBefore ?? keyBefore ?? capture(el)
				// Arming a fill starts an interaction before any step exists. Typing moves
				// the `value` *property*, invisible to the observer, so the previous
				// settle would otherwise capture this field mid-edit as its outcome.
				settlePendingStep(before)
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
			const pointerBefore = pointerFrameFor(el)
			const keyBefore = keyFrameFor(el)
			const before = pointerBefore ?? keyBefore
			// Left pending: `pushStep` spends the frame it used, and needs this one to
			// recognise the pre-state when settling the step before it. `repeat` is read
			// from the frame this change starts from — a held key leaves the flag set,
			// so a later pointer takeover would otherwise fold into a finished gesture.
			const keyDriven =
				pointerBefore === undefined && keyBefore !== undefined && !!pendingKey?.repeat
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
			// Clicking a submit button — or pressing Enter in a field — records that
			// interaction; the submit that follows is the same action, so it only
			// becomes its own step when nothing in this form just acted (a
			// programmatic submit, or one triggered from outside the form).
			const last = steps[steps.length - 1]
			const justActedInside =
				(last?.kind === 'click' || (last?.kind === 'key' && last.value === 'Enter')) &&
				!!lastStepEl &&
				!!el &&
				el.contains(lastStepEl) &&
				Date.now() - startTime - lastStepAt < SUBMIT_FOLD_MS
			if (justActedInside) return
			pushStep('submit', el, capture(el))
		})

		on('keydown', (e: KeyboardEvent) => {
			const el = isElementNode(e.target) ? e.target : undefined
			// Last moment the pre-change DOM exists for a control key (space, arrows).
			// Text fields go through `beforeinput`, which also sees paste and undo.
			// Snapshotting runs on the app's event path, so inert keys must not trigger it.
			if (el && isControl(el) && mutatingKey(e)) {
				// An auto-repeat continues the gesture the first press opened: keep that
				// frame (re-serializing per repeat would clone the whole document at the
				// key-repeat rate) and only note that the gesture is now repeating.
				if (e.repeat && pendingKey && keyFrameFor(el) !== undefined) pendingKey.repeat = true
				else pendingKey = { el, html: capture(el), repeat: e.repeat }
			}
			if (e.key !== 'Enter' && e.key !== 'Escape') return
			// Enter is already reported as a change, a click, or the fill's own newline,
			// so recording it again would double the interaction. Escape becomes none of
			// those and is only ever recorded here.
			if (e.key === 'Enter' && el && (isControl(el) || isActivatable(el) || isMultiline(el))) return
			// Enter in a field ends the edit: the fill step must land before the key.
			commitFill()
			// A held key is one interaction: `pushStep` folds the repeats into the step
			// the first press opened rather than serializing per repeat.
			pushStep('key', el, e.repeat ? undefined : capture(el), e.key, e.repeat)
		})
	}

	function detach() {
		detachers.forEach((fn) => fn())
		detachers = []
	}

	/** Outstanding `reqId`s on the bundle's bridge (RawAppBackgroundRunner) are the
	 * only sign a step waits on the backend: the request leaves the iframe without
	 * touching the DOM. By id, so an async dispatch awaited later still counts; on
	 * both windows, since request and response travel opposite ways. */
	function watchRunnableBridge(iframe: HTMLIFrameElement) {
		const bundle = iframe.contentWindow
		const onResponse = (e: MessageEvent) => {
			const data = e.data
			if (!data || typeof data !== 'object' || e.source !== window) return
			const { type, reqId } = data as { type?: unknown; reqId?: unknown }
			if (typeof type !== 'string' || !type.endsWith('Res')) return
			inFlight.delete(reqId)
			pendingJobs = inFlight.size
		}
		// Bound off the request, not just `load`: a reloaded document can be answered
		// while a slow subresource still holds `load` open, and the request both
		// proves the document is live and always precedes its own response.
		const bindResponses = () => {
			const d = doc()
			if (!d || d === boundDoc) return
			// Same handler identity, so re-adding it to a window that kept the old
			// registration is a no-op.
			bundle?.addEventListener('message', onResponse)
			boundDoc = d
		}
		const onRequest = (e: MessageEvent) => {
			const data = e.data
			if (!data || typeof data !== 'object' || e.source !== bundle) return
			const { type, reqId } = data as { type?: unknown; reqId?: unknown }
			if (typeof type !== 'string' || type.endsWith('Res') || reqId === undefined) return
			bindResponses()
			inFlight.add(reqId)
			pendingJobs = inFlight.size
		}
		window.addEventListener('message', onRequest)
		bindResponses()
		// Only the listeners: `inFlight` outlives a rebind on purpose (see its
		// declaration), and stop() is what finally empties it.
		return () => {
			window.removeEventListener('message', onRequest)
			bundle?.removeEventListener('message', onResponse)
			boundDoc = undefined
		}
	}

	/** A reload replaces the document the listeners are bound to. Anything the old
	 * one had in flight (a debounced fill, a pending outcome) refers to detached
	 * nodes and must be dropped, not carried into the new page's timeline. */
	function onIframeLoad() {
		detach()
		// The interaction that triggered the load never settled: its outcome is the
		// document that has just replaced the one it acted on.
		const reloadedFrom = settle?.step
		clearSettle()
		if (pendingFill) clearTimeout(pendingFill.timer)
		pendingFill = undefined
		pendingPointer = undefined
		pendingKey = undefined
		const d = doc()
		if (!d) return
		attach(d)
		// Half the runnable bridge is bound to the document's window, which the
		// reload replaced: rebind the listeners. What is in flight carries over —
		// the new document's bootstrap requests were seen by the outgoing watch.
		if (iframeEl) {
			unwatchBridge?.()
			unwatchBridge = watchRunnableBridge(iframeEl)
		}
		const loaded = capture()
		if (reloadedFrom) reloadedFrom.after = frameIndex(loaded)
		// Recording started before the app had loaded: this document IS the initial
		// state, not a navigation away from one.
		if (frames.length === 0) {
			frameIndex(loaded)
			return
		}
		// The wrapper is a blob: URL, so only the in-app hash is meaningful here.
		pushStep('navigate', undefined, loaded, d.location?.hash || undefined)
	}

	return {
		get active() {
			return active
		},
		get stepCount() {
			return stepCount
		},
		/** Stop is waiting on a runnable the last step kicked off. */
		get stopping() {
			return stopping
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
			// frames[0] is the app as recording started, the one frame no step claims.
			// Capturing a not-yet-loaded `about:blank` would open the replay empty and
			// make the real initial DOM look like a navigation, so defer to `load`.
			if (d.readyState === 'complete' && d.location?.href !== 'about:blank') {
				frameIndex(capture())
			}
			attach(d)
			// NOT in `detachers`: onIframeLoad calls detach(), which would otherwise
			// remove the very listener that rebinds the recorder on the next reload.
			iframe.addEventListener('load', onIframeLoad)
			inFlight.clear()
			pendingJobs = 0
			unwatchBridge = watchRunnableBridge(iframe)
			return true
		},
		async stop(): Promise<RawAppRecording> {
			commitFill()
			// Interactions stop counting the moment Stop is pressed, but the bridge
			// stays up through the drain below.
			detach()
			active = false
			// The last step is still waiting on the backend: capturing now would ship
			// its spinner as the outcome, and the bridge is torn down right after, so
			// nothing could correct it later. Same budget the scheduled settle spends.
			if (settle && pendingJobs > 0) {
				stopping = true
				const startedAt = settle.startedAt
				await drainPendingJobs(startedAt)
				// The response still has to render before it is the outcome.
				if (doc()) await wait(SETTLE_QUIET_MS)
				stopping = false
			}
			// The step the user just finished has no settled frame yet — take it now
			// rather than ship a step with no outcome.
			if (settle) {
				const step = settle.step
				clearSettle()
				step.after = frameIndex(capture())
			}
			unwatchBridge?.()
			unwatchBridge = undefined
			inFlight.clear()
			pendingJobs = 0
			iframeEl?.removeEventListener('load', onIframeLoad)
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
