import { describe, expect, it } from 'vitest'
import {
	buildSchemaContractContext,
	diffSchemaContracts,
	mapWarningsToMarkers,
	normalizeAssetPath,
	referencedDucklakePaths,
	type CapturedSchemaLite
} from './schemaContracts'
import { parsePipelineAnnotations } from './parsePipelineAnnotations'
import type { AssetWithAltAccessType } from '../lib'

// Mirrors backend/windmill-common/src/schema_contracts.rs unit tests — the two
// diffs must apply the same rules or the editor previews a different verdict
// than the save-time check returns.

function schema(cols: [string, string][], version = 2): CapturedSchemaLite {
	return {
		columns: cols.map(([name, type]) => ({ name, type })),
		version,
		capturedAt: '2026-01-01T00:00:00Z'
	}
}

function readAsset(path: string, cols: string[]): AssetWithAltAccessType {
	return {
		path,
		kind: 'ducklake',
		access_type: 'r',
		columns: Object.fromEntries(cols.map((c) => [c, 'r' as const]))
	}
}

const NO_ANN = { columnLineage: [], dataTests: [] }

describe('diffSchemaContracts', () => {
	it('warns on a missing read column, matching case-insensitively', () => {
		const schemas = new Map([
			[
				'lake/orders',
				schema([
					['Order_ID', 'BIGINT'],
					['amount_usd', 'DOUBLE']
				])
			]
		])
		const w = diffSchemaContracts({
			...NO_ANN,
			assets: [readAsset('lake/orders', ['order_id', 'amount'])],
			schemas,
			ignored: new Set()
		})
		expect(w).toHaveLength(1)
		expect(w[0].kind).toBe('missing_column')
		expect(w[0].column).toBe('amount')
		expect(w[0].schema_version).toBe(2)
	})

	it('skips unknown-column assets, "*" and reserved columns', () => {
		const schemas = new Map([['lake/orders', schema([['id', 'BIGINT']])]])
		const noColumns: AssetWithAltAccessType = {
			path: 'lake/orders',
			kind: 'ducklake',
			access_type: 'r'
		}
		expect(
			diffSchemaContracts({ ...NO_ANN, assets: [noColumns], schemas, ignored: new Set() })
		).toEqual([])
		expect(
			diffSchemaContracts({
				...NO_ANN,
				assets: [readAsset('lake/orders', ['*', '_wm_partition', 'id'])],
				schemas,
				ignored: new Set()
			})
		).toEqual([])
	})

	it('is silent for assets without a captured schema', () => {
		expect(
			diffSchemaContracts({
				...NO_ANN,
				assets: [readAsset('lake/unknown', ['whatever'])],
				schemas: new Map(),
				ignored: new Set()
			})
		).toEqual([])
	})

	it('normalizes the {partition} token before lookup', () => {
		const schemas = new Map([['lake/orders', schema([['id', 'BIGINT']])]])
		const w = diffSchemaContracts({
			...NO_ANN,
			assets: [readAsset('lake/orders/{partition}', ['gone'])],
			schemas,
			ignored: new Set()
		})
		expect(w).toHaveLength(1)
		expect(w[0].asset_path).toBe('lake/orders')
	})

	it('warns on broken // column lineage refs', () => {
		const ann = parsePipelineAnnotations(
			'// column total <- ducklake://lake/orders.amount\nSELECT 1;'
		)
		const schemas = new Map([['lake/orders', schema([['id', 'BIGINT']])]])
		const w = diffSchemaContracts({
			assets: [],
			columnLineage: ann.columnLineage,
			dataTests: [],
			schemas,
			ignored: new Set()
		})
		expect(w).toHaveLength(1)
		expect(w[0].kind).toBe('missing_lineage_source')
	})

	it('flags missing relationship columns and captured-type differences', () => {
		const ann = parsePipelineAnnotations(
			'// materialize ducklake://lake/orders\n' +
				'// data_test relationships customer_id -> ducklake://lake/customers.id\n' +
				'// data_test relationships customer_id -> ducklake://lake/customers.uuid\n' +
				'SELECT 1;'
		)
		const schemas = new Map([
			['lake/customers', schema([['id', 'VARCHAR']])],
			['lake/orders', schema([['customer_id', 'BIGINT']])]
		])
		const w = diffSchemaContracts({
			assets: [],
			columnLineage: ann.columnLineage,
			dataTests: ann.dataTests,
			materialize: ann.materialize,
			schemas,
			ignored: new Set()
		})
		expect(w).toHaveLength(2)
		expect(
			w.some(
				(x) =>
					x.kind === 'relationship_type_mismatch' &&
					x.expected_type === 'BIGINT' &&
					x.found_type === 'VARCHAR'
			)
		).toBe(true)
		expect(w.some((x) => x.kind === 'missing_relationship_column' && x.column === 'uuid')).toBe(
			true
		)
	})

	it('suppresses ignored assets down to one informational note', () => {
		const schemas = new Map([['lake/orders', schema([['id', 'BIGINT']])]])
		const ignored = new Set(['lake/orders'])
		const w = diffSchemaContracts({
			...NO_ANN,
			assets: [readAsset('lake/orders', ['a', 'b'])],
			schemas,
			ignored
		})
		expect(w).toHaveLength(1)
		expect(w[0].kind).toBe('suppressed')
		expect(
			diffSchemaContracts({
				...NO_ANN,
				assets: [readAsset('lake/orders', ['id'])],
				schemas,
				ignored
			})
		).toEqual([])
	})
})

