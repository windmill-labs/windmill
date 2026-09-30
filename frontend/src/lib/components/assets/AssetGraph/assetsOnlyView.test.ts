import { describe, expect, it } from 'vitest'
import { assetsOnlyView, upstreamDeletion, type AssetUpstream } from './assetsOnlyView'

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
