import { JobService } from './gen'
import type { InferAssetsSqlQueryDetails, PreparedAssetsSqlQuery } from './infer'
import { ChangeOnDeepInequality, MapResource } from './svelte5Utils.svelte'
import { sqlDataTypeToJsTypeHeuristic } from './components/apps/components/display/dbtable/utils'
import { chunkBy, clone, getQueryStmtCountHeuristic } from './utils'

function computeQueryKey(query: InferAssetsSqlQueryDetails, workspace?: string) {
	return `${query.source_kind}::${query.source_name}::${query.source_schema}::${workspace}::${query.query_string}`
}

export function usePreparedAssetSqlQueries(
	_getQueries: () => InferAssetsSqlQueryDetails[] | undefined,
	getWorkspace: () => string | undefined
): { current: InferAssetsSqlQueryDetails[] | undefined } {
	let getQueries = new ChangeOnDeepInequality(_getQueries)

	let map = new MapResource<InferAssetsSqlQueryDetails, PreparedAssetsSqlQuery>(
		() =>
			Object.fromEntries(
				getQueries.value?.map((q) => [computeQueryKey(q, getWorkspace() ?? ''), q]) || []
			),
		async (toFetch) => {
			let queries = Object.entries(clone(toFetch))
			queries = queries.filter(
				([_, q]) => q.source_kind === 'datatable' || q.source_kind === 'ducklake'
			)
			// We only support preparing single-statement queries for now.
			queries = queries.filter(([_, q]) => getQueryStmtCountHeuristic(q.query_string) === 1)

			if (!queries?.length) return {}
			let datatableQueries = queries.filter(([_, q]) => q.source_kind === 'datatable')
			let ducklakeQueries = queries.filter(([_, q]) => q.source_kind === 'ducklake')

			let allResults: [string, PreparedAssetsSqlQuery][] = []

			if (datatableQueries.length) {
				allResults.push(...(await prepareDatatableQueries(datatableQueries, getWorkspace)))
			}
			if (ducklakeQueries.length) {
				allResults.push(...(await prepareDucklakeQueries(ducklakeQueries, getWorkspace)))
			}

			return Object.fromEntries(allResults)
		}
	)

	let extendedQueries = $derived.by(() =>
		getQueries.value?.map((q) => ({
			...q,
			prepared: map.current?.[computeQueryKey(q, getWorkspace())]
		}))
	)
	return {
		get current() {
			return extendedQueries
		}
	}
}

type QueryEntry = [string, InferAssetsSqlQueryDetails]

function mapPrepareResults(
	res: { error?: string; columns?: { name: string; type: string }[] }[],
	chunk: QueryEntry[]
): [string, PreparedAssetsSqlQuery][] {
	return res.map((r, i) => [
		chunk[i][0],
		r.columns
			? {
					columns: Object.fromEntries(
						r.columns.map(({ name, type: t }) => [name, sqlDataTypeToJsTypeHeuristic(t)])
					)
				}
			: { error: r.error ?? "Couldn't prepare query " }
	])
}

async function prepareDatatableQueries(
	queries: QueryEntry[],
	getWorkspace: () => string | undefined
): Promise<[string, PreparedAssetsSqlQuery][]> {
	queries.sort((a, b) => a[1].source_name.localeCompare(b[1].source_name))
	let results = (
		await Promise.all(
			chunkBy(queries, ([_, q]) => q.source_name).map(async (chunk) => {
				let queryContent = chunk
					.flatMap(([_, q]) => [
						q.source_schema ? `SET search_path TO ${q.source_schema};` : 'RESET search_path;',
						q.query_string + (q.query_string.trim().endsWith(';') ? '' : ';')
					])
					.join('\n')
				queryContent = '-- prepare\n--result_collection=all_statements_first_row\n' + queryContent

				let res = (await JobService.runScriptPreviewAndWaitResult({
					workspace: getWorkspace()!,
					requestBody: {
						language: 'postgresql',
						content: queryContent,
						args: { database: `datatable://${chunk[0][1]?.source_name}` }
					}
				})) as { error?: string; columns?: { name: string; type: string }[] }[]

				return mapPrepareResults(res, chunk)
			})
		)
	).flat()
	return results
}

async function prepareDucklakeQueries(
	queries: QueryEntry[],
	getWorkspace: () => string | undefined
): Promise<[string, PreparedAssetsSqlQuery][]> {
	queries.sort((a, b) => a[1].source_name.localeCompare(b[1].source_name))
	let results = (
		await Promise.all(
			chunkBy(queries, ([_, q]) => `${q.source_name}::${q.source_schema ?? ''}`).map(async (chunk) => {
				let sourceName = chunk[0][1].source_name
				let sourceSchema = chunk[0][1].source_schema
				let attachSetup = `ATTACH 'ducklake://${sourceName}' AS dl;\n`
				attachSetup += sourceSchema ? `USE dl.${sourceSchema};\n` : `USE dl;\n`

				let queryContent = chunk
					.map(([_, q]) => q.query_string + (q.query_string.trim().endsWith(';') ? '' : ';'))
					.join('\n')
				queryContent =
					'-- prepare\n--result_collection=all_statements_first_row\n' + attachSetup + queryContent

				try {
					let res = (await JobService.runScriptPreviewAndWaitResult({
						workspace: getWorkspace()!,
						requestBody: {
							language: 'duckdb',
							content: queryContent,
							args: {}
						}
					})) as { error?: string; columns?: { name: string; type: string }[] }[]
					return mapPrepareResults(res, chunk)
				} catch (e) {
					const error = e instanceof Error ? e.message : JSON.stringify(e)
					return chunk.map(([key]) => [key, { error }] as [string, PreparedAssetsSqlQuery])
				}
			})
		)
	).flat()
	return results
}
