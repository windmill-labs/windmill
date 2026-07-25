/**
 * `isAppRecording` is what stands between a `?src=` URL anyone can point the
 * public replay page at and a player that indexes into the payload and renders
 * it. Each rejection below is a way a caller-supplied recording could otherwise
 * reach the DOM or the render loop unbounded.
 */
import { describe, expect, it } from 'vitest'
import { isAppRecording } from './rawAppRecordingLoad'
import { MAX_RECORDED_STEPS, MAX_STEP_TEXT_CHARS } from './rawAppSnapshot'

const valid = () => ({
	version: 1,
	type: 'app',
	recorded_at: '2026-07-25T00:00:00.000Z',
	app_path: 'f/demo/app',
	workspace: 'demo',
	total_duration_ms: 1200,
	viewport: { width: 1280, height: 720 },
	frames: ['<html></html>', '<html><body>x</body></html>'],
	steps: [{ t: 100, kind: 'click', label: 'Clicked button', before: 0, after: 1 }]
})

const rejects: [string, (r: ReturnType<typeof valid>) => unknown][] = [
	['a non-object payload', () => 42],
	['another recording type', (r) => ({ ...r, type: 'flow' })],
	['a future version', (r) => ({ ...r, version: 2 })],
	['a frame that is not a string', (r) => ({ ...r, frames: ['<html></html>', 7] })],
	['a step index past the last frame', (r) => ({ ...r, steps: [{ ...r.steps[0], after: 9 }] })],
	['a negative step index', (r) => ({ ...r, steps: [{ ...r.steps[0], before: -1 }] })],
	['an unknown interaction kind', (r) => ({ ...r, steps: [{ ...r.steps[0], kind: 'exec' }] })],
	['a NaN timestamp', (r) => ({ ...r, steps: [{ ...r.steps[0], t: NaN }] })],
	[
		'a label past the text budget',
		(r) => ({ ...r, steps: [{ ...r.steps[0], label: 'x'.repeat(MAX_STEP_TEXT_CHARS + 1) }] })
	],
	[
		'more steps than the recorder can produce',
		(r) => ({
			...r,
			steps: Array.from({ length: MAX_RECORDED_STEPS + 1 }, () => ({ ...r.steps[0] }))
		})
	],
	['a non-numeric viewport', (r) => ({ ...r, viewport: { width: '1280px', height: 720 } })],
	['a viewport beyond any screen', (r) => ({ ...r, viewport: { width: 1e6, height: 720 } })],
	['a negative duration', (r) => ({ ...r, total_duration_ms: -1 })],
	['a missing recorded_at', (r) => ({ ...r, recorded_at: undefined })]
]

describe('isAppRecording', () => {
	it('accepts a well-formed recording', () => {
		expect(isAppRecording(valid())).toBe(true)
		// Steps may legitimately lack a snapshot the recorder had to skip.
		expect(isAppRecording({ ...valid(), steps: [{ t: 1, kind: 'click', label: 'x' }] })).toBe(true)
	})

	it.each(rejects)('rejects %s', (_name, mutate) => {
		expect(isAppRecording(mutate(valid()))).toBe(false)
	})
})
