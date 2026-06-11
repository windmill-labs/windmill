import { describe, expect, it } from 'vitest'

import {
	formatRuntimeLogsForChat,
	genWmillTs,
	normalizeRawAppRuntimeLogs,
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
