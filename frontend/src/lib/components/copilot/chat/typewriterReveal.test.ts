import { describe, expect, it } from 'vitest'
import { TypewriterReveal, type TypewriterRevealOptions } from './typewriterReveal'

// A fake clock + manual scheduler modelling requestAnimationFrame: each frame()
// advances the clock by `dt` ms, then fires whatever callback is currently
// scheduled (which may reschedule the next). This drives the pacing
// deterministically without a browser.
class FakeScheduler {
	t = 0
	scheduleCount = 0
	private queue = new Map<number, () => void>()
	private id = 0

	now = () => this.t
	schedule = (cb: () => void) => {
		this.scheduleCount++
		const h = ++this.id
		this.queue.set(h, cb)
		return h
	}
	cancel = (h: unknown) => {
		this.queue.delete(h as number)
	}
	pending() {
		return this.queue.size
	}
	frame(dt: number) {
		this.t += dt
		const cbs = [...this.queue.values()]
		this.queue.clear()
		cbs.forEach((cb) => cb())
	}
	frames(count: number, dt = 16) {
		for (let i = 0; i < count; i++) this.frame(dt)
	}
}

function makeReveal(sched: FakeScheduler, opts: Partial<TypewriterRevealOptions> = {}) {
	const chunks: string[] = []
	const reveal = new TypewriterReveal({
		onReveal: (c) => chunks.push(c),
		now: sched.now,
		schedule: sched.schedule,
		cancel: sched.cancel,
		...opts
	})
	return { reveal, chunks, revealed: () => chunks.join('') }
}

// No lone surrogate at any string end (a split pair would leave one).
function hasLoneSurrogate(s: string): boolean {
	for (let i = 0; i < s.length; i++) {
		const c = s.charCodeAt(i)
		if (c >= 0xd800 && c <= 0xdbff) {
			const next = s.charCodeAt(i + 1)
			if (!(next >= 0xdc00 && next <= 0xdfff)) return true
			i++
		} else if (c >= 0xdc00 && c <= 0xdfff) {
			return true
		}
	}
	return false
}

