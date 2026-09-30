import { describe, expect, it } from 'vitest'
import {
	assetsOnlyView,
	upstreamDeletion,
	NO_ASSET_NODE_ID,
	withoutPassiveReads,
	type AssetUpstream
} from './assetsOnlyView'

const node = (id: string, type: string, data: any = {}) => ({ id, type, data })
const edge = (source: string, target: string, kind: string) => ({
	id: `${kind}:${source}->${target}`,
	source,
	target,
	kind
})

describe('assetsOnlyView', () => {
	const nodes = [
		node('asset:ducklake:raw', 'asset'),
		node('asset:ducklake:clean', 'asset'),
		node('asset:ducklake:report', 'asset'),
		node('script:f/p/clean', 'runnable'),
		node('script:f/p/report', 'runnable'),
		node('script:f/p/other', 'runnable'),
		node('trigger:schedule:f/p/daily', 'trigger', { kind: 'schedule', schedule: '0 0 4 * * *' })
	]
	const edges = [
		edge('trigger:schedule:f/p/daily', 'script:f/p/clean', 'trigger-native'),
		edge('asset:ducklake:raw', 'script:f/p/clean', 'lineage-read'),
		edge('script:f/p/clean', 'asset:ducklake:clean', 'lineage-write'),
		// read-modify-write: no self edge
		edge('asset:ducklake:clean', 'script:f/p/clean', 'lineage-read'),
		edge('asset:ducklake:clean', 'script:f/p/report', 'trigger-asset'),
		edge('asset:ducklake:clean', 'script:f/p/report', 'lineage-read'),
		edge('script:f/p/report', 'asset:ducklake:report', 'lineage-write'),
		edge('script:f/p/other', 'asset:ducklake:report', 'lineage-write')
	]
	const v = assetsOnlyView(nodes, edges)

	it('joins each script input to its outputs, once', () => {
		expect(v.edges.map((e) => `${e.source} -> ${e.target}`).sort()).toEqual([
			'asset:ducklake:clean -> asset:ducklake:report',
			'asset:ducklake:raw -> asset:ducklake:clean'
		])
		expect([...v.nodeIds].sort()).toEqual([
			'asset:ducklake:clean',
			'asset:ducklake:raw',
			'asset:ducklake:report'
		])
	})

	it('names the single producer and its trigger, or marks the asset ambiguous', () => {
		expect(v.upstream.get('asset:ducklake:raw')).toBeUndefined()
		expect(v.upstream.get('asset:ducklake:clean')).toMatchObject({
			multiple: false,
			runnableId: 'script:f/p/clean',
			trigger: { kind: 'schedule', data: { schedule: '0 0 4 * * *' } }
		})
		expect(v.upstream.get('asset:ducklake:report')).toEqual({
			multiple: true,
			runnableIds: ['script:f/p/report', 'script:f/p/other']
		})
	})
})

describe('the No asset node', () => {
	it('gathers scripts that build nothing, but not dbt projects or macro libraries', () => {
		const v = assetsOnlyView(
			[
				node('asset:ducklake:raw', 'asset'),
				node('script:f/p/notify', 'runnable'),
				node('script:f/p/dbt', 'runnable', { dbt: { model_count: 3 } }),
				node('script:f/p/macros', 'runnable', { macros: [{ name: 'm' }] })
			],
			[edge('asset:ducklake:raw', 'script:f/p/notify', 'lineage-read')]
		)
		expect(v.hasNoAssetNode).toBe(true)
		expect(v.upstream.get(NO_ASSET_NODE_ID)).toMatchObject({ runnableId: 'script:f/p/notify' })
		expect(v.edges.map((e) => `${e.source} -> ${e.target}`)).toEqual([
			`asset:ducklake:raw -> ${NO_ASSET_NODE_ID}`
		])
	})

	it('is absent when every script builds an asset', () => {
		const v = assetsOnlyView(
			[node('asset:ducklake:a', 'asset'), node('script:f/p/a', 'runnable')],
			[edge('script:f/p/a', 'asset:ducklake:a', 'lineage-write')]
		)
		expect(v.hasNoAssetNode).toBe(false)
	})
})

describe('upstreamDeletion', () => {
	const script = { runnable_kind: 'script', path: 'f/p/a' }
	const schedule = (targets: string[]) => ({
		kind: 'schedule',
		nodeId: 'trigger:schedule:f/p/a_schedule',
		data: { ref: 'f/p/a_schedule', runnable_paths: targets }
	})
	const own = (trigger?: any): AssetUpstream & { multiple: false } => ({
		multiple: false,
		runnableId: 'script:f/p/a',
		trigger
	})

	it('deletes the script and a trigger that fires only it', () => {
		const u = own(schedule(['f/p/a']))
		expect(upstreamDeletion(u, script, new Map([['asset:x', u]]))).toEqual({
			script: { path: 'f/p/a', unsaved: false },
			trigger: { kind: 'schedule', path: 'f/p/a_schedule', draft: false }
		})
	})

	it('keeps a trigger that fires other scripts', () => {
		const u = own(schedule(['f/p/a', 'f/p/b']))
		expect(upstreamDeletion(u, script, new Map([['asset:x', u]]))?.trigger).toBeUndefined()
	})

	it('deletes nothing when the script also co-produces another asset', () => {
		const u = own(schedule(['f/p/a']))
		const all = new Map<string, AssetUpstream>([
			['asset:x', u],
			['asset:y', { multiple: true, runnableIds: ['script:f/p/a', 'script:f/p/b'] }]
		])
		expect(upstreamDeletion(u, script, all)).toBeUndefined()
	})

	it('deletes nothing when the script builds another asset too', () => {
		const u = own(schedule(['f/p/a']))
		const all = new Map([
			['asset:x', u],
			['asset:y', u]
		])
		expect(upstreamDeletion(u, script, all)).toBeUndefined()
	})
})

describe('withoutPassiveReads', () => {
	it('drops reads that trigger nothing, and the assets only they drew', () => {
		const nodes = [
			node('asset:src', 'asset'),
			node('asset:lookup', 'asset'),
			node('asset:out', 'asset'),
			node('script:a', 'runnable')
		]
		const edges = [
			edge('asset:src', 'script:a', 'lineage-read'),
			edge('asset:src', 'script:a', 'trigger-asset'),
			edge('asset:lookup', 'script:a', 'lineage-read'),
			edge('asset:out', 'script:a', 'lineage-read'),
			edge('script:a', 'asset:out', 'lineage-write')
		]
		const r = withoutPassiveReads(nodes, edges)
		expect(r.nodes.map((n) => n.id)).toEqual(['asset:src', 'asset:out', 'script:a'])
		expect(r.edges.map((e) => e.id)).toEqual([
			'lineage-read:asset:src->script:a',
			'trigger-asset:asset:src->script:a',
			'lineage-write:script:a->asset:out'
		])
	})
})
