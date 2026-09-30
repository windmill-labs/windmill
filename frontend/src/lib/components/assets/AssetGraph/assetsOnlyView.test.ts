import { describe, expect, it } from 'vitest'
import { assetsOnlyView } from './assetsOnlyView'

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
		expect(v.upstream.get('asset:ducklake:report')).toEqual({ multiple: true })
	})
})