describe('referencedDucklakePaths', () => {
	it('collects paths from reads, lineage, relationships and materialize', () => {
		const ann = parsePipelineAnnotations(
			'// materialize ducklake://lake/out\n' +
				'// column total <- ducklake://lake/a.amount\n' +
				'// data_test relationships k -> ducklake://lake/b.id\n' +
				'SELECT 1;'
		)
		const refs = referencedDucklakePaths({
			assets: [readAsset('lake/c/{partition}', ['x'])],
			columnLineage: ann.columnLineage,
			dataTests: ann.dataTests,
			materialize: ann.materialize
		})
		expect(refs.sort()).toEqual(['lake/a', 'lake/b', 'lake/c', 'lake/out'])
	})
})

describe('buildSchemaContractContext', () => {
	it('derives ignored assets and scd2 _current bases from graph runnables', () => {
		const ctx = buildSchemaContractContext([
			{
				materialize_target: { kind: 'ducklake', path: 'lake/dim' },
				materialize_strategy: 'scd2',
				materialize_on_schema_change: 'ignore'
			},
			{
				materialize_target: { kind: 'ducklake', path: 'lake/orders' },
				materialize_strategy: 'replace'
			},
			// non-ducklake and absent targets are ignored
			{ materialize_target: { kind: 's3object', path: 'x/y' }, materialize_strategy: 'scd2' },
			{}
		])
		expect(ctx.ignoredAssets).toEqual(['lake/dim', 'lake/dim_current'])
		expect(ctx.scd2CurrentBases).toEqual({ 'lake/dim_current': 'lake/dim' })
	})

	it('ignores _current only for scd2 producers (backend spec.scd2 gate)', () => {
		const ctx = buildSchemaContractContext([
			{
				materialize_target: { kind: 'ducklake', path: 'lake/t' },
				materialize_strategy: 'replace',
				materialize_on_schema_change: 'ignore'
			}
		])
		// a non-scd2 producer's `<base>_current` is an unrelated asset — it must
		// keep warning, exactly like the server-side check
		expect(ctx.ignoredAssets).toEqual(['lake/t'])
		expect(ctx.scd2CurrentBases).toEqual({})
	})
})

describe('mapWarningsToMarkers', () => {
	it('anchors annotation warnings to their lines and body reads to the identifier', () => {
		const code =
			'-- pipeline\n' +
			'-- on ducklake://lake/orders\n' +
			'-- column total <- ducklake://lake/orders.amount\n' +
			'-- data_test relationships k -> ducklake://lake/customers.uuid\n' +
			'SELECT amount FROM dl.orders;'
		const markers = mapWarningsToMarkers(code, [
			{
				kind: 'missing_lineage_source',
				asset_path: 'lake/orders',
				column: 'amount',
				message: 'm1'
			},
			{
				kind: 'missing_relationship_column',
				asset_path: 'lake/customers',
				column: 'uuid',
				message: 'm2'
			},
			{ kind: 'missing_column', asset_path: 'lake/orders', column: 'amount', message: 'm3' },
			{ kind: 'suppressed', asset_path: 'lake/orders', message: 'hidden' }
		])
		expect(markers).toHaveLength(3)
		expect(markers[0].startLineNumber).toBe(3)
		expect(markers[1].startLineNumber).toBe(4)
		// body-read warning anchors to the first occurrence of the identifier,
		// which is the annotation line mentioning `amount` (line 3)
		expect(markers[2].startLineNumber).toBe(3)
		// token range is tight around the identifier, not the whole line
		const line3 = '-- column total <- ducklake://lake/orders.amount'
		expect(markers[2].startColumn).toBe(line3.indexOf('amount') + 1)
	})

	it('falls back to the line mentioning the asset path', () => {
		const code = '# pipeline\n# on ducklake://lake/orders\nprint(1)'
		const markers = mapWarningsToMarkers(code, [
			{ kind: 'missing_column', asset_path: 'lake/orders', column: 'zzz', message: 'm' }
		])
		expect(markers[0].startLineNumber).toBe(2)
	})
})

describe('normalizeAssetPath', () => {
	it('strips the partition token and trailing slashes', () => {
		expect(normalizeAssetPath('lake/orders/{partition}')).toBe('lake/orders')
		expect(normalizeAssetPath('lake/orders_{partition}')).toBe('lake/orders_')
		expect(normalizeAssetPath('lake/orders/')).toBe('lake/orders')
	})
})
