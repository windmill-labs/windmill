import { describe, expect, it } from 'vitest'
import { canSpliceFrame, mirrorFrameStart } from './sessionMirrorPayload'

describe('mirrorFrameStart', () => {
	// A resync exists to rescue a receiver whose prefix does not fit. Answering it
	// with a frame that still starts mid-transcript fails the same check again and
	// asks for another one, so the two tabs trade messages and nothing renders.
	it('sends the whole transcript when full, even mid-turn', () => {
		expect(mirrorFrameStart({ total: 40, turnStart: 36, full: true })).toBe(0)
	})

	it('reaches no further back than the running turn', () => {
		expect(mirrorFrameStart({ total: 40, turnStart: 36, full: false })).toBe(36)
	})

	it('caps the tail when the turn itself is long', () => {
		expect(mirrorFrameStart({ total: 40, turnStart: 2, full: false })).toBe(30)
	})

	it('handles a turn that has produced nothing yet', () => {
		expect(mirrorFrameStart({ total: 0, turnStart: 0, full: false })).toBe(0)
	})
})

describe('canSpliceFrame', () => {
	const base = { baseIndex: 30, total: 40, localLength: 35, onSameChat: true }

	it('accepts a tail that lands on a matching prefix', () => {
		expect(canSpliceFrame(base)).toBe(true)
	})

	it('always accepts a full frame', () => {
		expect(canSpliceFrame({ ...base, baseIndex: 0, onSameChat: false })).toBe(true)
	})

	it('rejects a tail when the receiver joined mid-run and has a gap', () => {
		expect(canSpliceFrame({ ...base, localLength: 12 })).toBe(false)
	})

	it('rejects a tail when the receiver holds more than the sender has', () => {
		expect(canSpliceFrame({ ...base, localLength: 44 })).toBe(false)
	})

	it('rejects a tail from a different conversation', () => {
		expect(canSpliceFrame({ ...base, onSameChat: false })).toBe(false)
	})
})
