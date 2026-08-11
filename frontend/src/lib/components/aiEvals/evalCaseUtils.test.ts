import { describe, it, expect } from 'vitest'
import { comparableCase, fromCaptureDraft, fromStoredCase } from './evalCaseUtils'
import type { EvalCase, EvalCaseDraft } from '$lib/gen'

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

describe('comparableCase', () => {
	// Whether a draft differs from its stored case decides whether Run sends `dataset`/`case_id`
	// or an inline case, and only the former stamps the job so the run shows up in the case's
	// history. The editor materializes keys the API omits, so comparing raw objects reported every
	// untouched case as edited.
	it('treats an untouched stored case as unedited once the editor has filled in its keys', () => {
		const stored = {
			id: 'c1',
			name: 'late order',
			input: { user_message: 'where is my parcel' },
			created_at: '2026-01-01T00:00:00Z',
			created_by: 'admin'
		} as EvalCase

		const draft = fromStoredCase(stored)
		// what EvalCaseEditor writes back for a case with no conversation
		draft.input = { ...draft.input, user_message: 'where is my parcel', messages: undefined }

		expect(comparableCase(draft)).toEqual(comparableCase(fromStoredCase(stored)))
	})

	it('still sees a real edit', () => {
		const stored = {
			id: 'c1',
			input: { user_message: 'where is my parcel' },
			created_at: '2026-01-01T00:00:00Z',
			created_by: 'admin'
		} as EvalCase

		const draft = fromStoredCase(stored)
		draft.input = { ...draft.input, user_message: 'somewhere else' }

		expect(comparableCase(draft)).not.toEqual(comparableCase(fromStoredCase(stored)))
	})
})
