import { describe, expect, it } from 'vitest'
import { stagedForkRequest } from './sessionForkDefaults'

const fork = {
	id: 'wm-fork-brave',
	name: 'brave',
	settings: {
		datatables: { main: 'schema_and_data' as const, gone: 'schema_only' as const },
		shared_ducklakes: ['lake', 'old_lake'],
		shared_storages: ['s3', 'old_s3'],
		lock_prod_deploy: true
	}
}
const base = {
	datatables: [
		{ name: 'main' },
		{ name: 'extra' },
		{ name: 'borrowed', database_workspace_id: 'other' }
	],
	ducklakes: ['lake'],
	storages: ['s3'],
	requireIsolation: false
}

describe('stagedForkRequest', () => {
	it('drops saved names the base no longer has', () => {
		const req = stagedForkRequest(fork, base)
		expect(req.forked_datatables).toEqual([
			{ name: 'main', new_dbname: 'wm_fork_brave__main', fork_behavior: 'schema_and_data' }
		])
		expect(req.shared_ducklakes).toEqual(['lake'])
		expect(req.shared_storages).toEqual(['s3'])
	})

	it('keeps saved lake and storage names when the base lists could not be read', () => {
		const req = stagedForkRequest(fork, { ...base, ducklakes: undefined, storages: undefined })
		expect(req.shared_ducklakes).toEqual(['lake', 'old_lake'])
		expect(req.shared_storages).toEqual(['s3', 'old_s3'])
	})

	it('copies every base-owned table and shares nothing under isolation', () => {
		const req = stagedForkRequest(fork, { ...base, requireIsolation: true })
		expect(req.forked_datatables.map((d) => [d.name, d.fork_behavior])).toEqual([
			['main', 'schema_and_data'],
			['extra', 'schema_only']
		])
		expect(req.shared_ducklakes).toEqual([])
		expect(req.shared_storages).toEqual([])
	})

	it('sends the prod locks only for a dev workspace', () => {
		expect(stagedForkRequest(fork, base).lock_prod_deploy).toBe(false)
		const dev = { ...fork, settings: { ...fork.settings, is_dev_workspace: true } }
		expect(stagedForkRequest(dev, base).lock_prod_deploy).toBe(true)
	})
})
