import { describe, expect, it } from 'vitest'
import {
	createPanelVisitTracker,
	createSettingsChangeTracker,
	dwellBucket,
	KEY_CHARSET,
	placementKey
} from './flowEditorTelemetry'
import type { StepSettingView } from './flowStepSettings'

function view(key: string, configured: boolean, state: 'configured' | 'default' | 'invalid') {
	return { key, configured, summary: { text: '', state } } as StepSettingView
}

// The producer is TypeScript and the validator is Rust, so nothing but this connects them:
// `log_feature_usage` skips a key outside its charset and still answers 204, which means a
// rejected key is invisible in the browser and shows up only as a hole in the table.
describe('emitted keys stay inside the backend charset', () => {
	const PREFERENCES = ['auto', 'docked', 'modal'] as const
	const MODES = ['docked', 'modal'] as const

	it.each([0, 4_999, 5_000, 29_999, 30_000, 119_999, 120_000, 10 * 60_000])(
		'dwell bucket for %ims',
		(ms) => {
			for (const mode of MODES) {
				expect(`${mode}:${dwellBucket(ms)}`).toMatch(KEY_CHARSET)
			}
		}
	)

	it('accepts every placement key', () => {
		for (const preference of PREFERENCES) {
			for (const mode of MODES) {
				expect(placementKey(preference, mode)).toMatch(KEY_CHARSET)
			}
		}
	})
})

describe('createPanelVisitTracker', () => {
	it('logs one open per visit and pairs it with the dwell it ended in', () => {
		const events: [string, string][] = []
		let now = 0
		const tracker = createPanelVisitTracker(
			(kind, key) => events.push([kind, key]),
			() => now
		)

		tracker.visit('a', 'docked')
		// A re-render of the same visit must not read as the panel having been reopened.
		tracker.visit('a', 'docked')
		now = 10_000
		tracker.visit('b', 'docked')
		now = 10_500
		tracker.end()

		expect(events).toEqual([
			['panel_open', 'docked'],
			['panel_dwell', 'docked:5-30s'],
			['panel_open', 'docked'],
			['panel_dwell', 'docked:0-5s']
		])
	})

	it('ends the visit when the same step moves to another placement', () => {
		const events: [string, string][] = []
		const tracker = createPanelVisitTracker(
			(kind, key) => events.push([kind, key]),
			() => 0
		)

		tracker.visit('a', 'docked')
		tracker.visit('a', 'modal')

		expect(events).toEqual([
			['panel_open', 'docked'],
			['panel_dwell', 'docked:0-5s'],
			['panel_open', 'modal']
		])
	})
})

describe('createSettingsChangeTracker', () => {
	it('treats another step as a new baseline rather than as changes', () => {
		const events: [string, string][] = []
		const tracker = createSettingsChangeTracker((kind, key) => events.push([kind, key]))

		tracker.observe('a', [view('retries', false, 'default')])
		tracker.observe('a', [view('retries', true, 'configured')])
		// Selecting a step that already has settings on is not the user switching them on.
		tracker.observe('b', [view('retries', true, 'configured'), view('sleep', true, 'invalid')])
		tracker.observe('b', [view('retries', false, 'default'), view('sleep', true, 'invalid')])

		expect(events).toEqual([
			['setting', 'retries:on'],
			['setting', 'retries:off']
		])
	})

	it('logs a setting that becomes invalid once, not on every re-observation', () => {
		const events: [string, string][] = []
		const tracker = createSettingsChangeTracker((kind, key) => events.push([kind, key]))

		tracker.observe('a', [view('early-stop', true, 'configured')])
		tracker.observe('a', [view('early-stop', true, 'invalid')])
		tracker.observe('a', [view('early-stop', true, 'invalid')])

		expect(events).toEqual([['setting_invalid', 'early-stop']])
	})
})
