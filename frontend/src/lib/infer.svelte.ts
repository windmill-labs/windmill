import { get } from 'svelte/store'
import { WorkspaceService } from './gen'
import type { InferAssetsSqlQueryDetails } from './infer'
import { workspaceStore } from './stores'
import { MapResource } from './svelte5Utils.svelte'
import { sqlDataTypeToJsTypeHeuristic } from './components/apps/components/display/dbtable/utils'
import { clone } from './utils'

export function usePreparedAssetSqlQueries(
	getQueries: () => InferAssetsSqlQueryDetails[] | undefined
) {
	let workspace = get(workspaceStore) ?? ''
	function computeQueryKey(query: InferAssetsSqlQueryDetails) {
		return `${query.source_kind}::${query.source_name}::${query.source_schema}::${workspace}::${query.query_string}`
	}

	let map = new MapResource<InferAssetsSqlQueryDetails, InferAssetsSqlQueryDetails>(
		() => Object.fromEntries(getQueries()?.map((q) => [computeQueryKey(q), q]) || []),
		async (toFetch) => {
			let queries = Object.values(clone(toFetch))
			let keys = Object.keys(clone(toFetch))

			queries = queries?.filter((q) => q.source_kind === 'datatable') // We only support datatable sources for now
			if (!queries?.length) return {}
			try {
				let prepareQueriesResponse = await WorkspaceService.prepareQueries({
					workspace,
					requestBody: queries.map((q) => ({
						datatable: q.source_name,
						query: q.query_string
					}))
				})
				for (let i = 0; i < prepareQueriesResponse.results.length; ++i) {
					let res = prepareQueriesResponse.results[i]
					queries[i].prepared = res.columns
						? {
								columns: Object.fromEntries(
									res.columns.map(({ name, type }) => [name, sqlDataTypeToJsTypeHeuristic(type)])
								)
							}
						: { error: res.error ?? "Couldn't prepare query" }
				}
			} catch (e) {
				throw e
			}

			let obj: Record<string, InferAssetsSqlQueryDetails> = {}
			for (let i = 0; i < queries.length; ++i) {
				obj[keys[i]] = queries[i]
			}
			return obj
		}
	)

	return map
}
