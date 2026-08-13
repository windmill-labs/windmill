import { z } from 'zod'
import { WorkspaceService } from '$lib/gen'
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

/** The workspace DuckLake tools, for registration in global mode. */
export function getDucklakeTools(): Tool<{}>[] {
	return [
		{
			def: listDucklakesToolDef,
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
