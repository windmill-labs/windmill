import { describe, it, expect } from 'vitest'
import {
	canRecordSession,
	inputResourceTypes,
	mergeAppTableOrigin,
	type DeployItem
} from './deployToHubItems'

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

describe('inputResourceTypes', () => {
	const schema = {
		properties: {
			db: { format: 'resource-postgresql' },
			file: { format: 'resource-s3_object' },
			typo: { format: 'resource-postgres' },
			theme: { format: 'resource-app_theme' },
			name: { format: 'email' }
		}
	}
	it('keeps only formats the workspace declares as a resource type', () => {
		expect(inputResourceTypes(schema, new Set(['postgresql', 'stripe']))).toEqual(['postgresql'])
	})
	// Undefined (still loading) and empty (a workspace that never synced the Hub's
	// types) are both "no catalog" — validating would drop every legitimate type.
	// `s3_object` is never a resource type, so it stays out even here.
	it('falls back to every non-hidden format without a type catalog', () => {
		const all = ['postgresql', 'postgres']
		expect(inputResourceTypes(schema, undefined)).toEqual(all)
		expect(inputResourceTypes(schema, new Set())).toEqual(all)
	})
})
