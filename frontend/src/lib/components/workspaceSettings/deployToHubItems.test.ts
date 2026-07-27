import { describe, it, expect } from 'vitest'
import { canRecordSession, mergeAppTableOrigin, type DeployItem } from './deployToHubItems'

function item(over: Partial<DeployItem> & Pick<DeployItem, 'key' | 'path' | 'kind'>): DeployItem {
	return { rec: 'none', ...over }
}

describe('canRecordSession', () => {
	it('offers the recorder only for app-table raw apps', () => {
		expect(
			canRecordSession(item({ key: 'raw_app:f/r', path: 'f/r', kind: 'raw_app', appTable: true }))
		).toBe(true)
		// Legacy entries live in the `raw_app` table, which the record surface's
		// AppService loader cannot see: offering the action opens a dead drawer.
		expect(canRecordSession(item({ key: 'raw_app:f/r', path: 'f/r', kind: 'raw_app' }))).toBe(false)
		expect(canRecordSession(item({ key: 'app:f/a', path: 'f/a', kind: 'app' }))).toBe(false)
	})
})

describe('mergeAppTableOrigin', () => {
	it('restores the origin so a reopened draft stays recordable', () => {
		const drafts = [item({ key: 'raw_app:f/r', path: 'f/r', kind: 'raw_app' })]
		const workspace = [item({ key: 'raw_app:f/r', path: 'f/r', kind: 'raw_app', appTable: true })]
		expect(canRecordSession(mergeAppTableOrigin(drafts, workspace)[0])).toBe(true)
	})
	it('keeps the reference when nothing changes, and ignores unmatched drafts', () => {
		const drafts = [item({ key: 'flow:f/f', path: 'f/f', kind: 'flow' })]
		expect(mergeAppTableOrigin(drafts, drafts)).toBe(drafts)
		const orphan = [item({ key: 'raw_app:f/gone', path: 'f/gone', kind: 'raw_app' })]
		expect(
			mergeAppTableOrigin(orphan, [
				item({ key: 'raw_app:f/r', path: 'f/r', kind: 'raw_app', appTable: true })
			])
		).toBe(orphan)
	})
})
