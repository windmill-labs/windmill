import { VariableService } from '$lib/gen'

/**
 * Store a connection's token at `path`, replacing whatever is already there.
 *
 * Disconnecting an MCP server deletes the resource but deliberately keeps its
 * token variable, because `delete_resource` cascade-deletes every variable the
 * value references and that credential may still belong to another resource. So
 * reconnecting the same server lands on an existing path.
 *
 * It is replaced rather than updated because `EditVariable` carries no `account`
 * or `is_oauth`: patching the value alone would leave the variable refreshing
 * through the previous authorization, so the old grant would overwrite the token
 * just minted. Deleting first is safe in the other direction — a variable has no
 * cascade of its own, and references to it are by path, which does not change.
 */
export async function upsertSecretVariable(args: {
	workspace: string
	path: string
	value: string
	description: string
	isOauth?: boolean
	account?: number
}): Promise<void> {
	const { workspace, path, value, description, isOauth, account } = args
	if (await VariableService.existsVariable({ workspace, path })) {
		await VariableService.deleteVariable({ workspace, path })
	}
	await VariableService.createVariable({
		workspace,
		requestBody: {
			path,
			value,
			is_secret: true,
			is_oauth: isOauth,
			account,
			description
		}
	})
}
