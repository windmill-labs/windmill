import { ResourceService, VariableService } from '$lib/gen'

/**
 * What this connection writes in its token variable's description, and the only
 * proof of ownership available: nothing else records that a variable belongs to
 * an MCP connection, and the path alone proves nothing.
 */
export function mcpTokenDescription(resourcePath: string): string {
	return `MCP connection token for ${resourcePath}`
}

/**
 * Store a connection's token at `path`.
 *
 * Disconnecting an MCP server deletes the resource but deliberately keeps its
 * token variable, because `delete_resource` cascade-deletes every variable the
 * value references and that credential may still belong to another resource. So
 * reconnecting the same server lands on an existing path, which is the only case
 * this may write over: a variable it did not write is refused, since overwriting
 * one silently retargets every `$var:` reference to it at the same time.
 *
 * An OAuth token cannot be patched in place: `EditVariable` carries no `account`
 * or `is_oauth`, so the variable would go on refreshing through the previous
 * authorization and the old grant would overwrite the token just minted. It has
 * to be recreated — and `delete_variable` takes the resource at the same path
 * with it (`variables.rs`, symmetric with `delete_resource`), so that branch also
 * refuses when a resource is sitting there.
 */
export async function upsertSecretVariable(args: {
	workspace: string
	path: string
	value: string
	/** The MCP resource this token belongs to; stamped into the description. */
	resourcePath: string
	isOauth?: boolean
	account?: number
}): Promise<void> {
	const { workspace, path, value, resourcePath, isOauth, account } = args
	const description = mcpTokenDescription(resourcePath)

	if (await VariableService.existsVariable({ workspace, path })) {
		const current = await VariableService.getVariable({ workspace, path, decryptSecret: false })
		if (current.description !== description) {
			throw new Error(`Variable at path ${path} already exists. Delete it or pick another path.`)
		}
		if (!isOauth) {
			// `is_secret` is inherited from the row when omitted, so it is restated
			// rather than assumed.
			await VariableService.updateVariable({
				workspace,
				path,
				requestBody: { value, is_secret: true }
			})
			return
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