describe('TypewriterReveal', () => {
	it('reveals gradually — a burst is not fully painted on the first frame', () => {
		const sched = new FakeScheduler()
		const { reveal, revealed } = makeReveal(sched)
		const text = 'x'.repeat(300)
		reveal.push(text)
		sched.frame(16)
		expect(revealed().length).toBeGreaterThan(0)
		expect(revealed().length).toBeLessThan(text.length)
	})

	it('preserves text exactly after flush (no loss, no duplication)', () => {
		const sched = new FakeScheduler()
		const { reveal, revealed } = makeReveal(sched)
		const parts = ['Here are ', 'some names: ', 'Charles, George, ', 'Alfred, Harold.']
		parts.forEach((p) => reveal.push(p))
		sched.frames(3) // reveal only part of it
		reveal.flush()
		expect(revealed()).toBe(parts.join(''))
	})

	it('flushes repeatedly across tool boundaries without loss or duplication', () => {
		const sched = new FakeScheduler()
		const { reveal, revealed } = makeReveal(sched)
		// Each segment is a message that ends in a flush (as onMessageEnd does at a
		// tool-call boundary); the buffer is compacted between them.
		const segments = ['first reply', 'second reply', 'third reply']
		segments.forEach((seg) => {
			reveal.push(seg)
			sched.frames(2) // partially reveal
			reveal.flush()
		})
		expect(revealed()).toBe(segments.join(''))
	})

	it('reset() drops un-revealed backlog', () => {
		const sched = new FakeScheduler()
		const { reveal, revealed } = makeReveal(sched)
		const text = 'y'.repeat(300)
		reveal.push(text)
		sched.frame(16) // reveal a prefix
		const afterOneFrame = revealed()
		expect(afterOneFrame.length).toBeLessThan(text.length)
		expect(text.startsWith(afterOneFrame)).toBe(true)
		reveal.reset()
		sched.frames(20)
		expect(revealed()).toBe(afterOneFrame) // nothing further reached onReveal
	})

	it('fast-path: a backlog above the cap is revealed whole in one emit', () => {
		const sched = new FakeScheduler()
		const { reveal, chunks, revealed } = makeReveal(sched, { maxBacklogChars: 1500 })
		const text = 'z'.repeat(2000)
		reveal.push(text)
		sched.frame(16)
		expect(revealed()).toBe(text)
		expect(chunks.length).toBe(1) // dumped, not stretched
	})

	it('never splits a surrogate pair across chunks', () => {
		const sched = new FakeScheduler()
		const { reveal, chunks, revealed } = makeReveal(sched, { smoothingMs: 5000 }) // force ~1 char/frame
		const text = 'a😀b👨‍👩‍👧c'
		reveal.push(text)
		sched.frames(60)
		reveal.flush()
		expect(revealed()).toBe(text)
		chunks.forEach((c) => expect(hasLoneSurrogate(c)).toBe(false))
	})

	it('instant mode reveals synchronously and never schedules', () => {
		const sched = new FakeScheduler()
		const { reveal, revealed } = makeReveal(sched, { instant: true })
		reveal.push('hello ')
		reveal.push('world')
		expect(revealed()).toBe('hello world')
		expect(sched.scheduleCount).toBe(0)
	})

	it('steady-state backlog converges to ~arrivalRate × smoothingMs (no runaway, no stall)', () => {
		const sched = new FakeScheduler()
		const { reveal, revealed } = makeReveal(sched, { smoothingMs: 500 })
		const perFrame = 10 // chars pushed each 16ms frame → 0.625 chars/ms
		let pushed = 0
		for (let i = 0; i < 200; i++) {
			reveal.push('c'.repeat(perFrame))
			pushed += perFrame
			sched.frame(16)
		}
		const backlog = pushed - revealed().length
		// Target ≈ 0.625 * 500 = ~312. Assert it neither ran away nor drained to zero.
		expect(backlog).toBeGreaterThan(50)
		expect(backlog).toBeLessThan(900)
	})

	it('emit frequency stays bounded by the throttle', () => {
		const sched = new FakeScheduler()
		const minEmitIntervalMs = 33
		const { reveal, chunks } = makeReveal(sched, { minEmitIntervalMs })
		const frames = 60
		const dt = 16
		for (let i = 0; i < frames; i++) {
			reveal.push('c'.repeat(10))
			sched.frame(dt)
		}
		const windowMs = frames * dt
		expect(chunks.length).toBeLessThanOrEqual(Math.ceil(windowMs / minEmitIntervalMs) + 2)
	})

	it('suspends when fully revealed and resumes on the next push', () => {
		const sched = new FakeScheduler()
		const { reveal, revealed } = makeReveal(sched)
		reveal.push('short')
		sched.frames(30)
		expect(revealed()).toBe('short')
		expect(sched.pending()).toBe(0) // idle: no live frame scheduled
		reveal.push(' more')
		expect(sched.pending()).toBe(1) // restarted
		sched.frames(30)
		expect(revealed()).toBe('short more')
	})

	it('clamps a large elapsed time (backgrounded-tab resume) instead of blasting the backlog', () => {
		const sched = new FakeScheduler()
		const { reveal, revealed } = makeReveal(sched, { smoothingMs: 500, maxCatchupMs: 100 })
		const text = 'q'.repeat(300)
		reveal.push(text)
		sched.frame(16) // first (nominal) emit; lastEmit now set
		const before = revealed().length
		sched.frame(5000) // loop kept running but the frame fired seconds late
		const revealedInBigFrame = revealed().length - before
		// Without the clamp, rate × 5000ms would reveal the entire backlog at once.
		expect(revealed().length).toBeLessThan(text.length)
		expect(revealedInBigFrame).toBeLessThan(text.length / 2)
	})
})
