import { z } from 'zod'
import { createToolDef } from './shared'
import { NONE, type SessionTool } from './sessionCapabilities'
import { runOnServer } from './global/backendSessionTools'

/**
 * Workspace-scoped DuckLake tools, the pipeline counterpart to `list_datatables`
 * in `datatableTools.ts`.
 *
 * A data pipeline materializes DuckLake tables and reads/writes S3 assets, which
 * only work once the workspace has object storage + a DuckLake catalog
 * configured. `list_ducklakes` lets the chat detect that prerequisite (and warn
 * with role-appropriate next steps) instead of silently producing a pipeline that
 * cannot run. `list_data_metrics` reads the declarations recorded against those
 * lake tables, not the tables themselves. Both are plain reads gated only by
 * workspace membership, so they need no app context and belong in the global tool
 * set.
 */

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

/** The workspace DuckLake tools, for registration in global mode. */
export function getDucklakeTools(): SessionTool<{}>[] {
	return [
		{
			requires: NONE,
			def: listDataMetricsToolDef,
			planModeSafe: true,
			showDetails: true,
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				listDataMetricsSchema.parse(args)
				toolCallbacks.setToolStatus(toolId, { content: 'Listing declared measures...' })
				const { text } = await runOnServer('list_data_metrics', workspace, args)
				toolCallbacks.setToolStatus(toolId, {
					content: `Listed ${JSON.parse(text).metrics.length} declared measure(s)/dimension(s)`,
					result: text
				})
				return text
			}
		},
		{
			requires: NONE,
			def: listDucklakesToolDef,
			planModeSafe: true,
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				toolCallbacks.setToolStatus(toolId, { content: 'Listing DuckLake catalogs...' })
				const { text } = await runOnServer('list_ducklakes', workspace, args)
				if (text === NO_DUCKLAKES_CONFIGURED_MESSAGE) {
					toolCallbacks.setToolStatus(toolId, {
						content: 'No DuckLake configured — set up object storage + DuckLake in workspace settings'
					})
				} else if (text.startsWith('Error listing DuckLake catalogs: ')) {
					toolCallbacks.setToolStatus(toolId, { content: text, error: text })
				} else {
					toolCallbacks.setToolStatus(toolId, {
						content: `Listed ${JSON.parse(text).ducklakes.length} DuckLake catalog(s)`
					})
				}
				return text
			}
		}
	]
}
