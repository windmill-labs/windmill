import { describe, expect, it } from 'vitest'
import type { EvalExperiment } from '$lib/gen'
import { parseThreshold, subjectLabel } from './evalUtils'

describe('parseThreshold', () => {
	it('keeps 0 as a threshold and reads only empty text as no threshold', () => {
		expect(parseThreshold(0)).toEqual({ value: 0, error: false })
		expect(parseThreshold('0')).toEqual({ value: 0, error: false })
		expect(parseThreshold('')).toEqual({ error: false })
		expect(parseThreshold('  ')).toEqual({ error: false })
		expect(parseThreshold(null)).toEqual({ error: false })
		expect(parseThreshold(undefined)).toEqual({ error: false })
	})

	it('refuses anything outside 0 to 1 or not a number', () => {
		expect(parseThreshold('0.5')).toEqual({ value: 0.5, error: false })
		expect(parseThreshold('1')).toEqual({ value: 1, error: false })
		expect(parseThreshold('1.5')).toEqual({ error: true })
		expect(parseThreshold('-0.1')).toEqual({ error: true })
		expect(parseThreshold('abc')).toEqual({ error: true })
	})
})

describe('subjectLabel', () => {
	function run(subject: Record<string, unknown>): EvalExperiment {
		return { subject: { path: 'u/me/agent', ...subject } } as unknown as EvalExperiment
	}

	it('names a deployed run and a pinned version by their number', () => {
		expect(subjectLabel(run({ kind: 'agent', version: 4 }))).toBe('v4')
		expect(subjectLabel(run({ kind: 'agent_version', version: 2 }))).toBe('v2')
	})

	it('says a draft run is edits on top of the version it was an edit of', () => {
		expect(subjectLabel(run({ kind: 'agent_draft', version: 4, draft_hash: 'h1' }))).toBe(
			'v4 + edits'
		)
		expect(subjectLabel(run({ kind: 'agent_draft', draft_hash: 'h1' }))).toBe('edits')
	})

	it('reads a draft whose configuration is now deployed as the current version', () => {
		const draft = run({ kind: 'agent_draft', version: 4, draft_hash: 'h1' })
		expect(subjectLabel(draft, 'h1', 5)).toBe('v5')
		expect(subjectLabel(draft, 'other', 5)).toBe('v4 + edits')
	})
})
