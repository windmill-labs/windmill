import { describe, it, expect } from 'vitest'
import { fromCaptureDraft } from './evalCaseUtils'
import type { EvalCaseDraft } from '$lib/gen'

describe('fromCaptureDraft', () => {
	// The draft is edited in place by the case editor. If it shared sub-objects with the capture,
	// editing a captured case would mutate the caller's copy — and the reactive proxy a capture
	// arrives wrapped in makes `structuredClone` throw, so the copy is easy to regress.
	it('does not share nested state with the capture it came from', () => {
		const capture = {
			name: 'from a conversation',
			input: {
				user_message: 'still waiting',
				messages: [{ role: 'user', content: 'any update?' }]
			},
			source: { captured_at: '2026-01-01T00:00:00Z' }
		} as EvalCaseDraft

		const draft = fromCaptureDraft(capture)
		draft.input!.user_message = 'edited'
		draft.input!.messages!.push({ role: 'assistant', content: 'added' })

		expect(capture.input.user_message).toBe('still waiting')
		expect(capture.input.messages).toHaveLength(1)
	})
})
