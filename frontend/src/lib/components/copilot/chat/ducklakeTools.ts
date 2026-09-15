import { z } from 'zod'
import { DataMetricService, WorkspaceService } from '$lib/gen'
import { createToolDef, type Tool } from './shared'

/**
 * Workspace-scoped DuckLake readiness tool, the pipeline counterpart to
 * `list_datatables` in `datatableTools.ts`.
 *
 * A data pipeline materializes DuckLake tables and reads/writes S3 assets, which
 * only work once the workspace has object storage + a DuckLake catalog
 * configured. This tool lets the chat detect that prerequisite (and warn with
 * role-appropriate next steps) instead of silently producing a pipeline that
 * cannot run. It is a plain read gated only by workspace membership, so it needs
 * no app context and belongs in the global tool set.
 *
 * `list_data_metrics` reads the same DuckLake tables, so it lives here too.
 */

/** List the names of the DuckLake catalogs configured in the workspace. */
export async function listDucklakes(workspace: string): Promise<string[]> {
	return await WorkspaceService.listDucklakes({ workspace })
}

const NO_DUCKLAKES_CONFIGURED_MESSAGE =
	'No DuckLake catalogs are configured in this workspace. A data pipeline that materializes DuckLake tables or reads/writes S3 assets cannot run until object storage and a DuckLake catalog are set up. ' +
	'You can still draft the pipeline scripts, but tell the user how to enable it by role: a workspace admin adds object storage under Workspace settings → Object Storage (S3/Azure/GCS), then a DuckLake catalog on top of it; a user without admin rights should ask a workspace admin. ' +
	"Do not assume a default 'main' DuckLake exists."

const listDucklakesSchema = z.object({})
const listDucklakesToolDef = createToolDef(
	listDucklakesSchema,
	'list_ducklakes',
	'List the DuckLake catalogs configured in this workspace, by name. Call this before building or deploying a data pipeline that materializes DuckLake tables or reads/writes S3 assets: if it returns none, the workspace has no object storage + DuckLake configured and the pipeline cannot run until a workspace admin sets it up. Returns names only.'
)

const listDataMetricsSchema = z.object({
	table: z
		.string()
		.optional()
		.describe(
			'Only declarations on this DuckLake table, as `<lake>/<table>` or `<lake>/<schema>.<table>`, with or without the `ducklake://` scheme. A name with no lake matches nothing and comes back empty.'
		),
	path_prefix: z
		.string()
		.optional()
		.describe('Only declarations made by scripts under this path, e.g. `f/analytics`.'),
	limit: z
		.number()
		.int()
		.min(1)
		.max(1000)
		.optional()
		.describe('Max number of declarations to return. Defaults to 200.')
})
const listDataMetricsToolDef = createToolDef(
	listDataMetricsSchema,
	'list_data_metrics',
	'List the measures and dimensions declared on DuckLake tables (from `// measure` / `// dimension` annotations in deployed scripts). Call this before writing any aggregate query over a DuckLake table: a declared measure is the canonical definition of that number, and reproducing it yourself silently disagrees with it (a `revenue` measure typically excludes refunds or test rows). Use each returned `expr` verbatim, and when a measure has a `filter` write it as `expr FILTER (WHERE filter)` so measures with different predicates share one GROUP BY. Only declarations whose producing script you can read are returned, so what comes back is never proof of what exists: if the number you need is not here you may still write your own aggregate, but say that you found no declared measure for it rather than implying none exists.'
)

// The endpoint drops declarations whose producing script the caller cannot read
// (token scope + RLS on `script`), so an empty result means "none declared" or
// "none readable by you" and the tool cannot tell which.
const NO_DATA_METRICS_NOTE =
	'Nothing matched. That does not establish the table has no declared measures: declarations whose producing script you cannot read are omitted from this list, not flagged. You may write your own aggregate, but tell the user you found no declared measure you can read rather than stating none is declared.'

// Well under the endpoint's 1000 cap: a full page is pretty-printed into the
// chat context, and 1000 declarations would cost tens of thousands of tokens.
const DEFAULT_DATA_METRICS_LIMIT = 200

/** The workspace DuckLake tools, for registration in global mode. */
export function getDucklakeTools(): Tool<{}>[] {
	return [
		{
			def: listDataMetricsToolDef,
			planModeSafe: true,
			showDetails: true,
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				const parsed = listDataMetricsSchema.parse(args)
				toolCallbacks.setToolStatus(toolId, { content: 'Listing declared measures...' })
				const limit = parsed.limit ?? DEFAULT_DATA_METRICS_LIMIT
				const { metrics, next_cursor } = await DataMetricService.listDataMetrics({
					workspace,
					table: parsed.table,
					pathPrefix: parsed.path_prefix,
					perPage: limit
				})
				const note = next_cursor
					? `More declarations exist beyond the first ${limit}. Re-call with table/path_prefix to target what you are looking for, or with a higher limit (max 1000) for the rest of the list, rather than concluding a measure is undeclared.`
					: metrics.length === 0
						? NO_DATA_METRICS_NOTE
						: undefined
				const result = JSON.stringify({ metrics, ...(note ? { note } : {}) }, null, 2)
				toolCallbacks.setToolStatus(toolId, {
					content: `Listed ${metrics.length} declared measure(s)/dimension(s)`,
					result
				})
				return result
			}
		},
		{
			def: listDucklakesToolDef,
			planModeSafe: true,
			fn: async ({ workspace, toolId, toolCallbacks }) => {
				toolCallbacks.setToolStatus(toolId, { content: 'Listing DuckLake catalogs...' })
				try {
					const ducklakes = await listDucklakes(workspace)
					if (ducklakes.length === 0) {
						toolCallbacks.setToolStatus(toolId, {
							content:
								'No DuckLake configured — set up object storage + DuckLake in workspace settings'
						})
						return NO_DUCKLAKES_CONFIGURED_MESSAGE
					}
					toolCallbacks.setToolStatus(toolId, {
						content: `Listed ${ducklakes.length} DuckLake catalog(s)`
					})
					return JSON.stringify({ ducklakes }, null, 2)
				} catch (e) {
					const errorMsg = `Error listing DuckLake catalogs: ${e instanceof Error ? e.message : String(e)}`
					toolCallbacks.setToolStatus(toolId, { content: errorMsg, error: errorMsg })
					return errorMsg
				}
			}
		}
	]
}
