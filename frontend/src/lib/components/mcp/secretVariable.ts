import { VariableService } from '$lib/gen'

/**
 * Store a connection's token at `path`, reusing the variable if it is already
 * there.
 *
 * Disconnecting an MCP server deletes the resource but deliberately keeps its
 * token variable, because `delete_resource` cascade-deletes every variable the
 * value references and that credential may still belong to another resource. So
 * reconnecting the same server lands on an existing path, and a plain create
 * would fail there every time.
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
		await VariableService.updateVariable({
			workspace,
			path,
			requestBody: { value, is_secret: true, description }
		})
		return
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
