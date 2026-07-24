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
	MAX_RECORDED_STEPS,
	maskValue,
	serializeDocument,
	stepLabel,
	type RawAppInteractionKind
} from './rawAppSnapshot'

/** A step's "after" frame is taken once mutations stop for this long… */
const SETTLE_QUIET_MS = 400
/** …but never later than this after the interaction (an app that animates or
 * polls forever would otherwise keep the frame pending). */
const SETTLE_MAX_MS = 3000
/** Typing is one step per field, committed after this much inactivity. */
const FILL_DEBOUNCE_MS = 800
/** Snapshots are full documents; stop storing them (steps keep coming) rather
 * than let a long session grow the tab's memory without bound. */
const MAX_TOTAL_FRAME_BYTES = 40 * 1024 * 1024

type PendingFill = {
	el: Element
	before: number | undefined
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
	let viewport = { width: 0, height: 0 }
	let baseHref = ''

	let detachers: (() => void)[] = []
	let pendingFill: PendingFill | undefined = undefined
	/** Pre-interaction snapshot taken on pointerdown, before a click handler runs. */
	let pendingPointer: { el: Element; frame: number | undefined } | undefined = undefined
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

	function frameIndex(html: string): number | undefined {
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

	function capture(target?: Element | null): number | undefined {
		const d = doc()
		if (!d) return undefined
		try {
			return frameIndex(serializeDocument(d, { target, baseHref }))
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
			step.after = capture()
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
		before: number | undefined,
		value?: string
	) {
		if (!active) return
		if (steps.length >= MAX_RECORDED_STEPS) {
			truncated = true
			return
		}
		// A step's outcome must be settled before the next one starts; the pending
		// snapshot can't be deferred past this point. It can't reuse `before`
		// either: that frame carries the NEW step's target stamp.
		if (settle) {
			const pending = settle.step
			clearSettle()
			pending.after = capture()
		}
		const target = el ? describeElement(el) : 'the app'
		const step: RawAppStep = {
			t: Date.now() - startTime,
			kind,
			label: stepLabel(kind, target, value),
			target,
			selector: el ? cssSelectorFor(el) : undefined,
			value,
			before
		}
		steps.push(step)
		stepCount = steps.length
		scheduleSettle(step)
	}

	function isTextEntry(el: Element): boolean {
		if (isTag(el, 'TEXTAREA')) return true
		if ((el as HTMLElement).isContentEditable) return true
		return (
			isTag(el, 'INPUT') &&
			!['checkbox', 'radio', 'file', 'submit', 'button', 'reset'].includes(
				(el as HTMLInputElement).type
			)
		)
	}

	function currentValue(el: Element): string {
		const raw = isTag(el, 'INPUT')
			? (el as HTMLInputElement).value
			: isTag(el, 'TEXTAREA')
				? (el as HTMLTextAreaElement).value
				: (el.textContent ?? '').trim()
		const secret =
			(isTag(el, 'INPUT') && (el as HTMLInputElement).type === 'password') || isRedacted(el)
		return secret ? maskValue(raw) : raw
	}

	function commitFill() {
		if (!pendingFill) return
		const { el, before } = pendingFill
		clearTimeout(pendingFill.timer)
		pendingFill = undefined
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
			pendingPointer = { el, frame: capture(el) }
		})

		on('click', (e: MouseEvent) => {
			const el = isElementNode(e.target) ? e.target : undefined
			if (!el) return
			// Controls report their own semantic step on `change`; a click on a text
			// field is just focus. Recording those too would double every step.
			if (isTextEntry(el)) return
			if (
				isTag(el, 'INPUT') &&
				['checkbox', 'radio', 'file'].includes((el as HTMLInputElement).type)
			)
				return
			if (isTag(el, 'SELECT') || isTag(el, 'OPTION')) return
			commitFill()
			const before = pendingPointer?.el === el ? pendingPointer.frame : capture(el)
			pendingPointer = undefined
			pushStep('click', el, before)
		})

		on('input', (e: Event) => {
			const el = isElementNode(e.target) ? e.target : undefined
			if (!el || !isTextEntry(el)) return
			if (pendingFill && pendingFill.el !== el) commitFill()
			if (!pendingFill) {
				// The pre-keystroke DOM is gone by the first `input`; the pointerdown
				// snapshot of the same field is the closest pre-typing state.
				const before = pendingPointer?.el === el ? pendingPointer.frame : capture(el)
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
			const before = pendingPointer?.el === el ? pendingPointer.frame : capture(el)
			pendingPointer = undefined
			if (isTag(el, 'SELECT')) {
				const selected = Array.from((el as HTMLSelectElement).selectedOptions)
					.map((o) => o.label || o.value)
					.join(', ')
				pushStep('select', el, before, selected)
			} else if (isTag(el, 'INPUT')) {
				const input = el as HTMLInputElement
				if (['checkbox', 'radio'].includes(input.type)) {
					pushStep('toggle', el, before, input.checked ? 'checked' : 'unchecked')
				} else if (input.type === 'file') {
					pushStep(
						'fill',
						el,
						before,
						Array.from(input.files ?? [])
							.map((f) => f.name)
							.join(', ')
					)
				}
			}
		})

		on('submit', (e: Event) => {
			const el = isElementNode(e.target) ? e.target : undefined
			commitFill()
			pushStep('submit', el, capture(el))
		})

		on('keydown', (e: KeyboardEvent) => {
			if (e.key !== 'Enter' && e.key !== 'Escape') return
			const el = isElementNode(e.target) ? e.target : undefined
			// Enter in a field ends the edit: the fill step must land before the key.
			commitFill()
			pushStep('key', el, capture(el), e.key)
		})
	}

	function detach() {
		detachers.forEach((fn) => fn())
		detachers = []
	}

	/** Every pointerdown snapshots the app, but only the ones a step turned out to
	 * need are worth downloading — drop the rest and renumber the references. */
	function compactFrames(): { frames: string[]; steps: RawAppStep[] } {
		const remap = new Map<number, number>()
		const kept: string[] = []
		const keep = (i: number | undefined) => {
			if (i === undefined) return undefined
			const existing = remap.get(i)
			if (existing !== undefined) return existing
			const next = kept.length
			kept.push(frames[i])
			remap.set(i, next)
			return next
		}
		// Frame 0 is the app as it was when recording started; the player opens on it.
		keep(0)
		for (const step of steps) {
			step.before = keep(step.before)
			step.after = keep(step.after)
		}
		return { frames: kept, steps }
	}

	/** A reload replaces the document the listeners are bound to. Anything the old
	 * one had in flight (a debounced fill, a pending outcome) refers to detached
	 * nodes and must be dropped, not carried into the new page's timeline. */
	function onIframeLoad() {
		detach()
		if (pendingFill) clearTimeout(pendingFill.timer)
		pendingFill = undefined
		pendingPointer = undefined
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
			stepCount = 0
			frames = []
			frameIndexes = new Map()
			framesBytes = 0
			truncated = false
			baseHref = typeof window !== 'undefined' ? window.location.origin : ''
			viewport = {
				width: iframe.clientWidth || d.documentElement.clientWidth,
				height: iframe.clientHeight || d.documentElement.clientHeight
			}
			capture()
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
				step.after = capture()
			}
			detach()
			iframeEl?.removeEventListener('load', onIframeLoad)
			active = false
			pendingPointer = undefined
			iframeEl = undefined
			const recording: RawAppRecording = {
				version: 1,
				type: 'app',
				recorded_at: new Date().toISOString(),
				app_path: appPath,
				workspace,
				total_duration_ms: Date.now() - startTime,
				viewport,
				...compactFrames(),
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
