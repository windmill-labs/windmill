import { describe, expect, it } from 'vitest'
import {
	MAX_RECORDED_JOBS,
	MAX_SAMPLE_CELLS,
	MAX_SAMPLE_COLUMNS,
	parseRecording
} from './recordingLoad'

const job = (events = 1) => ({
	initial_job: { id: 'j' },
	events: Array.from({ length: events }, (_, t) => ({ t, data: { completed: true } }))
})

const header = { recorded_at: '2026-07-25T00:00:00.000Z', total_duration_ms: 1000 }

const pipeline = (extra: Record<string, unknown> = {}) => ({
	version: 1,
	type: 'pipeline',
	folder: 'orders',
	...header,
	graph: { runnables: [], assets: [], edges: [], triggers: [] },
	timeline: [],
	jobs: {},
	...extra
})

const kindOf = (data: unknown) => {
	const res = parseRecording(data)
	return res.ok ? res.loaded.kind : `error: ${res.error}`
}
const rejected = (data: unknown) => parseRecording(data).ok === false

describe('parseRecording', () => {
	it('routes each kind to its player, defaulting a type-less payload to flow', () => {
		expect(kindOf({ version: 1, flow_path: 'f', ...header, jobs: { j: job() } })).toBe('flow')
		expect(
			kindOf({ version: 1, type: 'flow', flow_path: 'f', ...header, jobs: { j: job() } })
		).toBe('flow')
		expect(
			kindOf({
				version: 1,
				type: 'script',
				script_path: 's',
				...header,
				code: 'echo hi',
				language: 'bash',
				job: job()
			})
		).toBe('script')
		expect(kindOf(pipeline())).toBe('pipeline')
		expect(
			kindOf({
				version: 1,
				type: 'app',
				recorded_at: header.recorded_at,
				total_duration_ms: 0,
				viewport: { width: 800, height: 600 },
				frames: ['<html></html>'],
				steps: [{ t: 0, kind: 'click', label: 'Clicked button', before: 0 }]
			})
		).toBe('app')
	})

	it('rejects payloads a player would choke on rather than falling through to flow', () => {
		// A `?src=` origin is arbitrary: a claimed kind must be validated as that
		// kind, never silently reinterpreted.
		expect(rejected({ version: 1, type: 'script', code: 'x', language: 'bash' })).toBe(true)
		expect(rejected({ version: 1, type: 'pipeline', timeline: [], jobs: {} })).toBe(true)
		expect(rejected({ version: 1, type: 'somethingelse', jobs: {} })).toBe(true)
		expect(rejected({ version: 2, jobs: {} })).toBe(true)
		expect(rejected(null)).toBe(true)
		expect(rejected([{ version: 1 }])).toBe(true)
		// Header fields are required by the types and rendered by every player.
		expect(rejected({ version: 1, jobs: { j: job() } })).toBe(true)
		expect(rejected({ version: 1, flow_path: 'f', recorded_at: 'x', jobs: { j: job() } })).toBe(
			true
		)
		// JobLoader replays each event in a timer, where a throw escapes every
		// boundary, so a non-object event has to be caught at load.
		expect(
			rejected({
				version: 1,
				flow_path: 'f',
				...header,
				jobs: { j: { initial_job: {}, events: ['nope'] } }
			})
		).toBe(true)
		// Cardinality, not just shape: each job mounts a JobLoader.
		const tooManyJobs = Object.fromEntries(
			Array.from({ length: MAX_RECORDED_JOBS + 1 }, (_, i) => [`j${i}`, job(0)])
		)
		expect(rejected({ version: 1, flow_path: 'f', ...header, jobs: tooManyJobs })).toBe(true)
	})

	it('bounds an asset sample on its cell product, not just per axis', () => {
		const sample = (rows: number, columns: number) => ({
			assetSamples: {
				'ducklake:main/t': {
					kind: 'ducklake',
					path: 'main/t',
					uri: 'ducklake://main/t',
					rows: Array.from({ length: rows }, () => ({})),
					columns: Array.from({ length: columns }, (_, i) => ({ field: `c${i}` }))
				}
			}
		})
		// Each axis is individually under its cap, so only the product rejects this.
		const overBudget = MAX_SAMPLE_CELLS / MAX_SAMPLE_COLUMNS + 1
		expect(rejected(pipeline(sample(overBudget, MAX_SAMPLE_COLUMNS)))).toBe(true)
		expect(kindOf(pipeline(sample(10, 20)))).toBe('pipeline')
	})
})
