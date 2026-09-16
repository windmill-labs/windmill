import { describe, it, expect, vi, afterEach } from 'vitest'
import { Turn } from './turn.svelte'

function turnWith(onPoll = vi.fn()) {
	return new Turn({ conversationId: 'a', onReveal: vi.fn(), onPoll })
}

afterEach(() => {
	vi.useRealTimers()
})

/**
 * A turn is the thing the chat checks its own work against, so the only property that has to
 * hold is that an ended one says so and leaves nothing of itself running.
 */
describe('a turn that has ended', () => {
	it('answers that it has, and aborts what it started', () => {
		const turn = turnWith()
		expect(turn.ended).toBe(false)
		expect(turn.signal.aborted).toBe(false)

		turn.end()

		expect(turn.ended).toBe(true)
		expect(turn.signal.aborted).toBe(true)
	})

	it('stops polling, and cannot be started again', () => {
		vi.useFakeTimers()
		const onPoll = vi.fn()
		const turn = turnWith(onPoll)
		turn.startPolling()
		vi.advanceTimersByTime(1500)
		expect(onPoll).toHaveBeenCalled()
		const ticks = onPoll.mock.calls.length

		turn.end()
		// Both timers go together: a deadline left armed by a turn that stopped early would
		// fire into whatever is polling by the time it came round.
		turn.startPolling()
		vi.advanceTimersByTime(10 * 60 * 1000)

		expect(onPoll.mock.calls.length).toBe(ticks)
	})

	it('gives the reader back the chat after two minutes of a run nobody is watching', () => {
		vi.useFakeTimers()
		const onPoll = vi.fn()
		const turn = turnWith(onPoll)
		turn.startPolling()

		vi.advanceTimersByTime(2 * 60 * 1000)
		const ticks = onPoll.mock.calls.length
		vi.advanceTimersByTime(60 * 1000)

		expect(onPoll.mock.calls.length).toBe(ticks)
		// The deadline stops the reading, not the turn: the run is still the one that says
		// when it is over.
		expect(turn.ended).toBe(false)
	})
})
