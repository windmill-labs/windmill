import { HelpersService, OpenAPI } from '$lib/gen'
import { getHeaders } from '$lib/gen/core/request'
import { ColumnIdentity, type ColumnDef } from './apps/components/display/dbtable/utils'
import type { IDbTableOps } from './dbOps'

/** Files the server can page through as a table, without reading them whole. */
export function serverTableFormat(fileKey: string): 'parquet' | 'csv' | undefined {
	const key = fileKey.toLowerCase()
	if (key.endsWith('.parquet')) return 'parquet'
	if (/\.(csv|tsv|jsonl|ndjson)$/.test(key)) return 'csv'
	return undefined
}

export type S3TableSource = {
	workspace: string
	fileKey: string
	storage?: string
	s3ResourcePath?: string
	csvSeparator?: string
	csvHasHeader?: boolean
}

type Chunk = {
	columns: string[]
	column_types?: string[]
	rows: Record<string, unknown>[]
	csv_separator?: string
}

/** An Arrow type as the SQL type name the grid filters and formats by. */
export function datatypeOf(arrowType: string | undefined): string {
	const t = arrowType ?? ''
	if (/^U?Int\d+$/.test(t)) return 'bigint'
	if (/^Float\d+$/.test(t)) return 'double'
	if (/^Decimal/.test(t)) return 'decimal'
	if (t === 'Boolean') return 'boolean'
	if (/^(Large)?Utf8/.test(t)) return 'text'
	// A nested type spells out its fields, e.g. `List(Field { name: "item", … })`.
	return t.split('(')[0].toLowerCase() || 'text'
}

/** Read-only table ops over an object storage file, paged, searched, filtered and sorted
 * by the server, with the CSV separator it read the file with (guessed when none is given).
 */
export async function s3TableOps(
	source: S3TableSource
): Promise<{ ops: IDbTableOps; csvSeparator?: string }> {
	const format = serverTableFormat(source.fileKey)
	if (!format) throw new Error(`${source.fileKey} cannot be read as a table`)
	const common = {
		workspace: source.workspace,
		path: source.fileKey,
		storage: source.storage,
		s3ResourcePath: source.s3ResourcePath
	}
	const csvOptions: { csvSeparator?: string; csvHasHeader?: boolean } =
		format === 'csv' ? { csvSeparator: source.csvSeparator, csvHasHeader: source.csvHasHeader } : {}
	const filtersParam = (columnFilters: Record<string, unknown> | undefined) =>
		columnFilters && Object.keys(columnFilters).length ? JSON.stringify(columnFilters) : undefined
	const searchParam = (quicksearch: string) => quicksearch.trim() || undefined

	async function loadChunk(params: {
		offset: number
		limit: number
		quicksearch?: string
		columnFilters?: Record<string, unknown>
		orderBy?: string
		isDesc?: boolean
	}): Promise<Chunk> {
		const data = {
			...common,
			...csvOptions,
			offset: params.offset,
			limit: params.limit,
			searchTerm: searchParam(params.quicksearch ?? ''),
			filters: filtersParam(params.columnFilters),
			sortCol: params.orderBy,
			sortDesc: params.isDesc
		}
		return (await (format === 'parquet'
			? HelpersService.loadParquetPreview(data)
			: HelpersService.loadCsvPreview(data))) as Chunk
	}

	const head = await loadChunk({ offset: 0, limit: 0 })
	// Later reads reuse the separator the server guessed, rather than each guessing again.
	if (format === 'csv') csvOptions.csvSeparator ??= head.csv_separator
	const colDefs: ColumnDef[] = head.columns.map((field, i) => ({
		field,
		datatype: datatypeOf(head.column_types?.[i]),
		// The server cannot read a struct, map or union as text to filter it.
		filterable: !/^(Struct|Map|Union)/.test(head.column_types?.[i] ?? ""),
		defaultvalue: '',
		isprimarykey: false,
		isidentity: ColumnIdentity.No,
		isnullable: 'YES',
		isenum: false
	}))

	const ops: IDbTableOps = {
		dbType: 'duckdb',
		tableKey: source.fileKey.split('/').pop() ?? source.fileKey,
		colDefs,
		getCount: async ({ quicksearch, columnFilters }) => {
			const res = await HelpersService.loadTableRowCount({
				...common,
				...csvOptions,
				searchTerm: searchParam(quicksearch),
				filters: filtersParam(columnFilters)
			})
			return res.count ?? 0
		},
		getRows: async ({
			offset,
			limit,
			quicksearch,
			columnFilters,
			order_by,
			is_desc,
			explicitSort
		}) =>
			(
				await loadChunk({
					offset,
					limit,
					quicksearch,
					columnFilters,
					// Unsorted, a file keeps its own row order.
					orderBy: explicitSort ? order_by : undefined,
					isDesc: is_desc
				})
			).rows
	}
	return { ops, csvSeparator: head.csv_separator }
}

/** Plain JSON cannot be paged by the server, so only a file this small is read whole into
 * the browser to be shown as a table. */
export const JSON_TABLE_MAX_BYTES = 10 * 1024 * 1024

export function isJsonFile(fileKey: string): boolean {
	return fileKey.toLowerCase().endsWith('.json')
}

/** Rows of a JSON array, an element that is not an object becoming a one-column row;
 * undefined for anything else. */
export function jsonTableRows(value: unknown): Record<string, unknown>[] | undefined {
	if (!Array.isArray(value) || value.length === 0) return undefined
	return value.map((v) =>
		v !== null && typeof v === 'object' && !Array.isArray(v)
			? (v as Record<string, unknown>)
			: { value: v }
	)
}

export async function fetchJsonTableRows(
	source: S3TableSource
): Promise<Record<string, unknown>[] | undefined> {
	const query = new URLSearchParams({ file_key: source.fileKey })
	if (source.storage) query.set('storage', source.storage)
	if (source.s3ResourcePath) query.set('s3_resource_path', source.s3ResourcePath)
	const apiPath = `/w/${source.workspace}/job_helpers/download_s3_file?${query}`
	const headers = await getHeaders(OpenAPI, { method: 'GET', url: apiPath })
	const response = await fetch(`${OpenAPI.BASE}${apiPath}`, {
		headers,
		credentials: OpenAPI.CREDENTIALS
	})
	if (!response.ok) throw new Error(`${response.status} ${response.statusText}`)
	return jsonTableRows(await response.json())
}
