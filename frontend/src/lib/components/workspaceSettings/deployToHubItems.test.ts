import { describe, it, expect } from 'vitest'
import {
	canRecordSession,
	inputResourceTypes,
	mergeAppTableOrigin,
	projectResourceExports,
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

describe('projectResourceExports', () => {
	// An app that pins `$res:f/mine/prod_db` for a script arg declared as
	// `resource-postgresql` used to publish both `prod_db` and `postgresql`, so the
	// importer filled two credentials to satisfy one dependency.
	it('ships an input-derived stub unrequired when a $res: one covers the type', () => {
		expect(
			projectResourceExports(
				[{ newPath: 'f/proj/prod_db', resource_type: 'postgresql' }],
				['postgresql'],
				'proj'
			)
		).toEqual([
			{ path: 'f/proj/prod_db', resource_type: 'postgresql', required: true },
			{ path: 'f/proj/postgresql', resource_type: 'postgresql', required: false }
		])
	})

	// Unrequired even when it is the project's only resource: an input format names
	// no path, so there is nothing to say the item will ever be run. A project whose
	// resources are all input-derived asks the importer for nothing, and the stubs
	// are what a standalone run picks from.
	it('ships an input-derived stub unrequired even when nothing else covers the type', () => {
		expect(projectResourceExports([], ['postgresql'], 'proj')).toEqual([
			{ path: 'f/proj/postgresql', resource_type: 'postgresql', required: false }
		])
	})

	// A referenced resource named after its own type relocates onto the conventional
	// stub path. Something reads it, so the reference has to win the collision.
	it('lets a referenced resource win a path claimed by both', () => {
		expect(
			projectResourceExports(
				[{ newPath: 'f/proj/postgresql', resource_type: 'postgresql' }],
				['postgresql'],
				'proj'
			)
		).toEqual([{ path: 'f/proj/postgresql', resource_type: 'postgresql', required: true }])
	})
})
