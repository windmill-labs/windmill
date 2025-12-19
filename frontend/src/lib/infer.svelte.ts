import { JobService } from './gen'
import type { InferAssetsSqlQueryDetails, PreparedAssetsSqlQuery } from './infer'
import { ChangeOnDeepInequality, MapResource } from './svelte5Utils.svelte'
import { sqlDataTypeToJsTypeHeuristic } from './components/apps/components/display/dbtable/utils'
import { chunkBy, clone } from './utils'

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
			// We only support preparing single-statement queries for now.
			queries = queries.filter(([_, q]) => getQueryStmtCountHeuristic(q.query_string) === 1)

			if (!queries?.length) return {}
			try {
				// We chunk by source_name to minimize the number of requests.
				// For example if we have 10 queries on the same data table,
				// we can prepare them all with a single script.
				queries.sort((a, b) => a[1].source_name.localeCompare(b[1].source_name))
				let results = (
					await Promise.all(
						chunkBy(queries, ([key, q]) => q.source_name).map(async (chunk) => {
							console.log(
								'Preparing chunk of queries:',
								chunk.map(([_, q]) => q)
							)
							let queryContent = chunk
								.flatMap(([key, q]) => [
									q.source_schema ? `SET search_path TO ${q.source_schema};` : 'RESET search_path;',
									q.query_string + (q.query_string.trim().endsWith(';') ? '' : ';')
								])
								.join('\n')
							queryContent =
								'-- prepare\n--result_collection=all_statements_first_row\n' + queryContent

							let res = (await JobService.runScriptPreviewAndWaitResult({
								workspace: getWorkspace()!,
								requestBody: {
									language: 'postgresql',
									content: queryContent,
									args: { database: `datatable://${chunk[0][1]?.source_name}` }
								}
							})) as { error?: string; columns?: { name: string; type: string }[] }[]

							console.log('Prepared query content:', res)

							let res2: [string, PreparedAssetsSqlQuery][] = res.map((r, i) => [
								chunk[i][0],
								r.columns
									? {
											columns: Object.fromEntries(
												r.columns.map(({ name, type }) => [
													name,
													sqlDataTypeToJsTypeHeuristic(type)
												])
											)
										}
									: { error: r.error ?? "Couldn't prepare query " }
							])
							return res2
						})
					)
				).flat()

				return Object.fromEntries(results)
			} catch (e) {
				throw e
			}
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

// AI generated
function getQueryStmtCountHeuristic(query: string): number {
	let count = 0
	let currState: 'normal' | 'single-quote' | 'double-quote' | 'line-comment' | 'block-comment' =
		'normal'

	for (let i = 0; i < query.length; i++) {
		const char = query[i]
		const nextChar = query[i + 1]

		switch (currState) {
			case 'normal':
				if (char === "'" && query[i - 1] !== '\\') {
					currState = 'single-quote'
				} else if (char === '"' && query[i - 1] !== '\\') {
					currState = 'double-quote'
				} else if (char === '-' && nextChar === '-') {
					currState = 'line-comment'
					i++ // skip next char
				} else if (char === '/' && nextChar === '*') {
					currState = 'block-comment'
					i++ // skip next char
				} else if (char === ';') {
					count++
				}
				break

			case 'single-quote':
				if (char === "'" && query[i - 1] !== '\\') {
					currState = 'normal'
				}
				break

			case 'double-quote':
				if (char === '"' && query[i - 1] !== '\\') {
					currState = 'normal'
				}
				break

			case 'line-comment':
				if (char === '\n') {
					currState = 'normal'
				}
				break

			case 'block-comment':
				if (char === '*' && nextChar === '/') {
					currState = 'normal'
					i++ // skip next char
				}
				break
		}
	}

	if (currState === 'normal' && !query.trimEnd().endsWith(';')) {
		count++
	}

	return count
}
