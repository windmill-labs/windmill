import type { AssetKind } from '$lib/gen'
import { parseDbInputFromAssetSyntax } from '$lib/utils'
import { loadAllTablesMetaData } from '$lib/components/apps/components/display/dbtable/metadata'
import { dbTableOpsWithPreviewScripts } from '$lib/components/dbOps'
import type { PipelineAssetSample } from './types'

// How many rows to sample per asset — a preview, not a dump.
const SAMPLE_LIMIT = 100

/**
 * Capture a data-sample of a pipeline asset for the recorder, reusing the exact
 * same query path the live asset-preview panes use (`loadAllTablesMetaData` +
 * `dbTableOpsWithPreviewScripts.getRows`), so a replayed sample matches what the
 * pane would have shown. Only ducklake / datatable assets are sampleable this
 * way (s3object files use a different preview); other kinds return an error
 * marker the player renders as "no sample".
 *
 * Never throws — a failed capture (missing table, unconfigured datatable) is
 * returned as a `PipelineAssetSample` with `error` set so the recording still
 * completes.
 */
export async function capturePipelineAssetSample(
	workspace: string,
	kind: AssetKind,
	path: string
): Promise<PipelineAssetSample> {
	const uri = `${kind}://${path}`
	const base: PipelineAssetSample = { kind, path, uri, columns: [], rows: [] }
	if (kind !== 'ducklake' && kind !== 'datatable') {
		return { ...base, error: `no sample for ${kind} assets` }
	}
	try {
		const input = parseDbInputFromAssetSyntax(uri)
		if (!input) return { ...base, error: 'could not parse asset uri' }
		const table = 'specificTable' in input ? (input.specificTable as string | undefined) : undefined
		const schema =
			'specificSchema' in input ? (input.specificSchema as string | undefined) : undefined
		if (!table) return { ...base, error: 'asset uri has no table' }

		const defs = await loadAllTablesMetaData(workspace, input)
		if (!defs) return { ...base, error: 'table metadata unavailable' }

		// Same table-key resolution as the ducklake/datatable preview panes:
		// try `schema.table` then bare `table`, then any key ending in `.table`.
		const defaultSchema = kind === 'ducklake' ? 'main' : 'public'
		const colDefs =
			defs[`${schema ?? defaultSchema}.${table}`] ??
			defs[table] ??
			(() => {
				const key = Object.keys(defs).find((k) => k === table || k.endsWith(`.${table}`))
				return key ? defs[key] : undefined
			})()
		if (!colDefs) return { ...base, error: 'table does not exist yet' }

		const tableKey = schema && table ? `${schema}.${table}` : table
		const ops = dbTableOpsWithPreviewScripts({ input, tableKey, colDefs, workspace })
		const rows = await ops.getRows({
			offset: 0,
			limit: SAMPLE_LIMIT,
			quicksearch: '',
			order_by: '',
			is_desc: false
		})
		let rowCount: number | undefined
		try {
			rowCount = await ops.getCount({ quicksearch: '' })
		} catch {
			// count is best-effort — the sample rows are the important part
		}
		const columns = colDefs
			.filter((c) => c.field)
			.map((c) => ({ field: c.field as string, datatype: (c as any).datatype }))
		return { ...base, columns, rows: rows.slice(0, SAMPLE_LIMIT), rowCount }
	} catch (e) {
		return { ...base, error: e instanceof Error ? e.message : String(e) }
	}
}
