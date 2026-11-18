import { describe, it, expect } from 'vitest'
import { parseChanges } from './triggerHistoryChanges'

// The viewer's whole layout rests on `changes` only ever arriving in these three
// shapes; a backend that starts writing a fourth would otherwise render as an
// empty entry rather than fail.
describe('parseChanges', () => {
	it('classifies each side by whether it holds a value', () => {
		expect(
			parseChanges({
				schedule: { old: '0 0 * * *', new: '0 1 * * *' },
				summary: { new: 'created' },
				on_failure: { old: 'u/admin/handler', new: null },
				description: { old: null, new: 'set now' }
			})
		).toEqual({
			kind: 'fields',
			changes: [
				{ kind: 'added', field: 'description', next: 'set now' },
				{ kind: 'removed', field: 'on_failure', prev: 'u/admin/handler' },
				{ kind: 'changed', field: 'schedule', prev: '0 0 * * *', next: '0 1 * * *' },
				{ kind: 'added', field: 'summary', next: 'created' }
			]
		})
	})

	it('reads the truncation marker and the empty cases', () => {
		expect(parseChanges({ truncated_fields: ['args', 'schedule'] })).toEqual({
			kind: 'truncated',
			fields: ['args', 'schedule']
		})
		expect(parseChanges(null)).toEqual({ kind: 'none' })
		expect(parseChanges({})).toEqual({ kind: 'none' })
	})
})
