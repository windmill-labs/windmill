import { describe, expect, it } from 'vitest'
import { MAX_RECORDED_JOBS, parseRecording } from './recordingLoad'

const job = (events = 1) => ({
	initial_job: { id: 'j' },
	events: Array.from({ length: events }, (_, t) => ({ t, data: { completed: true } }))
})

describe('parseRecording', () => {
	it('routes each kind to its player, defaulting a type-less payload to flow', () => {
		const kindOf = (data: unknown) => {
			const res = parseRecording(data)
			return res.ok ? res.loaded.kind : `error: ${res.error}`
		}
		expect(kindOf({ version: 1, jobs: { j: job() } })).toBe('flow')
		expect(kindOf({ version: 1, type: 'flow', jobs: { j: job() } })).toBe('flow')
		expect(
			kindOf({ version: 1, type: 'script', code: 'echo hi', language: 'bash', job: job() })
		).toBe('script')
		expect(
			kindOf({
				version: 1,
				type: 'pipeline',
				graph: { runnables: [], assets: [], edges: [], triggers: [] },
				timeline: [],
				jobs: {}
			})
		).toBe('pipeline')
		expect(
			kindOf({
				version: 1,
				type: 'app',
				recorded_at: 'now',
				total_duration_ms: 0,
				viewport: { width: 800, height: 600 },
				frames: ['<html></html>'],
				steps: [{ t: 0, kind: 'click', label: 'Clicked button', before: 0 }]
			})
		).toBe('app')
	})

	it('rejects payloads a player would choke on rather than falling through to flow', () => {
		const rejected = (data: unknown) => parseRecording(data).ok === false
		// A `?src=` origin is arbitrary: a claimed kind must be validated as that
		// kind, never silently reinterpreted.
		expect(rejected({ version: 1, type: 'script', code: 'x', language: 'bash' })).toBe(true)
		expect(rejected({ version: 1, type: 'pipeline', timeline: [], jobs: {} })).toBe(true)
		expect(rejected({ version: 1, type: 'somethingelse', jobs: {} })).toBe(true)
		expect(rejected({ version: 2, jobs: {} })).toBe(true)
		expect(rejected(null)).toBe(true)
		expect(rejected([{ version: 1 }])).toBe(true)
		// JobLoader replays each event in a timer, where a throw escapes every
		// boundary, so a non-object event has to be caught at load.
		expect(rejected({ version: 1, jobs: { j: { initial_job: {}, events: ['nope'] } } })).toBe(true)
		// Cardinality, not just shape: each job mounts a JobLoader.
		const tooManyJobs = Object.fromEntries(
			Array.from({ length: MAX_RECORDED_JOBS + 1 }, (_, i) => [`j${i}`, job(0)])
		)
		expect(rejected({ version: 1, jobs: tooManyJobs })).toBe(true)
	})
})
