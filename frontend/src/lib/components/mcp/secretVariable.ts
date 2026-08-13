import { ResourceService, VariableService } from '$lib/gen'

/**
 * Store a connection's token at `path`.
 *
 * Disconnecting an MCP server deletes the resource but deliberately keeps its
 * token variable, because `delete_resource` cascade-deletes every variable the
 * value references and that credential may still belong to another resource. So
 * reconnecting the same server lands on an existing path.
 *
 * An OAuth token cannot be written over that variable: `EditVariable` carries no
 * `account` or `is_oauth`, so patching the value alone would leave the variable
 * refreshing through the previous authorization, and the old grant would
 * overwrite the token just minted. It has to be recreated — and `delete_variable`
 * takes the resource at the same path with it (`variables.rs`, symmetric with
 * `delete_resource`), which is why the destructive branch first proves the
 * variable is an OAuth one of ours and that no resource is sitting on it.
 * Anything else is refused rather than replaced: a path collision here is a
 * mistake, and taking someone's variable (and its resource) with us is not a
 * recoverable one.
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
	const existing = await VariableService.existsVariable({ workspace, path })

	if (existing && !isOauth) {
		// A static token needs none of the OAuth bookkeeping, so the value can be
		// patched in place and nothing else at this path is touched.
		await VariableService.updateVariable({ workspace, path, requestBody: { value } })
		return
	}

	if (existing) {
		const current = await VariableService.getVariable({
			workspace,
			path,
			decryptSecret: false
		})
		if (!current.is_oauth) {
			throw new Error(`A variable already exists at ${path}. Pick another path.`)
		}
		if (await ResourceService.existsResource({ workspace, path })) {
			throw new Error(`A resource already exists at ${path}. Pick another path.`)
		}
		await VariableService.deleteVariable({ workspace, path })
	}

	await VariableService.createVariable({
		workspace,
		requestBody: { path, value, is_secret: true, is_oauth: isOauth, account, description }
	})
}
