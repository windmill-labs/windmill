import { get } from 'svelte/store'
import { WorkspaceService } from './gen'
import type { InferAssetsSqlQueryDetails } from './infer'
import { workspaceStore } from './stores'
import { ChangeOnDeepInequality, MapResource } from './svelte5Utils.svelte'
import { sqlDataTypeToJsTypeHeuristic } from './components/apps/components/display/dbtable/utils'
import { clone } from './utils'

function computeQueryKey(query: InferAssetsSqlQueryDetails, workspace?: string) {
	return `${query.source_kind}::${query.source_name}::${query.source_schema}::${workspace}::${query.query_string}`
}

export function usePreparedAssetSqlQueries(
	_getQueries: () => InferAssetsSqlQueryDetails[] | undefined
): { current: InferAssetsSqlQueryDetails[] | undefined } {
	let workspace = get(workspaceStore) ?? ''

	let getQueries = new ChangeOnDeepInequality(_getQueries)

	let map = new MapResource<InferAssetsSqlQueryDetails, InferAssetsSqlQueryDetails['prepared']>(
		() =>
			Object.fromEntries(getQueries.value?.map((q) => [computeQueryKey(q, workspace), q]) || []),
		async (toFetch) => {
			let queries = Object.values(clone(toFetch))
			let keys = Object.keys(clone(toFetch))

			queries = queries?.filter((q) => q.source_kind === 'datatable') // We only support datatable sources for now
			if (!queries?.length) return {}
			try {
				console.log('Preparing queries', queries)
				let prepareQueriesResponse = await WorkspaceService.prepareQueries({
					workspace,
					requestBody: queries.map((q) => ({
						datatable: q.source_name,
						query: q.query_string,
						schema: q.source_schema
					}))
				})

				let obj: Record<string, InferAssetsSqlQueryDetails['prepared']> = {}
				for (let i = 0; i < prepareQueriesResponse.results.length; ++i) {
					let res = prepareQueriesResponse.results[i]
					obj[keys[i]] = queries[i].prepared = res.columns
						? {
								columns: Object.fromEntries(
									res.columns.map(({ name, type }) => [name, sqlDataTypeToJsTypeHeuristic(type)])
								)
							}
						: { error: res.error ?? "Couldn't prepare query" }
				}
				return obj
			} catch (e) {
				throw e
			}
		}
	)

	let extendedQueries = $derived.by(() =>
		getQueries.value?.map((q) => ({
			...q,
			prepared: map.current?.[computeQueryKey(q, workspace)]
		}))
	)
	return {
		get current() {
			return extendedQueries
		}
	}
}
