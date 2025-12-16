import { describe, it, expect } from 'vitest'
import { injectSqlTypes, create } from './sqlTypePlugin.worker.js'

describe('sqlTypePlugin.worker', () => {
	describe('code transformation', () => {
		it('inserts type annotations on data table queries', () => {
			let { transformed } = injectSqlTypes(testCode, testQueries)
			let expected = `
export async function main() {
  let sql = wmill.datatable('main')
  error_before_first_transform
  let friend = await sql<{ "id": number; "nickname": string }>\`SELECT id, name as nickname FROM friends\`.fetchOne()
  error_after_first_transform
  let friend2 = await sql<{ "name": string }>\`SELECT name FROM friends\`.fetchOne()
  let bait = await not_an_asset\`SELECT 23 FROM friends\`.fetchOne()
  let friend3 = await sql<{ "id": number; "nickname": string }>\`SELECT id, name as nickname FROM friends\`.fetchOne()
  return friend
}
  
error_at_the_end`
			expect(transformed).toBe(expected)
		})
	})

	describe('maps from transformed to original (displayed) position', () => {
		it("Doesn't change when before first transform", () => {
			let tsWorker = createMockTsWorker(testCode, testQueries)
			let pos = tsWorker._mapPositionToOriginal(44, 'test.ts')
			expect(pos).toEqual(44)
		})
		it('Maps correctly after first transform', () => {
			let tsWorker = createMockTsWorker(testCode, testQueries)
			let pos = tsWorker._mapPositionToOriginal(217, 'test.ts')
			expect(pos).toEqual(179)
		})
		it('Maps correctly at the end after multiple transforms', () => {
			let tsWorker = createMockTsWorker(testCode, testQueries)
			let pos = tsWorker._mapPositionToOriginal(533, 'test.ts')
			expect(pos).toEqual(437)
		})
	})

	describe('maps from original (displayed) to transformed position', () => {
		it("Doesn't change when before first transform", () => {
			let tsWorker = createMockTsWorker(testCode, testQueries)
			let pos = tsWorker._mapPositionToTransformed(44, 'test.ts')
			expect(pos).toEqual(44)
		})
		it('Maps correctly after first transform', () => {
			let tsWorker = createMockTsWorker(testCode, testQueries)
			let pos = tsWorker._mapPositionToTransformed(179, 'test.ts')
			expect(pos).toEqual(217)
		})
		it('Maps correctly at the end after multiple transforms', () => {
			let tsWorker = createMockTsWorker(testCode, testQueries)
			let pos = tsWorker._mapPositionToTransformed(437, 'test.ts')
			expect(pos).toEqual(533)
		})
	})
})

let testCode = `
export async function main() {
  let sql = wmill.datatable('main')
  error_before_first_transform
  let friend = await sql\`SELECT id, name as nickname FROM friends\`.fetchOne()
  error_after_first_transform
  let friend2 = await sql\`SELECT name FROM friends\`.fetchOne()
  let bait = await not_an_asset\`SELECT 23 FROM friends\`.fetchOne()
  let friend3 = await sql\`SELECT id, name as nickname FROM friends\`.fetchOne()
  return friend
}
  
error_at_the_end`

let testQueries = [
	{
		query_string: 'SELECT id, name as nickname FROM friends',
		span: [120, 165],
		source_kind: 'datatable',
		source_name: 'main',
		prepared: { columns: { id: 'number', nickname: 'string' } }
	},
	{
		query_string: 'SELECT name FROM friends',
		span: [229, 258],
		source_kind: 'datatable',
		source_name: 'main',
		prepared: { columns: { name: 'string' } }
	},
	{
		query_string: 'SELECT id, name as nickname FROM friends',
		span: [359, 404],
		source_kind: 'datatable',
		source_name: 'main',
		prepared: { columns: { id: 'number', nickname: 'string' } }
	}
]

function createMockTsWorker(code: string, queries: typeof testQueries) {
	let tsWorker = create()
	;(tsWorker as any).setMockCode(code)
	tsWorker.updateSqlQueries('test.ts', queries)
	return tsWorker
}
