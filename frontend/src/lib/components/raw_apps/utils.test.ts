import { describe, expect, it } from 'vitest'

import {
	canonicalRawAppDiffValue,
	formatRuntimeLogsForChat,
	genWmillTs,
	normalizeRawAppRuntimeLogs,
	stripRawAppDiffNoise,
	type Runnable
} from './utils'

const flowSchema = {
	$schema: 'https://json-schema.org/draft/2020-12/schema',
	type: 'object',
	required: ['string_input'],
	properties: {
		string_input: {
			type: 'string',
			default: ''
		},
		count: {
			type: 'integer'
		}
	}
} as const

describe('genWmillTs', () => {
	it('generates the correct flow args type for path runnables', () => {
		const runnables: Record<string, Runnable> = {
			myflow: {
				type: 'path',
				runType: 'flow',
				path: 'u/dev/my_flow',
				name: 'My flow',
				schema: flowSchema,
				fields: {
					count: {
						type: 'static',
						value: 1,
						fieldType: 'number'
					}
				}
			}
		}

		const dts = genWmillTs(runnables)

		expect(dts).toContain('myflow: (args: { string_input: string }) => Promise<any>;')
	})
})

describe('normalizeRawAppRuntimeLogs', () => {
	it('keeps only well-formed runtime log entries', () => {
		const entries = normalizeRawAppRuntimeLogs([
			{ level: 'log', message: 'ready', ts: 1718000000000 },
			{ level: 'trace', message: 'unsupported level', ts: 1718000000000 },
			{ level: 'error', message: 'bad date', ts: Number.MAX_VALUE },
			{ level: 'warn', message: 123, ts: 1718000000000 },
			'not an entry'
		])

		expect(entries).toEqual([{ level: 'log', message: 'ready', ts: 1718000000000 }])
		expect(formatRuntimeLogsForChat(entries)).toBe('[06:13:20.000] LOG: ready')
	})
})

// A deployed raw-app row as returned by getAppByPath: nested `value`, plus the
// server-managed columns and a recomputed inline-script lock.
function deployedRow() {
	return {
		id: 42,
		raw_app: true,
		is_draft: false,
		created_at: '2024-01-01',
		created_by: 'admin',
		versions: [1, 2],
		extra_perms: { 'u/admin': true },
		summary: 'app',
		path: 'u/admin/app',
		policy: { execution_mode: 'publisher' },
		value: {
			files: { '/App.tsx': 'export default 1' },
			runnables: {
				a: { type: 'inline', inlineScript: { content: 'main()', language: 'bun', lock: 'deps\n' } }
			}
		}
	}
}

describe('stripRawAppDiffNoise', () => {
	it('drops server-managed columns, nulls inline locks and canonicalizes data', () => {
		const cleaned = stripRawAppDiffNoise(deployedRow())

		for (const key of [
			'raw_app',
			'id',
			'created_at',
			'created_by',
			'versions',
			'extra_perms',
			'is_draft'
		]) {
			expect(cleaned).not.toHaveProperty(key)
		}
		expect(cleaned.value.runnables.a.inlineScript.lock).toBeUndefined()
		// absent `data` is canonicalized to the default empty shape
		expect(cleaned.value.data).toEqual({ tables: [], datatable: undefined, schema: undefined })
	})

	it('does not mutate the input (live editor state)', () => {
		const input = deployedRow()
		stripRawAppDiffNoise(input)
		expect(input.raw_app).toBe(true)
		expect(input.value.runnables.a.inlineScript.lock).toBe('deps\n')
	})

	it('handles the flat editor/draft shape (files/runnables top-level)', () => {
		const flat = {
			summary: 'app',
			files: { '/App.tsx': 'x' },
			runnables: {
				a: { type: 'inline', inlineScript: { content: 'm()', language: 'bun', lock: 'l' } }
			}
		}
		const cleaned = stripRawAppDiffNoise(flat)
		expect(cleaned.runnables.a.inlineScript.lock).toBeUndefined()
		expect(cleaned.data).toEqual({ tables: [], datatable: undefined, schema: undefined })
	})
})

describe('canonicalRawAppDiffValue', () => {
	it('collapses a nested deployed row and a flat draft to an identical value when content matches', () => {
		const deployed = deployedRow()
		// The flat draft shape a raw app autosaves: top-level files/runnables/data,
		// no server columns, lock cleared on edit.
		const draft = {
			summary: 'app',
			files: { '/App.tsx': 'export default 1' },
			runnables: { a: { type: 'inline', inlineScript: { content: 'main()', language: 'bun' } } },
			data: { tables: [] },
			policy: { execution_mode: 'publisher' }
		}

		expect(canonicalRawAppDiffValue(deployed)).toEqual(canonicalRawAppDiffValue(draft))
	})

	it('still surfaces a real change (summary edit)', () => {
		const deployed = deployedRow()
		const draft = {
			summary: 'app EDITED',
			files: { '/App.tsx': 'export default 1' },
			runnables: { a: { type: 'inline', inlineScript: { content: 'main()', language: 'bun' } } },
			data: { tables: [] }
		}

		const a = canonicalRawAppDiffValue(deployed)
		const b = canonicalRawAppDiffValue(draft)
		expect(a).not.toEqual(b)
		expect(a.summary).toBe('app')
		expect(b.summary).toBe('app EDITED')
	})
})
