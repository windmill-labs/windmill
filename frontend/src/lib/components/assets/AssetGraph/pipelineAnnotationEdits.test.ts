import { describe, expect, it } from 'vitest'
import {
	addTriggerDirective,
	listenToAsset,
	removeTriggerDirective,
	stopListeningToAsset
} from './pipelineAnnotationEdits'

const table = { kind: 'datatable' as const, path: 'main/orders' }
const lake = { kind: 'ducklake' as const, path: 'main/orders' }

describe('pipeline annotation edits', () => {
	it('adds `on` at the end of the header, in its comment style, leaving the body', () => {
		const sql = '-- pipeline\n-- materialize ducklake://main/x\nSELECT 1; -- on nothing'
		expect(listenToAsset(sql, table, false)).toBe(
			'-- pipeline\n-- materialize ducklake://main/x\n-- on datatable://main/orders\nSELECT 1; -- on nothing'
		)
	})

	it('adds `on` next to the existing `on` lines', () => {
		const sql = '-- pipeline\n-- on schedule\n-- materialize ducklake://main/x\nSELECT 1;'
		expect(listenToAsset(sql, table, false)).toBe(
			'-- pipeline\n-- on schedule\n-- on datatable://main/orders\n-- materialize ducklake://main/x\nSELECT 1;'
		)
	})

	it('removes an `on` line however it is spelled, options included', () => {
		const py = '# pipeline\n# on datatable://main/orders debounce=60s\nprint(1)'
		expect(stopListeningToAsset(py, table, true)).toBe('# pipeline\nprint(1)')
	})

	it('unmutes an auto-triggering read rather than adding `on`, and mutes it to stop', () => {
		const muted = '// pipeline\n// mute ducklake://main/orders\nexport function main() {}'
		expect(listenToAsset(muted, lake, true)).toBe('// pipeline\nexport function main() {}')
		expect(stopListeningToAsset('// pipeline\nexport function main() {}', lake, true)).toBe(
			'// pipeline\n// mute ducklake://main/orders\nexport function main() {}'
		)
	})

	it('adds `on` for an auto-triggering kind the script does not read', () => {
		expect(listenToAsset('-- pipeline\nSELECT 1;', lake, false)).toBe(
			'-- pipeline\n-- on ducklake://main/orders\nSELECT 1;'
		)
	})

	it('mutes only an auto-triggering asset the script reads', () => {
		const sub = '-- pipeline\n-- on ducklake://main/orders\nSELECT 1;'
		expect(stopListeningToAsset(sub, lake, false)).toBe('-- pipeline\nSELECT 1;')
	})

	it('adds `on` for an auto-triggering kind when every read is muted', () => {
		const all = '// pipeline\n// mute all\nexport function main() {}'
		expect(listenToAsset(all, lake, true)).toBe(
			'// pipeline\n// mute all\n// on ducklake://main/orders\nexport function main() {}'
		)
	})

	it('adds a trigger kind once, and drops it', () => {
		expect(addTriggerDirective('// pipeline\n// on webhook\nx()', 'webhook')).toBe(
			'// pipeline\n// on webhook\nx()'
		)
		expect(removeTriggerDirective('// pipeline\n// on kafka\nx()', 'kafka')).toBe(
			'// pipeline\nx()'
		)
	})
})
