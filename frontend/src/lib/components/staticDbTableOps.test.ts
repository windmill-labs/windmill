import { describe, expect, it } from 'vitest'
import { staticDbTableOps } from './staticDbTableOps'

const rows = [
	{ id: 3, name: 'carol', active: true },
	{ id: 1, name: null, active: false },
	{ id: 2, name: 'alice', active: true }
]
const ops = staticDbTableOps(rows, 'postgresql', 'query_result')
const read = (params: Partial<Parameters<typeof ops.getRows>[0]>) =>
	ops.getRows({ offset: 0, limit: 10, quicksearch: '', order_by: 'id', is_desc: false, ...params })

describe('staticDbTableOps', () => {
	it('types a column from its values', () => {
		expect(ops.colDefs.map((c) => c.datatype)).toEqual(['numeric', 'text', 'boolean'])
	})

	it('keeps the query order until a sort is picked, then puts nulls last', async () => {
		expect((await read({})).map((r: any) => r.id)).toEqual([3, 1, 2])
		expect((await read({ order_by: 'name', explicitSort: true })).map((r: any) => r.id)).toEqual([
			2, 3, 1
		])
	})

	it('filters and counts like the SQL reads do', async () => {
		expect(await read({ columnFilters: { id: '>=2', active: true } })).toHaveLength(2)
		expect(await ops.getCount({ quicksearch: 'ALI' })).toBe(1)
	})
})
