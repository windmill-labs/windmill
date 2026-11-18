import { describe, expect, it } from 'vitest'
import { describeEdge } from './edgeDescription'

const names: Record<string, string> = {
	'asset:ducklake:main/raw': 'raw',
	'asset:ducklake:main/clean': 'clean',
	'script:f/p/clean': 'Clean',
	'trigger:schedule:f/p/daily': 'Daily'
}
const nameOf = (id: string) => names[id] ?? id
const dataOf = (id: string) =>
	id === 'trigger:schedule:f/p/daily'
		? { kind: 'schedule', schedule: '0 0 12 * * *', timezone: 'UTC' }
		: undefined

describe('describeEdge', () => {
	it('names an asset trigger by what runs it: any write, changed or not', () => {
		expect(
			describeEdge(
				{ source: 'asset:ducklake:main/raw', target: 'script:f/p/clean', kind: 'trigger-asset' },
				nameOf,
				dataOf
			)
		).toEqual({ subject: 'Clean', phrase: 'runs after each write to', object: 'raw' })
	})

	it("gives a schedule's cadence", () => {
		expect(
			describeEdge(
				{ source: 'trigger:schedule:f/p/daily', target: 'script:f/p/clean', kind: 'trigger-native' },
				nameOf,
				dataOf
			)?.phrase
		).toBe('runs on schedule: Every day at 12:00 UTC')
	})

	it('names the folded script on an assets-only edge', () => {
		expect(
			describeEdge(
				{
					source: 'asset:ducklake:main/raw',
					target: 'asset:ducklake:main/clean',
					kind: 'asset-flow',
					via: ['script:f/p/clean']
				},
				nameOf,
				dataOf
			)
		).toEqual({ subject: 'clean', phrase: 'is built from', object: 'raw', note: 'By Clean. A write to raw does not rerun it.' })
	})
})
